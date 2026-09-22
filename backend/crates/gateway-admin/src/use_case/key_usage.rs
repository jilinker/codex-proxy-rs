//! Key 自助查询从统一会话或只读 Key 校验取得范围，复用现有观测和额度账本。

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use async_trait::async_trait;
use chrono::{Duration, Utc};

use crate::{
    AuthService, SystemService,
    model::{
        AdminError, AdminErrorKind,
        auth::SessionSubject,
        client_keys::ClientKeySecret,
        key_usage::{
            KeyUsageAccountDetailQuery, KeyUsageAccountList, KeyUsageAccountListQuery,
            KeyUsageAccountQuota, KeyUsageAccountScopeState, KeyUsageAccountSnapshot,
            KeyUsageOverview, KeyUsageQuery, KeyUsageQuotaAvailability, KeyUsageRecordKind,
            KeyUsageRecords, KeyUsageRecordsQuery,
        },
        observability::{
            OpsErrorFilter, OpsErrorQuery, TimeRange, UsageFilter, UsageQuery, china_day_start,
        },
        system::SystemVersion,
    },
    ports::{
        provider::{ProviderAdminErrorKind, ProviderAdminRegistry},
        store::{
            AccountGroupStore, AccountStore, AdminStorePorts, ClientKeyStore, ObservabilityStore,
        },
    },
};
use gateway_core::{
    account::ProviderAccountId,
    engine::{
        budget::ClientBudgetStatus,
        execution::{ClientAuthenticationError, ClientKeyVerifier},
    },
    policy::ClientApiKeyId,
};

use super::{map_store_error, observability::health_timeline_at};

#[async_trait]
pub trait KeyUsageService: Send + Sync {
    /// 验证 Key 并只读查询当前额度，不记录 Key 使用或执行推理准入。
    async fn budget(&self, plaintext: &str) -> Result<Option<ClientBudgetStatus>, AdminError>;

    async fn version(&self, session_id: Option<&str>) -> Result<Option<SystemVersion>, AdminError>;

    async fn config(&self, session_id: Option<&str>)
    -> Result<Option<ClientKeySecret>, AdminError>;

    async fn overview(
        &self,
        session_id: Option<&str>,
        query: KeyUsageQuery,
    ) -> Result<Option<KeyUsageOverview>, AdminError>;

    async fn records(
        &self,
        session_id: Option<&str>,
        query: KeyUsageRecordsQuery,
    ) -> Result<Option<KeyUsageRecords>, AdminError>;

    async fn accounts(
        &self,
        session_id: Option<&str>,
        query: KeyUsageAccountListQuery,
    ) -> Result<Option<KeyUsageAccountList>, AdminError>;

    async fn account_detail(
        &self,
        session_id: Option<&str>,
        query: KeyUsageAccountDetailQuery,
    ) -> Result<Option<KeyUsageAccountSnapshot>, AdminError>;
}

pub(crate) struct DefaultKeyUsageService {
    auth: Arc<dyn AuthService>,
    verifier: Arc<dyn ClientKeyVerifier>,
    keys: Arc<dyn ClientKeyStore>,
    account_groups: Arc<dyn AccountGroupStore>,
    accounts: Arc<dyn AccountStore>,
    observations: Arc<dyn ObservabilityStore>,
    providers: ProviderAdminRegistry,
    system: Arc<dyn SystemService>,
}

impl DefaultKeyUsageService {
    pub(crate) fn new(
        auth: Arc<dyn AuthService>,
        verifier: Arc<dyn ClientKeyVerifier>,
        store: &AdminStorePorts,
        providers: ProviderAdminRegistry,
        system: Arc<dyn SystemService>,
    ) -> Self {
        Self {
            auth,
            verifier,
            keys: store.client_keys(),
            account_groups: store.account_groups(),
            accounts: store.accounts(),
            observations: store.observability(),
            providers,
            system,
        }
    }

    async fn key_id(&self, session_id: Option<&str>) -> Result<Option<ClientApiKeyId>, AdminError> {
        match self
            .auth
            .session(session_id)
            .await?
            .map(|session| session.subject)
        {
            Some(SessionSubject::Key { client_key_id }) => Ok(Some(client_key_id)),
            Some(SessionSubject::Admin { .. }) => Err(AdminError::new(
                AdminErrorKind::Forbidden,
                "当前身份无权访问密钥用量接口",
            )),
            None => Ok(None),
        }
    }

    async fn key(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<crate::model::client_keys::ClientKeyRecord>, AdminError> {
        let Some(id) = self.key_id(session_id).await? else {
            return Ok(None);
        };
        self.keys
            .get_client_key(&id)
            .await
            .map_err(|error| map_store_error(error, "key usage profile"))
            .map(|key| key.filter(|key| key.enabled))
    }

    async fn account_scope(
        &self,
        key: &crate::model::client_keys::ClientKeyRecord,
    ) -> Result<(KeyUsageAccountScopeState, Vec<String>), AdminError> {
        if key.groups.is_empty() {
            return Ok((KeyUsageAccountScopeState::Unbound, Vec::new()));
        }
        let group_ids = key
            .groups
            .iter()
            .filter(|group| group.enabled)
            .map(|group| group.id.clone())
            .collect::<Vec<_>>();
        if group_ids.is_empty() {
            return Ok((KeyUsageAccountScopeState::NoEnabledGroups, Vec::new()));
        }
        let account_ids = self
            .account_groups
            .load_account_group_members(&group_ids)
            .await
            .map_err(|error| map_store_error(error, "key usage account scope"))?
            .into_iter()
            .map(|member| member.account_id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Ok((KeyUsageAccountScopeState::Available, account_ids))
    }

    async fn account_snapshots(
        &self,
        account_ids: &[String],
    ) -> Result<Vec<KeyUsageAccountSnapshot>, AdminError> {
        if account_ids.is_empty() {
            return Ok(Vec::new());
        }
        let loaded = futures::future::join_all(account_ids.iter().map(|account_id| {
            self.accounts.load_account(
                account_id,
                crate::model::accounts::AccountRuntimeSnapshot::default(),
            )
        }))
        .await;
        let mut items = Vec::with_capacity(account_ids.len());
        for result in loaded {
            if let Some(item) =
                result.map_err(|error| map_store_error(error, "key usage account"))?
            {
                items.push(item);
            }
        }
        let quota_results = futures::future::join_all(items.iter().map(|item| async {
            if item.account.authentication_kind == "api_key" {
                return KeyUsageAccountQuota {
                    availability: KeyUsageQuotaAvailability::Unsupported,
                    value: None,
                };
            }
            let account_id = match ProviderAccountId::new(item.account.id.clone()) {
                Ok(account_id) => account_id,
                Err(_) => {
                    return KeyUsageAccountQuota {
                        availability: KeyUsageQuotaAvailability::Unavailable,
                        value: None,
                    };
                }
            };
            let provider = match self.providers.require(&item.account.provider_kind) {
                Ok(provider) => provider,
                Err(_) => {
                    return KeyUsageAccountQuota {
                        availability: KeyUsageQuotaAvailability::Unavailable,
                        value: None,
                    };
                }
            };
            match provider.quota_snapshot(&account_id).await {
                Ok(quota) if quota.observed_at.is_some() || !quota.windows.is_empty() => {
                    KeyUsageAccountQuota {
                        availability: KeyUsageQuotaAvailability::Available,
                        value: Some(quota),
                    }
                }
                Ok(quota) => KeyUsageAccountQuota {
                    availability: KeyUsageQuotaAvailability::Unobserved,
                    value: Some(quota),
                },
                Err(error) if error.kind() == ProviderAdminErrorKind::Unsupported => {
                    KeyUsageAccountQuota {
                        availability: KeyUsageQuotaAvailability::Unsupported,
                        value: None,
                    }
                }
                Err(_) => KeyUsageAccountQuota {
                    availability: KeyUsageQuotaAvailability::Unavailable,
                    value: None,
                },
            }
        }))
        .await;
        let now = Utc::now();
        let mut snapshots = items
            .into_iter()
            .zip(quota_results)
            .map(|(item, quota)| KeyUsageAccountSnapshot {
                plan_type_display: self.providers.plan_type_display(
                    item.account.provider_kind.as_str(),
                    item.account.plan_type.as_deref(),
                ),
                item,
                quota,
                usage: None,
            })
            .collect::<Vec<_>>();
        let windows = snapshots
            .iter()
            .flat_map(|snapshot| {
                if snapshot.item.account.authentication_kind == "api_key" {
                    return vec![crate::model::accounts::AccountUsageWindowQuery {
                        account_id: snapshot.item.account.id.clone(),
                        key: "account-lifetime".to_owned(),
                        range: TimeRange {
                            start: snapshot.item.account.created_at,
                            end: now,
                        },
                    }];
                }
                snapshot
                    .quota
                    .value
                    .iter()
                    .flat_map(|quota| &quota.windows)
                    .filter_map(|window| quota_usage_window(&snapshot.item.account.id, window))
                    .collect()
            })
            .collect::<Vec<_>>();
        if windows.is_empty() {
            return Ok(snapshots);
        }
        let usage = self
            .accounts
            .load_account_usage_by_windows(&windows)
            .await
            .map_err(|error| map_store_error(error, "key usage account usage"))?
            .into_iter()
            .map(|result| ((result.account_id, result.key), result.usage))
            .collect::<BTreeMap<_, _>>();
        for snapshot in &mut snapshots {
            let account_id = snapshot.item.account.id.clone();
            if snapshot.item.account.authentication_kind == "api_key" {
                snapshot.usage = usage
                    .get(&(account_id, "account-lifetime".to_owned()))
                    .cloned();
                continue;
            }
            if let Some(quota) = snapshot.quota.value.as_mut() {
                for window in &mut quota.windows {
                    if let Some(value) = usage.get(&(account_id.clone(), window.key.clone())) {
                        window.local_usage = Some(value.clone());
                    }
                }
                snapshot.usage = quota
                    .usage_window()
                    .and_then(|(window, _)| window.local_usage.clone());
            }
        }
        Ok(snapshots)
    }
}

fn quota_usage_window(
    account_id: &str,
    window: &crate::model::provider_credentials::ProviderQuotaWindow,
) -> Option<crate::model::accounts::AccountUsageWindowQuery> {
    use crate::model::provider_credentials::QuotaLocalUsageAttribution;
    if window.local_usage_attribution != QuotaLocalUsageAttribution::AccountWide {
        return None;
    }
    let reset_at = window.reset_at?;
    let seconds = i64::try_from(window.window_seconds?).ok()?;
    let start = reset_at.checked_sub_signed(Duration::try_seconds(seconds)?)?;
    Some(crate::model::accounts::AccountUsageWindowQuery {
        account_id: account_id.to_owned(),
        key: window.key.clone(),
        range: TimeRange::new(start, reset_at).ok()?,
    })
}

fn usage_filter(id: &ClientApiKeyId, model: Option<String>) -> UsageFilter {
    UsageFilter {
        client_api_key_ref: Some(id.as_str().to_owned()),
        model,
        ..UsageFilter::default()
    }
}

#[async_trait]
impl KeyUsageService for DefaultKeyUsageService {
    async fn budget(&self, plaintext: &str) -> Result<Option<ClientBudgetStatus>, AdminError> {
        let id = match self.verifier.verify_client_key(plaintext) {
            Ok(id) => id,
            Err(ClientAuthenticationError::InvalidKey) => return Ok(None),
            Err(ClientAuthenticationError::SnapshotUnavailable) => {
                return Err(AdminError::new(
                    AdminErrorKind::Unavailable,
                    "密钥验证暂时不可用",
                ));
            }
        };
        self.keys
            .get_client_key(&id)
            .await
            .map(|key| key.filter(|key| key.enabled).map(|key| key.budget))
            .map_err(|error| map_store_error(error, "key usage budget"))
    }

    async fn version(&self, session_id: Option<&str>) -> Result<Option<SystemVersion>, AdminError> {
        if self.key_id(session_id).await?.is_none() {
            return Ok(None);
        }
        self.system.version().await.map(Some)
    }

    async fn config(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<ClientKeySecret>, AdminError> {
        let Some(id) = self.key_id(session_id).await? else {
            return Ok(None);
        };
        // 明文只按服务端会话绑定的 Key 读取，禁用或删除后不再提供配置。
        self.keys
            .reveal_client_key(&id)
            .await
            .map(|secret| secret.filter(|secret| secret.record.enabled))
            .map_err(|error| map_store_error(error, "key usage config"))
    }

    async fn overview(
        &self,
        session_id: Option<&str>,
        query: KeyUsageQuery,
    ) -> Result<Option<KeyUsageOverview>, AdminError> {
        let Some(key) = self.key(session_id).await? else {
            return Ok(None);
        };
        let id = key.id.clone();
        let filter = usage_filter(&id, query.model);
        let now = Utc::now();
        // 健康条始终展示北京时间今日，不随历史范围或模型筛选改变。
        let today = TimeRange {
            start: china_day_start(now),
            end: now,
        };
        let (overview, trend, health_points) = futures::try_join!(
            self.observations.usage_summary(query.range, filter.clone()),
            self.observations.usage_trend(query.range, filter),
            self.observations
                .usage_trend(today, usage_filter(&id, None)),
        )
        .map_err(|error| map_store_error(error, "key usage overview"))?;
        Ok(Some(KeyUsageOverview {
            key,
            overview,
            trend,
            health_timeline: health_timeline_at(&health_points, now),
        }))
    }

    async fn records(
        &self,
        session_id: Option<&str>,
        query: KeyUsageRecordsQuery,
    ) -> Result<Option<KeyUsageRecords>, AdminError> {
        let Some(id) = self.key_id(session_id).await? else {
            return Ok(None);
        };
        let result = match query.kind {
            KeyUsageRecordKind::Success => KeyUsageRecords::Success(
                self.observations
                    .list_usage_records(UsageQuery {
                        range: query.usage.range,
                        filter: usage_filter(&id, query.usage.model),
                        current_page: query.current_page,
                        page_size: query.page_size,
                    })
                    .await
                    .map_err(|error| map_store_error(error, "key usage records"))?,
            ),
            KeyUsageRecordKind::Error => KeyUsageRecords::Error(
                self.observations
                    .list_ops_errors(OpsErrorQuery {
                        range: query.usage.range,
                        filter: OpsErrorFilter {
                            client_api_key_ref: Some(id.as_str().to_owned()),
                            model: query.usage.model,
                            ..OpsErrorFilter::default()
                        },
                        current_page: query.current_page,
                        page_size: query.page_size,
                    })
                    .await
                    .map_err(|error| map_store_error(error, "key usage errors"))?,
            ),
        };
        Ok(Some(result))
    }

    async fn accounts(
        &self,
        session_id: Option<&str>,
        query: KeyUsageAccountListQuery,
    ) -> Result<Option<KeyUsageAccountList>, AdminError> {
        let Some(key) = self.key(session_id).await? else {
            return Ok(None);
        };
        let (scope_state, account_ids) = self.account_scope(&key).await?;
        let total = u64::try_from(account_ids.len()).unwrap_or(u64::MAX);
        let start = usize::try_from(query.current_page.saturating_sub(1))
            .unwrap_or(usize::MAX)
            .saturating_mul(usize::from(query.page_size.get()));
        let page_ids = account_ids
            .get(
                start
                    ..account_ids
                        .len()
                        .min(start.saturating_add(usize::from(query.page_size.get()))),
            )
            .unwrap_or_default();
        Ok(Some(KeyUsageAccountList {
            scope_state,
            items: self.account_snapshots(page_ids).await?,
            current_page: query.current_page,
            page_size: query.page_size.get(),
            total,
        }))
    }

    async fn account_detail(
        &self,
        session_id: Option<&str>,
        query: KeyUsageAccountDetailQuery,
    ) -> Result<Option<KeyUsageAccountSnapshot>, AdminError> {
        let Some(key) = self.key(session_id).await? else {
            return Ok(None);
        };
        let (_, account_ids) = self.account_scope(&key).await?;
        if account_ids.binary_search(&query.account_id).is_err() {
            return Err(AdminError::new(AdminErrorKind::NotFound, "资源不存在"));
        }
        self.account_snapshots(std::slice::from_ref(&query.account_id))
            .await?
            .into_iter()
            .next()
            .map(Some)
            .ok_or_else(|| AdminError::new(AdminErrorKind::NotFound, "资源不存在"))
    }
}
