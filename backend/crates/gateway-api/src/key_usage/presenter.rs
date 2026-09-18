//! 只投影当前 Key 可见的用量字段，不序列化账号、凭据、上游标识或诊断正文。

use chrono::{DateTime, Utc};
use gateway_admin::model::{
    accounts::{AccountModelUsage, AccountUsage},
    key_usage::{
        KeyUsageAccountList, KeyUsageAccountQuota, KeyUsageAccountScopeState,
        KeyUsageAccountSnapshot, KeyUsageOverview, KeyUsageQuotaAvailability, KeyUsageRecords,
    },
    observability::{
        CostCoverage, CurrencyCost, Granularity, OpsError, RequestMetrics, UsageListRecord,
    },
};
use serde::Serialize;
use serde_json::{Value, json};

use crate::admin::observability::{
    BillingView, HealthTimelineView, PageData, TokenDetailsView, billing_view,
    health_timeline_view, usage_list_token_details,
};
use crate::admin::presenter::{format_compact_number, format_number};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OverviewView {
    as_of: DateTime<Utc>,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    key: KeyView,
    summary: MetricsView,
    trend: Vec<TrendPointView>,
    health_timeline: HealthTimelineView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyView {
    name: String,
    prefix: String,
    max_concurrency: u64,
    requests_per_minute: u64,
    daily_limit_usd: String,
    daily_used_usd: String,
    daily_resets_at: Option<DateTime<Utc>>,
    weekly_limit_usd: String,
    weekly_used_usd: String,
    weekly_resets_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MetricsView {
    requests: u64,
    input_tokens: u64,
    output_tokens: u64,
    cached_tokens: u64,
    cache_write_tokens: u64,
    reasoning_tokens: u64,
    total_tokens: u64,
    cost_usd: Option<String>,
    cost_incomplete: bool,
}

fn metrics(value: &RequestMetrics, costs: &[CurrencyCost], coverage: &CostCoverage) -> MetricsView {
    MetricsView {
        requests: value.request_count,
        input_tokens: value.input_tokens,
        output_tokens: value.output_tokens,
        cached_tokens: value.cached_tokens,
        cache_write_tokens: value.cache_write_tokens,
        reasoning_tokens: value.reasoning_tokens,
        total_tokens: value.total_tokens,
        cost_usd: costs
            .iter()
            .find(|cost| cost.currency.eq_ignore_ascii_case("USD"))
            .map(|cost| cost.amount.as_str().to_owned())
            .or_else(|| (value.request_count == 0).then(|| "0".to_owned())),
        cost_incomplete: coverage.partial_count > 0 || coverage.unavailable_count > 0,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TrendPointView {
    time: DateTime<Utc>,
    bucket_seconds: u32,
    #[serde(flatten)]
    metrics: MetricsView,
}

pub(super) fn overview(value: KeyUsageOverview) -> OverviewView {
    let key = value.key;
    OverviewView {
        as_of: Utc::now(),
        start_time: value.overview.range.start,
        end_time: value.overview.range.end,
        key: KeyView {
            name: key.name,
            prefix: key.prefix,
            max_concurrency: key.limits.max_concurrency,
            requests_per_minute: key.limits.requests_per_minute,
            daily_limit_usd: key.budget.limits.daily_usd.canonical(),
            daily_used_usd: key.budget.daily_used_usd.canonical(),
            daily_resets_at: key.budget.daily_resets_at.map(DateTime::from),
            weekly_limit_usd: key.budget.limits.weekly_usd.canonical(),
            weekly_used_usd: key.budget.weekly_used_usd.canonical(),
            weekly_resets_at: key.budget.weekly_resets_at.map(DateTime::from),
        },
        summary: metrics(
            &value.overview.requests,
            &value.overview.attempts.costs,
            &value.overview.attempts.cost_coverage,
        ),
        trend: value
            .trend
            .into_iter()
            .map(|point| TrendPointView {
                time: point.bucket_start,
                bucket_seconds: match point.granularity {
                    Granularity::FifteenMinutes => 900,
                    Granularity::Hour => 3600,
                    Granularity::Day => 86400,
                },
                metrics: metrics(&point.metrics, &point.costs, &point.cost_coverage),
            })
            .collect(),
        health_timeline: health_timeline_view(value.health_timeline),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RecordView {
    id: String,
    created_at: DateTime<Utc>,
    model: Option<String>,
    route: Option<String>,
    reasoning_effort: Option<String>,
    client_transport: Option<String>,
    upstream_transport: Option<String>,
    token_details: Option<TokenDetailsView>,
    billing: Option<BillingView>,
    latency_ms: Option<u64>,
    first_token_latency_ms: Option<u64>,
    latency_details: OutputTimingView,
    client_ip: Option<String>,
    user_agent: Option<String>,
    status: &'static str,
    status_code: Option<u16>,
}

// Key 只查看自身请求的输出时间，不包含账号容量、调度等待等管理侧观测。
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct OutputTimingView {
    #[serde(skip_serializing_if = "Option::is_none")]
    first_event_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_reasoning_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_text_ms: Option<u64>,
}

fn success_record(value: UsageListRecord) -> RecordView {
    let token_details = Some(usage_list_token_details(&value));
    let billing = billing_view(value.billing.as_ref());
    RecordView {
        id: value.id,
        created_at: value.started_at,
        model: value.requested_model_id,
        route: Some(value.endpoint),
        reasoning_effort: value.reasoning_effort,
        client_transport: Some(value.client_transport),
        upstream_transport: value.upstream_transport,
        token_details,
        billing,
        latency_ms: value.latency_ms,
        first_token_latency_ms: value.first_token_ms,
        latency_details: OutputTimingView {
            first_event_ms: value.first_event_ms,
            first_reasoning_ms: value.first_reasoning_ms,
            first_text_ms: value.first_text_ms,
        },
        client_ip: value.client_ip,
        user_agent: value.user_agent,
        status: "success",
        // 成功记录不保存 HTTP 状态，不能用 200 伪造缺失的原始事实。
        status_code: None,
    }
}

fn error_record(value: OpsError) -> RecordView {
    RecordView {
        id: value.event_id,
        created_at: value.occurred_at,
        model: value.requested_model_id,
        route: value.endpoint,
        reasoning_effort: value.reasoning_effort,
        client_transport: value.client_transport,
        upstream_transport: value.upstream_transport,
        token_details: None,
        billing: None,
        latency_ms: value.latency_ms,
        first_token_latency_ms: None,
        latency_details: OutputTimingView::default(),
        client_ip: value.client_ip,
        user_agent: value.user_agent,
        status: "error",
        status_code: value.client_status_code,
    }
}

pub(super) fn records(value: KeyUsageRecords) -> PageData<RecordView> {
    match value {
        KeyUsageRecords::Success(page) => PageData {
            items: page.items.into_iter().map(success_record).collect(),
            current_page: page.current_page,
            page_size: page.page_size,
            total: page.total,
        },
        KeyUsageRecords::Error(page) => PageData {
            items: page.items.into_iter().map(error_record).collect(),
            current_page: page.current_page,
            page_size: page.page_size,
            total: page.total,
        },
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AccountListView {
    scope_state: &'static str,
    items: Vec<AccountListItemView>,
    current_page: u32,
    page_size: u16,
    total: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountListItemView {
    id: String,
    identity: Option<String>,
    provider: String,
    authentication_kind: String,
    plan_type: Option<String>,
    plan_type_display: String,
    status: String,
    quota: AccountQuotaView,
    usage: AccountUsageSummaryView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AccountDetailView {
    id: String,
    identity: Option<String>,
    provider: String,
    authentication_kind: String,
    plan_type: Option<String>,
    plan_type_display: String,
    status: String,
    quota: AccountQuotaView,
    usage: AccountUsageView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountQuotaView {
    availability: &'static str,
    refreshed_at_display: String,
    limit_reached: bool,
    windows: Vec<AccountQuotaWindowView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountQuotaWindowView {
    key: String,
    group: String,
    limit_id: Option<String>,
    limit_name: Option<String>,
    role: Option<String>,
    window_seconds: Option<u64>,
    label_display: String,
    window_label_display: String,
    used_percent: Option<f64>,
    used_percent_display: String,
    limit_reached: bool,
    local_usage: Option<Value>,
    reset_at_display: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountUsageSummaryView {
    window_label_display: String,
    request_count: Option<u64>,
    request_count_display: String,
    success_count: Option<u64>,
    success_rate: Option<f64>,
    success_rate_display: String,
    total_tokens: Option<u64>,
    total_tokens_display: String,
    last_used_at: Option<DateTime<Utc>>,
    last_used_at_display: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountUsageView {
    #[serde(flatten)]
    summary: AccountUsageSummaryView,
    input_tokens: Option<u64>,
    input_tokens_display: String,
    output_tokens: Option<u64>,
    output_tokens_display: String,
    cached_tokens: Option<u64>,
    cached_tokens_display: String,
    reasoning_tokens: Option<u64>,
    reasoning_tokens_display: String,
    created_tokens: Option<u64>,
    created_tokens_display: String,
    read_tokens: Option<u64>,
    read_tokens_display: String,
    models: Vec<AccountModelUsageView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountModelUsageView {
    model: String,
    request_count: u64,
    request_count_display: String,
    success_rate: Option<f64>,
    success_rate_display: String,
    input_tokens: Option<u64>,
    input_tokens_display: String,
    output_tokens: Option<u64>,
    output_tokens_display: String,
    cached_tokens: Option<u64>,
    cached_tokens_display: String,
    reasoning_tokens: Option<u64>,
    reasoning_tokens_display: String,
    total_tokens: Option<u64>,
    total_tokens_display: String,
    last_used_at: DateTime<Utc>,
    last_used_at_display: String,
}

pub(super) fn accounts(value: KeyUsageAccountList) -> AccountListView {
    AccountListView {
        scope_state: scope_state(value.scope_state),
        items: value.items.into_iter().map(account_list_item).collect(),
        current_page: value.current_page,
        page_size: value.page_size,
        total: value.total,
    }
}

pub(super) fn account_detail(value: KeyUsageAccountSnapshot) -> AccountDetailView {
    let usage = account_usage_view(value.usage.as_ref(), &value);
    let account = account_list_item(value);
    AccountDetailView {
        id: account.id,
        identity: account.identity,
        provider: account.provider,
        authentication_kind: account.authentication_kind,
        plan_type: account.plan_type,
        plan_type_display: account.plan_type_display,
        status: account.status,
        quota: account.quota,
        usage,
    }
}

fn account_list_item(value: KeyUsageAccountSnapshot) -> AccountListItemView {
    let account = &value.item.account;
    let label = usage_label(&value);
    let usage = usage_summary(value.usage.as_ref(), label);
    let identity = account
        .email
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            let name = account.name.trim();
            (!name.is_empty()).then_some(name)
        });
    AccountListItemView {
        id: account.id.clone(),
        identity: mask_identity(identity),
        provider: account.provider_kind.to_string(),
        authentication_kind: account.authentication_kind.clone(),
        plan_type: account.plan_type.clone(),
        plan_type_display: value
            .plan_type_display
            .unwrap_or_else(|| "未知套餐".to_owned()),
        status: value.item.projection.status.as_str().to_owned(),
        quota: quota_view(value.quota),
        usage,
    }
}

fn scope_state(value: KeyUsageAccountScopeState) -> &'static str {
    match value {
        KeyUsageAccountScopeState::Unbound => "unbound",
        KeyUsageAccountScopeState::NoEnabledGroups => "no_enabled_groups",
        KeyUsageAccountScopeState::Available => "available",
    }
}

fn quota_availability(value: KeyUsageQuotaAvailability) -> &'static str {
    match value {
        KeyUsageQuotaAvailability::Available => "available",
        KeyUsageQuotaAvailability::Unsupported => "unsupported",
        KeyUsageQuotaAvailability::Unobserved => "unobserved",
        KeyUsageQuotaAvailability::Unavailable => "unavailable",
    }
}

fn quota_view(value: KeyUsageAccountQuota) -> AccountQuotaView {
    let availability = quota_availability(value.availability);
    let Some(quota) = value.value else {
        return AccountQuotaView {
            availability,
            refreshed_at_display: "—".to_owned(),
            limit_reached: false,
            windows: Vec::new(),
        };
    };
    AccountQuotaView {
        availability,
        refreshed_at_display: crate::admin::observability::relative_time(
            quota.observed_at,
            Utc::now(),
        ),
        limit_reached: quota.limit_reached,
        windows: quota.windows.into_iter().map(quota_window_view).collect(),
    }
}

fn quota_window_view(
    window: gateway_admin::model::provider_credentials::ProviderQuotaWindow,
) -> AccountQuotaWindowView {
    let label_display = window.limit_name.as_ref().map_or_else(
        || window.label.clone(),
        |name| format!("{name} · {}", window.label),
    );
    AccountQuotaWindowView {
        key: window.key,
        group: window.group,
        limit_id: window.limit_id,
        limit_name: window.limit_name,
        role: window.role.map(|role| role.as_str().to_owned()),
        window_seconds: window.window_seconds,
        label_display,
        window_label_display: window.label,
        used_percent: window.used_percent,
        used_percent_display: display_percent(window.used_percent),
        limit_reached: window.limit_reached,
        local_usage: window.local_usage.as_ref().map(local_usage_view),
        reset_at_display: window.reset_at.map_or_else(
            || "—".to_owned(),
            |time| crate::admin::observability::china_datetime(&time),
        ),
    }
}

fn local_usage_view(usage: &AccountUsage) -> Value {
    json!({
        "requestCount": usage.request_count,
        "requestCountDisplay": format_number(usage.request_count),
        "inputTokens": usage.input_tokens,
        "inputTokensDisplay": display_tokens(usage.input_tokens),
        "outputTokens": usage.output_tokens,
        "outputTokensDisplay": display_tokens(usage.output_tokens),
        "cachedTokens": usage.cached_tokens,
        "cachedTokensDisplay": display_tokens(usage.cached_tokens),
        "totalTokens": usage.total_tokens,
        "totalTokensDisplay": display_tokens(usage.total_tokens),
    })
}

fn usage_label(value: &KeyUsageAccountSnapshot) -> &'static str {
    if value.item.account.authentication_kind == "api_key" {
        return "通用额度";
    }
    match value
        .quota
        .value
        .as_ref()
        .and_then(|quota| quota.usage_window())
        .map(|(_, period)| period)
    {
        Some(gateway_admin::model::provider_credentials::AccountUsagePeriod::Weekly) => {
            "周额度窗口"
        }
        Some(gateway_admin::model::provider_credentials::AccountUsagePeriod::Monthly) => {
            "月额度窗口"
        }
        None => "周月额度窗口",
    }
}

fn usage_summary(usage: Option<&AccountUsage>, label: &str) -> AccountUsageSummaryView {
    let Some(usage) = usage else {
        return AccountUsageSummaryView {
            window_label_display: label.to_owned(),
            request_count: None,
            request_count_display: "—".to_owned(),
            success_count: None,
            success_rate: None,
            success_rate_display: "—".to_owned(),
            total_tokens: None,
            total_tokens_display: "—".to_owned(),
            last_used_at: None,
            last_used_at_display: "—".to_owned(),
        };
    };
    AccountUsageSummaryView {
        window_label_display: label.to_owned(),
        request_count: Some(usage.request_count),
        request_count_display: format_number(usage.request_count),
        success_count: Some(usage.success_count),
        success_rate: success_rate(usage.success_count, usage.request_count),
        success_rate_display: display_percent(success_rate(
            usage.success_count,
            usage.request_count,
        )),
        total_tokens: usage.total_tokens,
        total_tokens_display: display_tokens(usage.total_tokens),
        last_used_at: usage.last_used_at,
        last_used_at_display: crate::admin::observability::relative_time(
            usage.last_used_at,
            Utc::now(),
        ),
    }
}

fn account_usage_view(
    usage: Option<&AccountUsage>,
    snapshot: &KeyUsageAccountSnapshot,
) -> AccountUsageView {
    let summary = usage_summary(usage, usage_label(snapshot));
    let Some(usage) = usage else {
        return AccountUsageView {
            summary,
            input_tokens: None,
            input_tokens_display: "—".to_owned(),
            output_tokens: None,
            output_tokens_display: "—".to_owned(),
            cached_tokens: None,
            cached_tokens_display: "—".to_owned(),
            reasoning_tokens: None,
            reasoning_tokens_display: "—".to_owned(),
            created_tokens: None,
            created_tokens_display: "—".to_owned(),
            read_tokens: None,
            read_tokens_display: "—".to_owned(),
            models: Vec::new(),
        };
    };
    AccountUsageView {
        summary,
        input_tokens: usage.input_tokens,
        input_tokens_display: display_tokens(usage.input_tokens),
        output_tokens: usage.output_tokens,
        output_tokens_display: display_tokens(usage.output_tokens),
        cached_tokens: usage.cached_tokens,
        cached_tokens_display: display_tokens(usage.cached_tokens),
        reasoning_tokens: usage.reasoning_tokens,
        reasoning_tokens_display: display_tokens(usage.reasoning_tokens),
        created_tokens: usage.cache_write_tokens,
        created_tokens_display: display_tokens(usage.cache_write_tokens),
        read_tokens: usage.cached_tokens,
        read_tokens_display: display_tokens(usage.cached_tokens),
        models: usage.models.iter().map(model_usage_view).collect(),
    }
}

fn model_usage_view(usage: &AccountModelUsage) -> AccountModelUsageView {
    let rate = success_rate(usage.success_count, usage.request_count);
    AccountModelUsageView {
        model: usage.model.clone(),
        request_count: usage.request_count,
        request_count_display: format_number(usage.request_count),
        success_rate: rate,
        success_rate_display: display_percent(rate),
        input_tokens: usage.input_tokens,
        input_tokens_display: display_tokens(usage.input_tokens),
        output_tokens: usage.output_tokens,
        output_tokens_display: display_tokens(usage.output_tokens),
        cached_tokens: usage.cached_tokens,
        cached_tokens_display: display_tokens(usage.cached_tokens),
        reasoning_tokens: usage.reasoning_tokens,
        reasoning_tokens_display: display_tokens(usage.reasoning_tokens),
        total_tokens: usage.total_tokens,
        total_tokens_display: display_tokens(usage.total_tokens),
        last_used_at: usage.last_used_at,
        last_used_at_display: crate::admin::observability::relative_time(
            Some(usage.last_used_at),
            Utc::now(),
        ),
    }
}

fn mask_identity(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= 5 {
        return Some(format!("{}***", chars[0]));
    }
    Some(format!(
        "{}{}***{}",
        chars[0],
        chars[1],
        chars[chars.len() - 3..].iter().collect::<String>()
    ))
}

fn success_rate(success: u64, total: u64) -> Option<f64> {
    (total > 0).then(|| success as f64 * 100.0 / total as f64)
}

fn display_percent(value: Option<f64>) -> String {
    value.map_or_else(|| "—".to_owned(), |value| format!("{value:.1}%"))
}

fn display_tokens(value: Option<u64>) -> String {
    value.map_or_else(|| "—".to_owned(), format_compact_number)
}
