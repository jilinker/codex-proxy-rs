//! 单页 Key 用量的查询合同；身份范围不由调用方提供。

use super::{
    PageSize,
    accounts::{AccountPageItem, AccountUsage},
    client_keys::ClientKeyRecord,
    observability::{
        HealthTimeline, OpsErrorPage, RequestMetricPoint, TimeRange, UsageOverview, UsagePage,
    },
    provider_credentials::ProviderQuota,
};

#[derive(Debug, Clone)]
pub struct KeyUsageQuery {
    pub range: TimeRange,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum KeyUsageRecordKind {
    Success,
    Error,
}

#[derive(Debug, Clone)]
pub struct KeyUsageRecordsQuery {
    pub usage: KeyUsageQuery,
    pub kind: KeyUsageRecordKind,
    pub current_page: u32,
    pub page_size: PageSize,
}

pub struct KeyUsageOverview {
    pub key: ClientKeyRecord,
    pub overview: UsageOverview,
    pub trend: Vec<RequestMetricPoint>,
    pub health_timeline: HealthTimeline,
}

pub enum KeyUsageRecords {
    Success(UsagePage),
    Error(OpsErrorPage),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyUsageAccountListQuery {
    pub current_page: u32,
    pub page_size: PageSize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyUsageAccountDetailQuery {
    pub account_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyUsageAccountScopeState {
    Unbound,
    NoEnabledGroups,
    Available,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyUsageQuotaAvailability {
    Available,
    Unsupported,
    Unobserved,
    Unavailable,
}

#[derive(Debug, Clone)]
pub struct KeyUsageAccountQuota {
    pub availability: KeyUsageQuotaAvailability,
    pub value: Option<ProviderQuota>,
}

#[derive(Debug, Clone)]
pub struct KeyUsageAccountSnapshot {
    pub item: AccountPageItem,
    pub plan_type_display: Option<String>,
    pub quota: KeyUsageAccountQuota,
    pub usage: Option<AccountUsage>,
}

#[derive(Debug, Clone)]
pub struct KeyUsageAccountList {
    pub scope_state: KeyUsageAccountScopeState,
    pub items: Vec<KeyUsageAccountSnapshot>,
    pub current_page: u32,
    pub page_size: u16,
    pub total: u64,
}
