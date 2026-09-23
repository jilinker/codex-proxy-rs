//! Key 账号操作仅通过授权服务分派 复用安全响应合同
use super::*;
use crate::admin::{
    AdminJson,
    accounts::{
        AccountActionRequest, AccountIdQuery, AccountPersonalInfoData, AccountProfileAvatarQuery,
        AccountQuotaForecastData, AccountResetCreditConsumeRequest, AccountResetCreditResultData,
        AccountResetCreditsData, profile_avatar_response,
    },
};

pub(super) fn router<S: SessionState + Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route(
            "/api/key-usage/accounts/personal-info",
            get(personal_info::<S>),
        )
        .route(
            "/api/key-usage/accounts/profile-avatar",
            get(profile_avatar::<S>),
        )
        .route(
            "/api/key-usage/accounts/quota-forecast",
            get(quota_forecast::<S>),
        )
        .route(
            "/api/key-usage/accounts/quota/refresh",
            post(refresh_quota::<S>),
        )
        .route(
            "/api/key-usage/accounts/reset-credits",
            get(reset_credits::<S>).post(consume_reset_credit::<S>),
        )
}

fn wire_error(_: crate::admin::WireValidationError) -> AdminError {
    AdminError::bad_request("账号操作参数不合法")
}

async fn personal_info<S: SessionState + Send + Sync>(
    State(state): State<S>,
    headers: HeaderMap,
    AdminQuery(query): AdminQuery<AccountIdQuery>,
) -> Result<impl IntoResponse, AdminError> {
    let result = state
        .admin_services()
        .key_usage()
        .personal_info(
            session_cookie::value(&headers).as_deref(),
            query.into_id().map_err(wire_error)?,
        )
        .await
        .map_err(map_admin_service_error)?
        .ok_or_else(AdminError::session_required)?;
    let mut data = AccountPersonalInfoData::from(result);
    if data.profile_error.is_some() {
        data.profile_error = Some("个人信息暂不可用".to_owned());
    }
    Ok(AdminResponse::new(StatusCode::OK, AdminEnvelope::ok(data)))
}

async fn quota_forecast<S: SessionState + Send + Sync>(
    State(state): State<S>,
    headers: HeaderMap,
    AdminQuery(query): AdminQuery<AccountIdQuery>,
) -> Result<impl IntoResponse, AdminError> {
    let result = state
        .admin_services()
        .key_usage()
        .quota_forecast(
            session_cookie::value(&headers).as_deref(),
            query.into_id().map_err(wire_error)?,
        )
        .await
        .map_err(map_admin_service_error)?
        .ok_or_else(AdminError::session_required)?;
    let data = AccountQuotaForecastData::from(result);
    Ok(AdminResponse::new(StatusCode::OK, AdminEnvelope::ok(data)))
}

async fn reset_credits<S: SessionState + Send + Sync>(
    State(state): State<S>,
    headers: HeaderMap,
    AdminQuery(query): AdminQuery<AccountIdQuery>,
) -> Result<impl IntoResponse, AdminError> {
    let result = state
        .admin_services()
        .key_usage()
        .reset_credits(
            session_cookie::value(&headers).as_deref(),
            query.into_id().map_err(wire_error)?,
        )
        .await
        .map_err(map_admin_service_error)?
        .ok_or_else(AdminError::session_required)?;
    let data = AccountResetCreditsData::from(result);
    Ok(AdminResponse::new(StatusCode::OK, AdminEnvelope::ok(data)))
}

async fn profile_avatar<S: SessionState + Send + Sync>(
    State(state): State<S>,
    headers: HeaderMap,
    AdminQuery(query): AdminQuery<AccountProfileAvatarQuery>,
) -> Result<Response, AdminError> {
    let result = state
        .admin_services()
        .key_usage()
        .profile_avatar(
            session_cookie::value(&headers).as_deref(),
            query.into_id().map_err(wire_error)?,
        )
        .await
        .map_err(map_admin_service_error)?
        .ok_or_else(AdminError::session_required)?;
    Ok(profile_avatar_response(result))
}
async fn refresh_quota<S: SessionState + Send + Sync>(
    State(state): State<S>,
    headers: HeaderMap,
    AdminJson(request): AdminJson<AccountActionRequest>,
) -> Result<impl IntoResponse, AdminError> {
    let result = state
        .admin_services()
        .key_usage()
        .refresh_quota(
            session_cookie::value(&headers).as_deref(),
            request.into_id().map_err(wire_error)?,
        )
        .await
        .map_err(map_admin_service_error)?
        .ok_or_else(AdminError::session_required)?;
    Ok(AdminResponse::new(
        StatusCode::OK,
        AdminEnvelope::ok(presenter::account_detail(result)),
    ))
}
async fn consume_reset_credit<S: SessionState + Send + Sync>(
    State(state): State<S>,
    headers: HeaderMap,
    AdminJson(request): AdminJson<AccountResetCreditConsumeRequest>,
) -> Result<impl IntoResponse, AdminError> {
    let result = state
        .admin_services()
        .key_usage()
        .consume_reset_credit(
            session_cookie::value(&headers).as_deref(),
            request.into_command().map_err(wire_error)?,
        )
        .await
        .map_err(map_admin_service_error)?
        .ok_or_else(AdminError::session_required)?;
    Ok(AdminResponse::new(
        StatusCode::OK,
        AdminEnvelope::ok(AccountResetCreditResultData::from(result)),
    ))
}
