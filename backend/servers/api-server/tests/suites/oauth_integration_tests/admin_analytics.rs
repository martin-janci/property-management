//! OAuth admin client-audit + token-usage analytics / read-route tests.
//!
//! Split from the former monolithic `oauth_integration_tests.rs`
//! (by OAuth flow). Test bodies are unchanged.

pub(crate) use super::fixtures::*;

// ─── module: admin_client_audit ─────────────────────────────────────────────
//
// Findings #768 / #800: privileged OAuth-client mutations on the admin surface
// must be audited. `update_client` (PATCH /api/v1/admin/oauth/clients/{id})
// previously wrote NO audit_log row at all and silently dropped the operator
// `reason` the admin-web UI requires for scope edits. These tests pin the
// regression: an `oauth_client_update` row is written, and the supplied
// `reason` is persisted into its `details`.
#[cfg(test)]
mod admin_client_audit {
    use super::*;
    use api_server::services::JwtService;

    const TEST_JWT_SECRET: &str =
        "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

    /// Seed a platform-principal user. Returns the new user id.
    async fn seed_platform_user(pool: &PgPool, email: &str) -> Uuid {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
            VALUES ($1, 'h', 'OAuth Admin Test', 'active', NOW(), 'platform')
            RETURNING id
            "#,
        )
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("seed platform user")
    }

    /// Mark the user as MFA-enrolled (the capability gate's step 2.5 wall).
    async fn enroll_mfa(pool: &PgPool, user_id: Uuid) {
        sqlx::query(
            r#"
            INSERT INTO user_2fa (user_id, secret, enabled, enabled_at, backup_codes, backup_codes_remaining)
            VALUES ($1, 'unused-secret', true, NOW(), '[]'::jsonb, 0)
            ON CONFLICT (user_id) DO UPDATE SET enabled = true, enabled_at = NOW()
            "#,
        )
        .bind(user_id)
        .execute(pool)
        .await
        .expect("enroll mfa");
    }

    /// Grant `oauth_client_write` to the user with `mfa_required = false` so
    /// the capability extractor's recent-MFA recency check is skipped (we are
    /// not exercising MFA here — only the audit behaviour of update_client).
    async fn grant_oauth_client_write(pool: &PgPool, user_id: Uuid, granted_by: Uuid) {
        sqlx::query(
            r#"
            INSERT INTO capability_grants
                (user_id, capability, granted_by, expires_at, mfa_required, note)
            VALUES ($1, 'oauth_client_write', $2, NULL, false, 'ci-audit-test')
            "#,
        )
        .bind(user_id)
        .bind(granted_by)
        .execute(pool)
        .await
        .expect("grant oauth_client_write");
    }

    /// Look up the internal UUID of a seeded client by its `client_id`.
    async fn client_uuid(pool: &PgPool, client_id: &str) -> Uuid {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM oauth_clients WHERE client_id = $1")
            .bind(client_id)
            .fetch_one(pool)
            .await
            .expect("client uuid")
    }

    /// Mint a platform-kind bearer JWT validated by `TestApp`'s JWT secret.
    fn mint_platform_token(user_id: Uuid, email: &str) -> String {
        let svc = JwtService::new(TEST_JWT_SECRET).expect("jwt service");
        svc.generate_access_token_with_kind(
            user_id,
            email,
            "OAuth Admin Test",
            None,
            None,
            Some("platform".to_string()),
        )
        .expect("mint token")
    }

    /// Build a PATCH request (the test RequestBuilder has no `patch` helper).
    fn patch_request(uri: &str, token: &str, body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method(Method::PATCH)
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    /// Updating a client's scopes via the admin route must write exactly one
    /// `oauth_client_update` audit row, attributed to the acting operator,
    /// with the supplied `reason` persisted in `details.reason`.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    #[ignore = "requires postgres + migrations"]
    async fn update_client_scope_change_writes_audit_with_reason(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;

        let admin = seed_platform_user(&pool, "oauth-audit-admin@test.local").await;
        let granter = seed_platform_user(&pool, "oauth-audit-granter@test.local").await;
        enroll_mfa(&pool, admin).await;
        grant_oauth_client_write(&pool, admin, granter).await;
        let token = mint_platform_token(admin, "oauth-audit-admin@test.local");

        let (client_id, _secret, _redirect) = seed_confidential_client(&pool).await;
        let id = client_uuid(&pool, &client_id).await;

        let before = count_audit_rows(&pool, admin, "oauth_client_update").await;

        let reason = "Granting calendar scope per ticket OPS-4821";
        let req = patch_request(
            &format!("/api/v1/admin/oauth/clients/{id}"),
            &token,
            serde_json::json!({
                "name": "CI Conf (renamed)",
                "scopes": ["profile", "email", "openid"],
                "reason": reason,
            }),
        );
        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "update_client must succeed for a granted platform principal (got {}): {}",
            resp.status,
            resp.text()
        );

        // Exactly one audit row was added for this action.
        let after = count_audit_rows(&pool, admin, "oauth_client_update").await;
        assert_eq!(
            after,
            before + 1,
            "exactly one oauth_client_update audit row must be written on update_client"
        );

        // The supplied reason is persisted into the audit row's details, and
        // the scope change is recorded.
        let row: (serde_json::Value, Option<Uuid>) = sqlx::query_as(
            r#"
            SELECT details, resource_id
            FROM audit_logs
            WHERE user_id = $1 AND action::text = 'oauth_client_update'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(admin)
        .fetch_one(&pool)
        .await
        .expect("fetch audit row");

        assert_eq!(
            row.0.get("reason").and_then(|v| v.as_str()),
            Some(reason),
            "audit details.reason must persist the operator reason, got {}",
            row.0
        );
        assert_eq!(
            row.0.get("scopes_changed").and_then(|v| v.as_bool()),
            Some(true),
            "audit must flag scopes_changed=true, got {}",
            row.0
        );
        assert_eq!(
            row.1,
            Some(id),
            "audit resource_id must reference the updated client UUID"
        );
    }
}

// ─── module: token_usage_analytics ───────────────────────────────────────────
//
// Epic 10A (data audit) follow-up #2628. PR #2526 landed the
// `oauth_token_events` data layer (model + repository + migration) but nothing
// on the running token path called it, so the table stayed permanently empty
// and the ecosystem-health dashboard rendered all-zeros. These tests pin the
// producer wired into `OAuthService`: every issuance / refresh / revocation on
// the real `/api/v1/oauth/*` router must persist a corresponding analytics row.
// On `dev` (no producer) each assertion below fails with an observed count of 0.

#[cfg(test)]
mod token_usage_analytics {
    use super::*;

    /// A full lifecycle — authorization_code issuance, refresh, then access-token
    /// revocation — must write exactly one `issued`, one `refreshed`, and one
    /// `revoked` row for the client, and the per-client rollup must reflect them.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_lifecycle_writes_analytics_events(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        // 1) authorization_code exchange → one `issued` event. Recording is
        // detached off the hot path (#2643), so poll for it rather than reading
        // once; a genuinely dropped write makes this time out.
        let (access_token, refresh_token) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;
        await_token_event_count(&pool, &client_id, "issued", 1).await;

        // 2) refresh_token grant → one `refreshed` event.
        let refresh_body = form_body(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let refresh_resp = app
            .execute(form_request("/api/v1/oauth/token", &refresh_body))
            .await;
        assert_eq!(
            refresh_resp.status,
            StatusCode::OK,
            "refresh must succeed. body={}",
            refresh_resp.text()
        );
        await_token_event_count(&pool, &client_id, "refreshed", 1).await;

        // 3) RFC 7009 revocation of the access token → one `revoked` event.
        let revoke_status = revoke_rfc7009(
            &app,
            &access_token,
            "access_token",
            &client_id,
            &client_secret,
        )
        .await;
        assert_eq!(
            revoke_status,
            StatusCode::OK,
            "revoke must succeed per RFC 7009"
        );
        await_token_event_count(&pool, &client_id, "revoked", 1).await;

        // All three detached writes have landed (we polled for each above), so
        // the per-client rollup the dashboard reads must now be non-empty and
        // consistent with the three lifecycle transitions above.
        let repo = db::repositories::OAuthTokenEventRepository::new(pool.clone());
        let since = chrono::Utc::now() - chrono::Duration::hours(1);
        let usage = repo
            .usage_per_client(since)
            .await
            .expect("usage_per_client");
        let row = usage
            .iter()
            .find(|u| u.client_id == client_id)
            .expect("client must appear in the per-client rollup");
        assert_eq!(row.issued_count, 1, "rollup issued_count");
        assert_eq!(row.refreshed_count, 1, "rollup refreshed_count");
        assert_eq!(row.revoked_count, 1, "rollup revoked_count");

        let totals = repo.totals(since).await.expect("totals");
        assert_eq!(totals.issued_count, 1);
        assert_eq!(totals.refreshed_count, 1);
        assert_eq!(totals.revoked_count, 1);
        assert_eq!(totals.active_clients, 1);
    }

    /// A best-effort recording failure must never break the OAuth flow: the
    /// issuance path still returns tokens even though we assert the analytics
    /// row is present on the happy path. This test focuses on the happy-path
    /// contract; the log-and-ignore behaviour is unit-covered by the service.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_public_client_issuance_records_event(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;

        let (verifier, challenge) = pkce_pair();
        let code = consent_and_get_code(
            &app,
            &user_at,
            &client_id,
            &redirect_uri,
            "profile",
            &challenge,
        )
        .await;
        let token_form = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("code_verifier", &verifier),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &token_form))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "public client token exchange must succeed. body={}",
            resp.text()
        );
        // Detached recording off the hot path (#2643): poll for the `issued`
        // event rather than reading once.
        await_token_event_count(&pool, &client_id, "issued", 1).await;
    }
}

// ─── module: token_usage_read_route ──────────────────────────────────────────
//
// Router-driven HTTP coverage for `GET /api/v1/platform-admin/oauth/token-usage`
// (issue #2630 — post-merge-review follow-up on PR #2629). The sibling
// `token_usage_analytics` module above only exercises the producer and the
// repository rollups (`OAuthTokenEventRepository::{totals,usage_per_client}`)
// called *directly*; nothing drove the read *handler* through the real Axum
// router, so the response envelope, its authz walls, and its `since` decoding
// shipped with zero executing HTTP-level coverage.
//
// These cases close that gap end-to-end against the real router:
//   * super-admin + `AuditRead` → 200, and the JSON body carries the
//     camelCase envelope (`totals` / `perClient` / `since`) after one seeded
//     issuance.
//   * the two authz walls each deny with 403: the `require_capability(AuditRead)`
//     tower layer, and the in-handler `extract_super_admin_token` super-admin
//     re-check.
//   * a malformed `?since=` → 400 `INVALID_SINCE`, and a valid RFC 3339
//     `?since=` narrows the window.
//
// IG3 note: NONE of these executed on `dev` before #2630. The deny-path (403)
// assertions, the 400 `INVALID_SINCE` decode assertion, and the camelCase
// envelope assertions are entirely net-new — the prior `token_usage_analytics`
// module never constructed an HTTP request to this route.
//
// The platform-admin provisioning recipe mirrors the shipped, passing pattern
// in `admin_platform_happy_path_tests.rs`: seed a `principal_kind = 'platform'`
// user (the INSERT bypasses the BEFORE-UPDATE guard), enroll MFA, and grant the
// capability with `mfa_required = false` so the extractor's recent-MFA recency
// check is skipped. The OAuth issuance is seeded through the real router via the
// file-level `confidential_auth_flow` helper.

#[cfg(test)]
mod token_usage_read_route {
    use super::*;
    use api_server::services::JwtService;

    const TOKEN_USAGE_URL: &str = "/api/v1/platform-admin/oauth/token-usage";
    /// Matches `TestConfig::default().jwt_secret`, so tokens minted here are
    /// accepted by the `TestApp`'s `JwtService`.
    const TEST_JWT_SECRET: &str =
        "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

    /// Seed a platform-principal user and return its id. The INSERT bypasses the
    /// `BEFORE UPDATE` principal_kind guard, so `'platform'` can be set directly.
    async fn seed_platform_user(pool: &PgPool, email: &str) -> Uuid {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
            VALUES ($1, 'h', 'Issue2630 Admin', 'active', NOW(), 'platform')
            RETURNING id
            "#,
        )
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("seed platform user")
    }

    /// Mark the user as MFA-enrolled (the capability gate's step-2.5 wall).
    async fn enroll_mfa(pool: &PgPool, user_id: Uuid) {
        sqlx::query(
            r#"
            INSERT INTO user_2fa (user_id, secret, enabled, enabled_at, backup_codes, backup_codes_remaining)
            VALUES ($1, 'unused-secret', true, NOW(), '[]'::jsonb, 0)
            ON CONFLICT (user_id) DO UPDATE SET enabled = true, enabled_at = NOW()
            "#,
        )
        .bind(user_id)
        .execute(pool)
        .await
        .expect("enroll mfa");
    }

    /// Grant a single named `capability` with `mfa_required = false`. `granted_by`
    /// must reference a distinct real user (FK + app-layer no-self-grant rule).
    async fn grant_capability(pool: &PgPool, user_id: Uuid, granted_by: Uuid, capability: &str) {
        sqlx::query(
            r#"
            INSERT INTO capability_grants
                (user_id, capability, granted_by, expires_at, mfa_required, note)
            VALUES ($1, $2, $3, NULL, false, 'issue-2630')
            "#,
        )
        .bind(user_id)
        .bind(capability)
        .bind(granted_by)
        .execute(pool)
        .await
        .expect("grant capability");
    }

    /// Mint a platform-kind bearer JWT validated by `TestApp`'s JWT secret, with
    /// the supplied `roles` claim (the wall under test in the deny cases).
    fn mint_token(user_id: Uuid, email: &str, roles: Option<Vec<String>>) -> String {
        let svc = JwtService::new(TEST_JWT_SECRET).expect("jwt service");
        svc.generate_access_token_with_kind(
            user_id,
            email,
            "Issue2630 Admin",
            None,
            roles,
            Some("platform".to_string()),
        )
        .expect("mint token")
    }

    /// Provision a fully-authorized platform super-admin holding `AuditRead`
    /// (platform principal + MFA + active grant + `super_admin` role) and return
    /// its bearer token — passes both the capability layer and
    /// `extract_super_admin_token`.
    async fn super_admin_with_audit_read(pool: &PgPool, label: &str) -> String {
        let email = format!("issue2630-sa-{label}-{}@test.local", Uuid::new_v4());
        let granter_email = format!("issue2630-gr-{label}-{}@test.local", Uuid::new_v4());
        let admin = seed_platform_user(pool, &email).await;
        let granter = seed_platform_user(pool, &granter_email).await;
        enroll_mfa(pool, admin).await;
        grant_capability(pool, admin, granter, "audit_read").await;
        mint_token(admin, &email, Some(vec!["super_admin".to_string()]))
    }

    /// Seed exactly one `issued` OAuth token event through the real router and
    /// return the client_id it was recorded against.
    async fn seed_one_issuance(app: &TestApp, pool: &PgPool) -> String {
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(pool).await;
        confidential_auth_flow(app, &user_at, &client_id, &client_secret, &redirect_uri).await;
        // Recording is detached off the hot path (#2643): wait for the seeded
        // issuance to land before the read-route assertions depend on it.
        await_token_event_count(pool, &client_id, "issued", 1).await;
        client_id
    }

    /// Case 1 — super-admin + `AuditRead` gets 200, and the JSON body carries the
    /// camelCase envelope (`totals` / `perClient` / `since`). The `#[serde(rename_all
    /// = "camelCase")]` on `OAuthTokenUsageResponse` renames `per_client` →
    /// `perClient`; assert both that the camelCase spelling is present and that the
    /// snake_case spelling is absent, so the contract is pinned in both directions.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_usage_super_admin_returns_camelcase_body(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let client_id = seed_one_issuance(&app, &pool).await;

        let token = super_admin_with_audit_read(&pool, "ok").await;
        let resp = app.get(TOKEN_USAGE_URL).bearer(&token).send().await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "super-admin with AuditRead must get 200. body={}",
            resp.text()
        );

        let body = resp.json_value();
        // camelCase envelope keys are present …
        assert!(
            body.get("totals").is_some(),
            "body must carry `totals`: {body}"
        );
        assert!(
            body.get("perClient").is_some(),
            "body must carry `perClient`: {body}"
        );
        assert!(
            body.get("since").is_some(),
            "body must carry `since`: {body}"
        );
        // … and the snake_case spelling is absent (locks the camelCase rename).
        assert!(
            body.get("per_client").is_none(),
            "`per_client` must be renamed to `perClient`: {body}"
        );
        assert!(
            body["since"].is_string(),
            "`since` must serialize as an RFC 3339 string: {body}"
        );

        // The seeded issuance is reflected in the totals + per-client rollup
        // (nested structs are camelCase too).
        assert_eq!(
            body["totals"]["issuedCount"], 1,
            "totals.issuedCount: {body}"
        );
        assert_eq!(
            body["totals"]["activeClients"], 1,
            "totals.activeClients: {body}"
        );
        assert!(
            body["totals"].get("issued_count").is_none(),
            "totals must be camelCase (no `issued_count`): {body}"
        );
        let per_client = body["perClient"].as_array().expect("perClient array");
        let row = per_client
            .iter()
            .find(|c| c["clientId"] == client_id)
            .expect("seeded client must appear in perClient");
        assert_eq!(row["issuedCount"], 1, "perClient issuedCount: {row}");
    }

    /// Case 2a — a platform principal holding `AuditRead` (so the capability layer
    /// passes) but whose token carries no `super_admin` role must be rejected with
    /// 403 by the in-handler `extract_super_admin_token` re-check.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_usage_denied_without_super_admin_role(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;

        let email = format!("issue2630-norole-{}@test.local", Uuid::new_v4());
        let granter_email = format!("issue2630-norole-gr-{}@test.local", Uuid::new_v4());
        let admin = seed_platform_user(&pool, &email).await;
        let granter = seed_platform_user(&pool, &granter_email).await;
        enroll_mfa(&pool, admin).await;
        grant_capability(&pool, admin, granter, "audit_read").await;
        // Token carries a non-privileged role only.
        let token = mint_token(admin, &email, Some(vec!["member".to_string()]));

        let resp = app.get(TOKEN_USAGE_URL).bearer(&token).send().await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "a non-super-admin caller must be rejected by extract_super_admin_token. body={}",
            resp.text()
        );
    }

    /// Case 2b — a platform `super_admin` with no `AuditRead` grant must be
    /// rejected with 403 by the `require_capability(AuditRead)` tower layer,
    /// before the handler runs.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_usage_denied_without_audit_read_capability(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;

        let email = format!("issue2630-nocap-{}@test.local", Uuid::new_v4());
        let admin = seed_platform_user(&pool, &email).await;
        enroll_mfa(&pool, admin).await;
        // super_admin role but NO capability grant.
        let token = mint_token(admin, &email, Some(vec!["super_admin".to_string()]));

        let resp = app.get(TOKEN_USAGE_URL).bearer(&token).send().await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "a caller missing the AuditRead capability must be rejected by the layer. body={}",
            resp.text()
        );
    }

    /// Case 3a — a malformed `?since=` must be rejected with 400 and the
    /// programmatic error code `INVALID_SINCE`.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_usage_invalid_since_returns_400(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let token = super_admin_with_audit_read(&pool, "badsince").await;

        let resp = app
            .get(&format!("{TOKEN_USAGE_URL}?since=not-a-timestamp"))
            .bearer(&token)
            .send()
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "a malformed `since` must be rejected with 400. body={}",
            resp.text()
        );
        let body = resp.json_value();
        assert_eq!(
            body["code"], "INVALID_SINCE",
            "error code must be INVALID_SINCE: {body}"
        );
    }

    /// Case 3b — a valid RFC 3339 `?since=` narrows the window: a bound in the
    /// past includes the seeded issuance (and the echoed `since` equals the
    /// supplied bound), while a bound in the future excludes it.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_usage_valid_since_narrows_window(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        seed_one_issuance(&app, &pool).await;
        let token = super_admin_with_audit_read(&pool, "since").await;

        // A `since` in the past includes the issuance.
        let past = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        let included = app
            .get(&format!(
                "{TOKEN_USAGE_URL}?since={}",
                urlencoding::encode(&past)
            ))
            .bearer(&token)
            .send()
            .await;
        assert_eq!(
            included.status,
            StatusCode::OK,
            "valid past `since` must be accepted. body={}",
            included.text()
        );
        let included_body = included.json_value();
        assert_eq!(
            included_body["totals"]["issuedCount"], 1,
            "a past `since` must include the seeded issuance: {included_body}"
        );
        // The echoed `since` reflects the caller-supplied bound, not the default.
        let echoed = included_body["since"].as_str().expect("since string");
        let echoed_dt = chrono::DateTime::parse_from_rfc3339(echoed).expect("echoed since parses");
        let supplied_dt =
            chrono::DateTime::parse_from_rfc3339(&past).expect("supplied since parses");
        assert_eq!(
            echoed_dt, supplied_dt,
            "the echoed `since` must equal the supplied bound"
        );

        // A `since` in the future excludes the issuance → window narrows to empty.
        let future = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let narrowed = app
            .get(&format!(
                "{TOKEN_USAGE_URL}?since={}",
                urlencoding::encode(&future)
            ))
            .bearer(&token)
            .send()
            .await;
        assert_eq!(
            narrowed.status,
            StatusCode::OK,
            "valid future `since` must be accepted. body={}",
            narrowed.text()
        );
        let narrowed_body = narrowed.json_value();
        assert_eq!(
            narrowed_body["totals"]["issuedCount"], 0,
            "a future `since` must narrow the window to empty: {narrowed_body}"
        );
        assert!(
            narrowed_body["perClient"]
                .as_array()
                .map(|a| a.is_empty())
                .unwrap_or(false),
            "a future `since` must yield an empty perClient: {narrowed_body}"
        );
    }
}
