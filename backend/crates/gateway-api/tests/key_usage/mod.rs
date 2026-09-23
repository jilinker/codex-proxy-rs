//! 通过真实路由验证会话隔离、范围收敛和对外字段白名单。

use std::sync::atomic::Ordering;

use axum::{
    Router,
    http::{Method, StatusCode, header},
    response::Response,
};
use chrono::{Duration, Utc};
use gateway_admin::model::observability::{RequestMetrics, china_day_start};
use serde_json::{Value, json};
use tower::ServiceExt as _;

use crate::support::{RAW_KEY, cookie_request, empty_request, json_request, response_json};

mod fixtures;

const RANGE: &str = "startTime=2026-09-01T00:00:00Z&endTime=2026-09-02T00:00:00Z";

async fn login(app: &Router, mode: &str) -> String {
    let body = if mode == "key" {
        json!({"mode": "key", "apiKey": RAW_KEY})
    } else {
        json!({"mode": "admin", "username": "admin_1", "password": "strong-admin-password"})
    };
    let response = app
        .clone()
        .oneshot(json_request(Method::POST, "/api/auth/login", body))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

async fn get(app: &Router, resource: &str, suffix: &str, cookie: &str) -> Response {
    let path = if matches!(resource, "config" | "version") {
        format!("/api/key-usage/{resource}{suffix}")
    } else {
        format!("/api/key-usage/{resource}?{RANGE}{suffix}")
    };
    let response = app
        .clone()
        .oneshot(cookie_request(Method::GET, &path, cookie))
        .await
        .unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    response
}

async fn get_accounts(app: &Router, suffix: &str, cookie: &str) -> Response {
    let path = format!("/api/key-usage/accounts{suffix}");
    let response = app
        .clone()
        .oneshot(cookie_request(Method::GET, &path, cookie))
        .await
        .unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    response
}

#[tokio::test]
async fn version_exposes_only_build_identifiers_for_key_sessions() {
    let fixture = fixtures::fixture().await;
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let cookie = login(&app, "key").await;
    let response = get(&app, "version", "", &cookie).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await["data"],
        json!({"version": "3.7.0", "gitSha": "internal-revision"})
    );
    assert_eq!(
        get(&app, "version", "?keyId=other", &cookie).await.status(),
        StatusCode::BAD_REQUEST
    );
    let response = app
        .clone()
        .oneshot(cookie_request(
            Method::GET,
            "/api/admin/system/version",
            &cookie,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn config_reveals_only_the_session_keys_name_and_plaintext() {
    let fixture = fixtures::fixture().await;
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let cookie = login(&app, "key").await;
    let response = get(&app, "config", "", &cookie).await;
    assert_eq!(response.status(), StatusCode::OK);
    let data = response_json(response).await["data"].clone();
    assert_eq!(
        data,
        json!({
            "name": "Development",
            "plaintextKey": format!("sk_{}", "a".repeat(43)),
        })
    );
    let overview = get(&app, "overview", "", &cookie).await;
    assert!(
        !response_json(overview)
            .await
            .to_string()
            .contains("plaintextKey")
    );
}

#[tokio::test]
async fn config_rejects_caller_selected_scope() {
    let fixture = fixtures::fixture().await;
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let cookie = login(&app, "key").await;
    for query in [
        "?keyId=other",
        "?id=other",
        "?clientApiKeyRef=other",
        "?provider=openai",
    ] {
        let response = get(&app, "config", query, &cookie).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(response_json(response).await["data"].is_null());
    }
}

#[tokio::test]
async fn config_does_not_reveal_disabled_or_other_keys() {
    let fixture = fixtures::fixture().await;
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let cookie = login(&app, "key").await;
    {
        let mut key = fixture.client_key.lock().unwrap();
        key.as_mut().unwrap().enabled = false;
    }
    assert_eq!(
        get(&app, "config", "", &cookie).await.status(),
        StatusCode::UNAUTHORIZED
    );
    {
        let mut key = fixture.client_key.lock().unwrap();
        let key = key.as_mut().unwrap();
        key.enabled = true;
        key.id = gateway_core::policy::ClientApiKeyId::new("other-key").unwrap();
    }
    assert_eq!(
        get(&app, "config", "", &cookie).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

fn assert_fields(value: &Value, expected: &[&str]) {
    let mut actual: Vec<_> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    actual.sort_unstable();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

#[tokio::test]
async fn overview_scopes_every_query_and_projects_only_key_visible_fields() {
    let fixture = fixtures::fixture().await;
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let cookie = login(&app, "key").await;
    let response = get(&app, "overview", "&model=%20coding%20", &cookie).await;
    assert_eq!(response.status(), StatusCode::OK);
    let data = response_json(response).await["data"].clone();
    assert_fields(
        &data,
        &[
            "asOf",
            "startTime",
            "endTime",
            "key",
            "summary",
            "trend",
            "healthTimeline",
        ],
    );
    assert_fields(
        &data["key"],
        &[
            "name",
            "prefix",
            "maxConcurrency",
            "requestsPerMinute",
            "dailyLimitUsd",
            "dailyUsedUsd",
            "dailyResetsAt",
            "weeklyLimitUsd",
            "weeklyUsedUsd",
            "weeklyResetsAt",
        ],
    );
    assert_eq!(data["key"]["dailyUsedUsd"], "0.640001");
    assert_eq!(data["key"]["maxConcurrency"], 0);
    assert_eq!(data["summary"]["totalTokens"], 1100);
    assert_eq!(data["summary"]["reasoningTokens"], 40);
    assert_eq!(data["trend"][0]["reasoningTokens"], 40);
    assert_eq!(data["summary"]["costUsd"], "0.123456");
    assert_eq!(data["summary"]["costIncomplete"], true);
    assert_eq!(data["trend"][0]["bucketSeconds"], 900);
    assert_eq!(
        data["healthTimeline"]["points"].as_array().unwrap().len(),
        96
    );
    assert!(!data.to_string().contains("private-sentinel"));
    let observations = fixture.observations.lock().unwrap();
    assert_eq!(observations.summaries.len(), 1);
    assert_eq!(observations.trends.len(), 2);
    for (_, filter) in observations
        .summaries
        .iter()
        .chain(observations.trends.iter())
    {
        assert_eq!(filter.client_api_key_ref.as_deref(), Some("key-42"));
        assert!(filter.provider_account_ref.is_none());
        assert!(filter.provider_kind.is_none());
    }
    assert_eq!(observations.summaries[0].1.model.as_deref(), Some("coding"));
    let health = observations
        .trends
        .iter()
        .find(|(_, filter)| filter.model.is_none())
        .unwrap();
    assert_eq!(health.0.start, china_day_start(health.0.end));
    assert!((Utc::now() - health.0.end) < Duration::seconds(5));
}

#[tokio::test]
async fn records_keep_pagination_and_hide_admin_and_upstream_data() {
    let fixture = fixtures::fixture().await;
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let cookie = login(&app, "key").await;
    for kind in ["success", "error"] {
        let response = get(
            &app,
            "records",
            &format!("&kind={kind}&model=coding&currentPage=3&pageSize=10"),
            &cookie,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let data = response_json(response).await["data"].clone();
        assert_eq!(data["currentPage"], 3);
        assert_eq!(data["pageSize"], 10);
        assert_eq!(data["total"], 1);
        let record = &data["items"][0];
        assert_fields(
            record,
            &[
                "id",
                "createdAt",
                "model",
                "route",
                "reasoningEffort",
                "clientTransport",
                "upstreamTransport",
                "tokenDetails",
                "billing",
                "latencyMs",
                "firstTokenLatencyMs",
                "latencyDetails",
                "clientIp",
                "userAgent",
                "status",
                "statusCode",
            ],
        );
        assert_eq!(record["status"], kind);
        assert_eq!(record["model"], "coding");
        assert_eq!(record["reasoningEffort"], "xhigh");
        assert_eq!(record["clientTransport"], "http_sse");
        assert_eq!(record["upstreamTransport"], "websocket");
        assert_eq!(record["clientIp"], "192.0.2.42");
        assert_eq!(record["userAgent"], "key-usage-test/1.0");
        assert!(!data.to_string().contains("private-sentinel"));
        if kind == "success" {
            assert_eq!(record["billing"]["totalAmountDisplay"], "$0.1235");
            assert_eq!(record["tokenDetails"]["reasoningTokens"], 40);
            assert_eq!(record["tokenDetails"]["totalTokens"], 1100);
            assert_eq!(record["firstTokenLatencyMs"], 210);
            assert_fields(
                &record["latencyDetails"],
                &["firstEventMs", "firstReasoningMs", "firstTextMs"],
            );
            assert_eq!(record["latencyDetails"]["firstEventMs"], 100);
            assert!(record["statusCode"].is_null());
        } else {
            assert_eq!(record["statusCode"], 502);
            assert!(record["billing"].is_null());
            assert!(record["tokenDetails"].is_null());
            assert_eq!(record["latencyDetails"], json!({}));
        }
    }
    let data = fixture.observations.lock().unwrap();
    assert_eq!(
        data.records[0].filter.client_api_key_ref.as_deref(),
        Some("key-42")
    );
    assert_eq!(
        data.errors[0].filter.client_api_key_ref.as_deref(),
        Some("key-42")
    );
    assert_eq!(data.records[0].filter.model.as_deref(), Some("coding"));
    assert_eq!(data.errors[0].filter.model.as_deref(), Some("coding"));
}

#[tokio::test]
async fn accounts_report_unbound_scope_without_reading_account_data() {
    let fixture = fixtures::fixture().await;
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let cookie = login(&app, "key").await;
    let response = get_accounts(&app, "?currentPage=1&pageSize=20", &cookie).await;
    assert_eq!(response.status(), StatusCode::OK);
    let data = response_json(response).await["data"].clone();
    assert_eq!(data["scopeState"], "unbound");
    assert_eq!(data["items"], json!([]));
    assert_eq!(data["currentPage"], 1);
    assert_eq!(data["pageSize"], 20);
    assert_eq!(data["total"], 0);
    assert!(!data.to_string().to_lowercase().contains("cost"));
    assert!(!data.to_string().to_lowercase().contains("fee"));
}

#[tokio::test]
async fn accounts_project_only_read_only_usage_fields_and_mask_identity() {
    let fixture = fixtures::fixture().await;
    fixtures::bind_primary_group(&fixture);
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let cookie = login(&app, "key").await;
    let response = get_accounts(&app, "?currentPage=1&pageSize=20", &cookie).await;
    assert_eq!(response.status(), StatusCode::OK);
    let data = response_json(response).await["data"].clone();
    assert_eq!(data["scopeState"], "available");
    assert_eq!(data["total"], 2);
    let account = &data["items"][0];
    assert_eq!(account["id"], "acct_group_ready");
    assert_eq!(account["identity"], "vi***com");
    assert_eq!(account["quota"]["availability"], "unsupported");
    assert_eq!(account["usage"]["totalTokensDisplay"], "—");
    let body = data.to_string();
    assert!(!body.contains("private-sentinel"));
    assert!(!body.to_lowercase().contains("cost"));
    assert!(!body.to_lowercase().contains("fee"));
    assert!(!body.contains("credential"));

    let response = get_accounts(&app, "/detail?accountId=acct_group_ready", &cookie).await;
    assert_eq!(response.status(), StatusCode::OK);
    let detail = response_json(response).await["data"].clone();
    assert_eq!(detail["id"], "acct_group_ready");
    assert_eq!(detail["usage"]["models"], json!([]));
    assert!(!detail.to_string().to_lowercase().contains("cost"));
}

#[tokio::test]
async fn missing_admin_and_revoked_sessions_cannot_read_key_usage() {
    let fixture = fixtures::fixture().await;
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let admin = login(&app, "admin").await;
    let key = login(&app, "key").await;
    for resource in ["overview", "records", "config", "version"] {
        assert_eq!(
            get(&app, resource, "", "").await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            get(&app, resource, "", &admin).await.status(),
            StatusCode::FORBIDDEN
        );
        fixture.auth.enabled.store(false, Ordering::SeqCst);
        let response = get(&app, resource, "", &key).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response_json(response).await["code"], 40101);
        fixture.auth.enabled.store(true, Ordering::SeqCst);
    }
    assert_eq!(
        get_accounts(&app, "", "").await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        get_accounts(&app, "", &admin).await.status(),
        StatusCode::FORBIDDEN
    );
    assert!(fixture.observations.lock().unwrap().records.is_empty());
    assert!(fixture.observations.lock().unwrap().summaries.is_empty());
}

#[tokio::test]
async fn unknown_scope_fields_and_unbounded_queries_are_rejected() {
    let fixture = fixtures::fixture().await;
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let cookie = login(&app, "key").await;
    for resource in ["overview", "records"] {
        for extra in [
            "&clientApiKeyRef=other",
            "&keyId=other",
            "&account=other",
            "&provider=openai",
            "&model=%00",
            "&startTime=duplicate",
        ] {
            assert!(
                get(&app, resource, extra, &cookie)
                    .await
                    .status()
                    .is_client_error()
            );
        }
    }
    for extra in [
        "&currentPage=0",
        "&pageSize=0",
        "&pageSize=101",
        "&kind=all",
        "&currentPage=-1",
    ] {
        assert!(
            get(&app, "records", extra, &cookie)
                .await
                .status()
                .is_client_error()
        );
    }
    for range in [
        "startTime=invalid&endTime=invalid",
        "startTime=2026-01-01T00:00:00Z&endTime=2026-09-02T00:00:00Z",
        "startTime=2026-09-02T00:00:00Z&endTime=2026-09-01T00:00:00Z",
    ] {
        let response = app
            .clone()
            .oneshot(cookie_request(
                Method::GET,
                &format!("/api/key-usage/overview?{range}"),
                &cookie,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    assert!(fixture.observations.lock().unwrap().summaries.is_empty());
    assert!(fixture.observations.lock().unwrap().records.is_empty());
}

#[tokio::test]
async fn empty_usage_has_zero_cost_but_unknown_pricing_stays_unknown() {
    let fixture = fixtures::fixture().await;
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let cookie = login(&app, "key").await;
    for requests in [0, 2] {
        {
            let mut data = fixture.observations.lock().unwrap();
            let summary = data.summary.as_mut().unwrap();
            summary.requests = RequestMetrics {
                request_count: requests,
                ..Default::default()
            };
            summary.attempts.costs.clear();
            summary.attempts.cost_coverage = Default::default();
            summary.attempts.cost_coverage.unavailable_count = requests;
            data.trend = Some(Vec::new());
        }
        let data = response_json(get(&app, "overview", "", &cookie).await).await["data"].clone();
        assert_eq!(
            data["summary"]["costUsd"],
            if requests == 0 {
                json!("0")
            } else {
                Value::Null
            }
        );
        assert_eq!(data["summary"]["costIncomplete"], requests > 0);
    }
}

#[tokio::test]
async fn overview_does_not_mask_missing_keys_or_unavailable_observations() {
    let fixture = fixtures::fixture().await;
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let cookie = login(&app, "key").await;
    fixture.observations.lock().unwrap().summary = None;
    assert_eq!(
        get(&app, "overview", "", &cookie).await.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    *fixture.client_key.lock().unwrap() = None;
    assert_eq!(
        get(&app, "overview", "", &cookie).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn namespace_errors_remain_json_and_uncacheable() {
    let fixture = fixtures::fixture().await;
    let app = crate::openai::api_router_with_admin(fixture.services);
    for (method, path, status) in [
        (Method::GET, "/api/key-usage/missing", StatusCode::NOT_FOUND),
        (
            Method::POST,
            "/api/key-usage/overview",
            StatusCode::METHOD_NOT_ALLOWED,
        ),
        (
            Method::POST,
            "/api/key-usage/config",
            StatusCode::METHOD_NOT_ALLOWED,
        ),
        (
            Method::POST,
            "/api/key-usage/version",
            StatusCode::METHOD_NOT_ALLOWED,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(empty_request(method, path))
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert!(response_json(response).await["code"].is_number());
    }
}

#[tokio::test]
async fn account_panels_expose_usage_without_nested_sensitive_fields() {
    let fixture = fixtures::fixture().await;
    fixtures::bind_usage(&fixture);
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let cookie = login(&app, "key").await;
    let response = get_accounts(&app, "", &cookie).await;
    assert_eq!(response.status(), StatusCode::OK);
    let data = response_json(response).await["data"].clone();
    let recent = &data["items"][0]["usage"]["recentModel"];
    assert_eq!(recent["model"], "latest-model");
    assert_eq!(recent.as_object().unwrap().len(), 2);
    let response = get_accounts(&app, "/detail?accountId=acct_group_ready", &cookie).await;
    assert_eq!(response.status(), StatusCode::OK);
    let detail = response_json(response).await["data"].clone();
    assert_eq!(detail["usage"]["models"].as_array().unwrap().len(), 2);
    let local = &detail["quota"]["windows"][0]["localUsage"];
    assert_eq!(local["requestCount"], 4);
    assert_eq!(local["cachedTokens"], Value::Null);
    assert_eq!(local["requestBuckets"][0]["requestCount"], 4);
    let mut fields: Vec<_> = local
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    fields.sort_unstable();
    assert_eq!(
        fields,
        vec![
            "cachedTokens",
            "cachedTokensDisplay",
            "inputTokens",
            "inputTokensDisplay",
            "outputTokens",
            "outputTokensDisplay",
            "requestBuckets",
            "requestCount",
            "requestCountDisplay",
            "totalTokens",
            "totalTokensDisplay"
        ]
    );
    for payload in [data, detail] {
        let body = payload.to_string().to_lowercase();
        for forbidden in [
            "cost",
            "billing",
            "credential",
            "private-sentinel",
            "123.456",
            "notes",
            "groups",
        ] {
            assert!(
                !body.contains(forbidden),
                "unexpected field or value: {forbidden}"
            );
        }
    }
}

#[tokio::test]
async fn account_details_recheck_scope_and_session_on_every_request() {
    let fixture = fixtures::fixture().await;
    fixtures::bind_usage(&fixture);
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let cookie = login(&app, "key").await;
    let path = "/detail?accountId=acct_group_ready";
    assert_eq!(
        get_accounts(&app, path, &cookie).await.status(),
        StatusCode::OK
    );
    fixture.client_key.lock().unwrap().as_mut().unwrap().groups[0].enabled = false;
    assert_eq!(
        get_accounts(&app, path, &cookie).await.status(),
        StatusCode::NOT_FOUND
    );
    let data = response_json(get_accounts(&app, "", &cookie).await).await;
    assert_eq!(data["data"]["scopeState"], "no_enabled_groups");
    fixture
        .client_key
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .groups
        .clear();
    assert_eq!(
        get_accounts(&app, path, &cookie).await.status(),
        StatusCode::NOT_FOUND
    );
    fixtures::bind_primary_group(&fixture);
    assert_eq!(
        get_accounts(&app, "/detail?accountId=unknown", &cookie)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    fixture.auth.enabled.store(false, Ordering::SeqCst);
    assert_eq!(
        get_accounts(&app, path, &cookie).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

async fn post_account_action(app: &Router, path: &str, cookie: &str, body: Value) -> Response {
    let mut request = json_request(Method::POST, path, body);
    request
        .headers_mut()
        .insert(header::COOKIE, cookie.parse().unwrap());
    app.clone().oneshot(request).await.unwrap()
}

#[tokio::test]
async fn group_grants_are_admin_only_and_independent_from_routing() {
    let fixture = fixtures::fixture().await;
    fixtures::bind_primary_group(&fixture);
    fixture
        .client_key
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .groups
        .clear();
    fixture
        .account
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .account
        .name = "Full account name".to_owned();
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let admin = login(&app, "admin").await;
    let key = login(&app, "key").await;
    let path = "/api/admin/account-groups/key-authorizations";
    let body = json!({"id":"grp_11111111111111111111111111111111", "keyIds":["key-42"]});
    assert_eq!(
        post_account_action(&app, path, &key, body.clone())
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post_account_action(&app, path, &admin, body).await.status(),
        StatusCode::OK
    );
    let data = response_json(get_accounts(&app, "", &key).await).await;
    assert_eq!(data["data"]["scopeState"], "available");
    let account = &data["data"]["items"][0];
    assert_eq!(account["identity"], "visible@example.com");
    assert_eq!(account["name"], "Full account name");
    assert_eq!(account["capabilities"]["fullIdentity"], true);
    assert!(!data.to_string().contains("private-sentinel"));
    assert!(
        fixture
            .client_key
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .groups
            .is_empty()
    );
    let revoke = json!({"id":"grp_11111111111111111111111111111111", "keyIds":[]});
    assert_eq!(
        post_account_action(&app, path, &admin, revoke)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        get_accounts(&app, "/detail?accountId=acct_group_ready", &key)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn account_operations_require_explicit_grants_even_for_routing_members() {
    let fixture = fixtures::fixture().await;
    fixtures::bind_primary_group(&fixture);
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let key = login(&app, "key").await;
    let admin = login(&app, "admin").await;
    for suffix in [
        "personal-info",
        "profile-avatar",
        "quota-forecast",
        "reset-credits",
    ] {
        let path = format!("/{suffix}?accountId=acct_group_ready");
        assert_eq!(
            get_accounts(&app, &path, &key).await.status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            get_accounts(&app, &path, &admin).await.status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            get_accounts(&app, &path, "").await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    for (suffix, body) in [
        ("quota/refresh", json!({"accountId":"acct_group_ready"})),
        (
            "reset-credits",
            json!({"accountId":"acct_group_ready", "redeemRequestId":"3f7a6791-1b02-4a76-94c2-5b5b41e99d87"}),
        ),
    ] {
        let path = format!("/api/key-usage/accounts/{suffix}");
        assert_eq!(
            post_account_action(&app, &path, &key, body.clone())
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            post_account_action(&app, &path, &admin, body.clone())
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            post_account_action(&app, &path, "", body).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
}

#[tokio::test]
async fn authorized_operations_use_account_services_and_key_audit_identity() {
    let fixture = fixtures::fixture().await;
    fixtures::bind_usage(&fixture);
    fixture
        .account
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .account
        .name = "Authorized account".to_owned();
    // 空窗口仍返回可解释的预测状态 不要求真实上游调用
    fixture
        .account_quota
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .windows
        .clear();
    let app = crate::openai::api_router_with_admin(fixture.services.clone());
    let admin = login(&app, "admin").await;
    let key = login(&app, "key").await;
    let grants = "/api/admin/account-groups/key-authorizations";
    assert_eq!(
        post_account_action(
            &app,
            grants,
            &admin,
            json!({"id":"grp_11111111111111111111111111111111", "keyIds":["key-42"]})
        )
        .await
        .status(),
        StatusCode::OK
    );
    for suffix in [
        "personal-info",
        "profile-avatar",
        "quota-forecast",
        "reset-credits",
    ] {
        let response =
            get_accounts(&app, &format!("/{suffix}?accountId=acct_group_ready"), &key).await;
        assert_eq!(response.status(), StatusCode::OK, "{suffix}");
        assert_eq!(
            get_accounts(&app, &format!("/{suffix}?accountId=acct_other"), &key)
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    let refresh = post_account_action(
        &app,
        "/api/key-usage/accounts/quota/refresh",
        &key,
        json!({"accountId":"acct_group_ready"}),
    )
    .await;
    assert_eq!(refresh.status(), StatusCode::OK);
    let data = response_json(refresh).await;
    assert_eq!(data["data"]["capabilities"]["resetCredits"], true);
    assert!(!data.to_string().contains("private-sentinel"));
    let body = json!({"accountId":"acct_group_ready", "redeemRequestId":"3f7a6791-1b02-4a76-94c2-5b5b41e99d87"});
    let response = post_account_action(
        &app,
        "/api/key-usage/accounts/reset-credits",
        &key,
        body.clone(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["data"]["code"], "reset");
    {
        let audits = fixture.auth.audits.lock().unwrap();
        let event = audits
            .iter()
            .find(|event| event.action == "key_account.reset_credit.requested")
            .unwrap();
        assert_eq!(
            event.actor_kind,
            gateway_admin::model::auth::AuditActorKind::ClientKey
        );
        assert_eq!(event.actor_ref, "key:key-42");
        assert!(event.actor_admin_user_id.is_none());
    }
    fixture.auth.fail_audit.store(true, Ordering::SeqCst);
    assert_eq!(
        post_account_action(
            &app,
            "/api/key-usage/accounts/reset-credits",
            &key,
            body.clone()
        )
        .await
        .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    fixture.auth.fail_audit.store(false, Ordering::SeqCst);
    assert_eq!(
        post_account_action(
            &app,
            grants,
            &admin,
            json!({"id":"grp_11111111111111111111111111111111", "keyIds":[]})
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        post_account_action(&app, "/api/key-usage/accounts/reset-credits", &key, body)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let detail =
        response_json(get_accounts(&app, "/detail?accountId=acct_group_ready", &key).await).await;
    assert_eq!(detail["data"]["identity"], "vi***com");
    assert_eq!(detail["data"]["capabilities"]["fullIdentity"], false);
}
