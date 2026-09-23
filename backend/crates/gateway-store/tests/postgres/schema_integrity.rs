use super::TestDatabase;

async fn seed_request(pool: &sqlx::PgPool) {
    sqlx::query(
        "insert into model_requests (
           id, client_api_key_ref, config_revision, protocol, operation, endpoint,
           client_transport, started_at, deadline_at, outcome, completed_at, routing_scope
         ) values (
           'req_integrity', 'deleted_key', 1, 'openai', 'responses', '/v1/responses',
           'http_sse', now(), now() + interval '1 hour', 'failed', now(), 'all'
         )",
    )
    .execute(pool)
    .await
    .expect("seed request without optional facts");
}

fn assert_check_rejected(error: &sqlx::Error) {
    assert_eq!(
        error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("23514")
    );
}

#[tokio::test]
async fn request_fact_groups_reject_partial_writes_and_accept_complete_observations() {
    let Some(db) = TestDatabase::create("fact_groups").await else {
        return;
    };
    seed_request(&db.pool).await;
    for assignments in [
        "cost_source = 'calculated', cost_amount = 1",
        "capacity_used_slots = 1",
        "capacity_total_slots = 2",
        "upstream_connection_id = 'connection'",
        "upstream_connection_id = 'connection', upstream_connection_exit_reason = 'peer_closed', upstream_connection_age_ms = 10",
        "recovery_request_id = 'req_recovery', recovered_at = completed_at, recovery_attempt_count = 1",
        "recovery_request_id = 'req_recovery', recovered_at = completed_at, recovery_attempt_count = 1, recovery_retry_delay_ms = 0",
        "outcome = 'running', completed_at = null, recovered_at = now(), recovery_request_id = 'req_recovery', recovery_attempt_count = 1, recovery_retry_delay_ms = 0, recovery_total_latency_ms = 1",
    ] {
        let error = sqlx::query(sqlx::AssertSqlSafe(format!(
            "update model_requests set {assignments} where id = 'req_integrity'"
        )))
        .execute(&db.pool)
        .await
        .expect_err(assignments);
        assert_check_rejected(&error);
    }
    sqlx::query(
        "update model_requests set cost_source = 'calculated', cost_amount = 1,
           cost_currency = 'USD', capacity_used_slots = 1, capacity_total_slots = 2,
           upstream_connection_id = 'connection', upstream_connection_exit_reason = 'peer_closed',
           upstream_connection_age_ms = 10, upstream_connection_idle_ms = 2,
           recovery_request_id = 'req_recovery', recovered_at = completed_at,
           recovery_attempt_count = 1, recovery_retry_delay_ms = 0, recovery_total_latency_ms = 1
         where id = 'req_integrity'",
    )
    .execute(&db.pool)
    .await
    .expect("complete observations satisfy presence and value constraints");
    db.close().await;
}

#[tokio::test]
async fn account_snapshots_are_required_before_live_foreign_keys_can_be_cleared() {
    let Some(db) = TestDatabase::create("account_fact_refs").await else {
        return;
    };
    seed_request(&db.pool).await;
    sqlx::query(
        "insert into provider_accounts (id, provider_kind, name, authentication_kind,
           provider_credentials_json, has_refresh_token, credential_observed_at, created_at, updated_at)
         values ('acct_integrity', 'openai', 'test', 'oauth', '{}', false, now(), now(), now())",
    ).execute(&db.pool).await.expect("seed account");
    let error = sqlx::query("update model_requests set provider_account_id = 'acct_integrity'")
        .execute(&db.pool)
        .await
        .expect_err("request requires snapshot ref");
    assert_check_rejected(&error);
    let error = sqlx::query(
        "insert into ops_events (id, level, component, operation, provider_account_id,
           failure_kind, message, created_at)
         values ('ops_missing_ref', 'error', 'probe', 'probe', 'acct_integrity', 'timeout', 'test', now())",
    ).execute(&db.pool).await.expect_err("event requires snapshot ref");
    assert_check_rejected(&error);
    sqlx::query(
        "update model_requests set provider_account_id = 'acct_integrity', provider_account_ref = 'acct_integrity'",
    ).execute(&db.pool).await.expect("write complete account reference");
    sqlx::query("delete from provider_accounts where id = 'acct_integrity'")
        .execute(&db.pool)
        .await
        .expect("delete live account through narrowed FK index");
    let refs: (Option<String>, String) = sqlx::query_as(
        "select provider_account_id, provider_account_ref from model_requests where id = 'req_integrity'",
    ).fetch_one(&db.pool).await.expect("read retained snapshot");
    assert_eq!(refs, (None, "acct_integrity".to_owned()));
    db.close().await;
}

#[tokio::test]
async fn audit_requires_canonical_admin_identity_and_retains_it_after_admin_deletion() {
    let Some(db) = TestDatabase::create("audit_identity").await else {
        return;
    };
    sqlx::raw_sql(
        "insert into admin_users (id, password_hash, created_at, updated_at)
           values ('admin_test', 'test_hash', now(), now());
         insert into admin_audit_events (id, actor_kind, actor_admin_user_id, actor_ref,
           action, entity_kind, entity_ref, created_at)
           values ('audit_identity', 'admin_session', 'admin_test', 'admin:admin_test',
             'update', 'settings', '1', now());",
    )
    .execute(&db.pool)
    .await
    .expect("seed audit event with canonical admin identity");
    let error = sqlx::query("update admin_audit_events set actor_ref = 'admin_test'")
        .execute(&db.pool)
        .await
        .expect_err("new writes require the canonical actor identity");
    assert_check_rejected(&error);
    sqlx::query("delete from admin_users where id = 'admin_test'")
        .execute(&db.pool)
        .await
        .expect("audit retains identity when live admin is deleted");
    let identity: (Option<String>, String) =
        sqlx::query_as("select actor_admin_user_id, actor_ref from admin_audit_events")
            .fetch_one(&db.pool)
            .await
            .expect("read historical identity");
    assert_eq!(identity, (None, "admin:admin_test".to_owned()));
    db.close().await;
}

#[tokio::test]
async fn backup_completion_cannot_precede_its_start() {
    let Some(db) = TestDatabase::create("backup_time_order").await else {
        return;
    };
    let error = sqlx::query(
        "insert into backup_records (id, trigger_kind, status, object_key, size_bytes, sha256,
           started_at, completed_at, created_at, updated_at)
         values ('backup_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'manual', 'completed', 'test.dump',
           1, repeat('a', 64), now() + interval '10 seconds', now() + interval '5 seconds',
           now(), now() + interval '20 seconds')",
    )
    .execute(&db.pool)
    .await
    .expect_err("completed backup must not end before it starts");
    assert_check_rejected(&error);
    db.close().await;
}

// 只构造公开测试标识 不包含真实 Key 或账号信息
async fn seed_account_authorization_subjects(pool: &sqlx::PgPool) {
    sqlx::raw_sql(
        "insert into account_groups (id, name, color, created_at, updated_at) values
           ('grp_000000000000000000000000000000a1', 'Authorization A', '#2563EBFF', now(), now()),
           ('grp_000000000000000000000000000000a2', 'Authorization B', '#2563EBFF', now(), now());
         insert into client_api_keys (id, name, key, created_at, updated_at) values
           ('key_grant_a', 'Grant A', 'sk_schema_grant_a', now(), now()),
           ('key_grant_b', 'Grant B', 'sk_schema_grant_b', now(), now());
         insert into client_api_key_groups (client_api_key_id, account_group_id, created_at)
           values ('key_grant_a', 'grp_000000000000000000000000000000a1', now());",
    )
    .execute(pool)
    .await
    .expect("seed independent routing and authorization subjects");
}

// 路由绑定升级后保持原样且不会隐式获得账号操作权限
#[tokio::test]
async fn account_authorization_migration_preserves_routing_without_implicit_grants() {
    let Some(db) = TestDatabase::create_through("account_grant_upgrade", 17).await else {
        return;
    };
    seed_account_authorization_subjects(&db.pool).await;
    super::TEST_MIGRATOR
        .run(&db.pool)
        .await
        .expect("upgrade existing schema with routing data");
    let count: i64 = sqlx::query_scalar("select count(*) from account_group_key_authorizations")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let bindings: Vec<(String, String)> =
        sqlx::query_as("select client_api_key_id, account_group_id from client_api_key_groups")
            .fetch_all(&db.pool)
            .await
            .unwrap();
    assert_eq!(
        bindings,
        vec![(
            "key_grant_a".to_owned(),
            "grp_000000000000000000000000000000a1".to_owned()
        )]
    );
    super::TEST_MIGRATOR
        .run(&db.pool)
        .await
        .expect("restart with applied migrations");
    db.close().await;
}

// 联合主键和外键保护授权关系并允许独立于路由的多对多授权
#[tokio::test]
async fn account_authorizations_enforce_unique_pairs_and_existing_subjects() {
    let Some(db) = TestDatabase::create("account_grant_constraints").await else {
        return;
    };
    seed_account_authorization_subjects(&db.pool).await;
    sqlx::query(
        "insert into account_group_key_authorizations (account_group_id, client_api_key_id, created_at)
         select g.id, k.id, now() from account_groups g cross join client_api_keys k",
    )
    .execute(&db.pool)
    .await
    .expect("grant each group to multiple keys without routing prerequisites");
    for (group, key, code) in [
        (
            "grp_000000000000000000000000000000a1",
            "key_grant_a",
            "23505",
        ),
        (
            "grp_000000000000000000000000000000ff",
            "key_grant_a",
            "23503",
        ),
        (
            "grp_000000000000000000000000000000a1",
            "key_missing",
            "23503",
        ),
    ] {
        let error = sqlx::query(
            "insert into account_group_key_authorizations (account_group_id, client_api_key_id, created_at)
             values ($1, $2, now())",
        )
        .bind(group)
        .bind(key)
        .execute(&db.pool)
        .await
        .expect_err("reject duplicate or orphan authorization");
        assert_eq!(
            error
                .as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some(code)
        );
    }
    let counts: (i64, i64) = sqlx::query_as(
        "select (select count(*) from account_group_key_authorizations),
                (select count(*) from client_api_key_groups)",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(counts, (4, 1));
    db.close().await;
}

// 授权随主体删除清理 既有路由分组的删除保护继续生效
#[tokio::test]
async fn account_authorizations_cascade_without_weakening_routing_references() {
    let Some(db) = TestDatabase::create("account_grant_delete").await else {
        return;
    };
    seed_account_authorization_subjects(&db.pool).await;
    sqlx::query(
        "insert into account_group_key_authorizations (account_group_id, client_api_key_id, created_at)
         select g.id, k.id, now() from account_groups g cross join client_api_keys k",
    ).execute(&db.pool).await.unwrap();
    let error =
        sqlx::query("delete from account_groups where id = 'grp_000000000000000000000000000000a1'")
            .execute(&db.pool)
            .await
            .expect_err("routing-bound group remains protected");
    assert_eq!(
        error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("23001")
    );
    sqlx::query("delete from account_groups where id = 'grp_000000000000000000000000000000a2'")
        .execute(&db.pool)
        .await
        .expect("delete grant-only group");
    sqlx::query("delete from client_api_keys where id = 'key_grant_b'")
        .execute(&db.pool)
        .await
        .expect("delete authorized key");
    let remaining: Vec<(String, String)> = sqlx::query_as(
        "select account_group_id, client_api_key_id from account_group_key_authorizations",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    assert_eq!(
        remaining,
        vec![(
            "grp_000000000000000000000000000000a1".to_owned(),
            "key_grant_a".to_owned()
        )]
    );
    db.close().await;
}
