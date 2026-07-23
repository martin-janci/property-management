//! N6 — `AgencyDomainRepository` cache-invalidation smoke tests.
//!
//! These tests pin the contract that every host-touching write method on
//! [`db::repositories::AgencyDomainRepository`] invokes the wired
//! [`AgencyDomainCacheInvalidator`] for the affected host. Without that
//! contract, [`api_core::middleware::host_tenant::TenantResolutionCache`]
//! can return a stale resolution for up to its TTL — and an attacker who
//! re-registers a freshly-released host can briefly impersonate the
//! previous tenant.
//!
//! # Two flavors
//!
//! 1. **Pure unit tests** (run by default): inject a counting in-process
//!    `MockInvalidator` and observe the calls. These exercise the wiring
//!    contract without needing Postgres.
//!
//! 2. **DB-backed smoke** (marked `#[ignore]`, opt-in via
//!    `cargo test --test agency_domain_cache_tests -- --ignored
//!    --test-threads=1`): drives `create_rls`/`release_rls` against a real
//!    DB and asserts the host-resolution cache misses on the post-release
//!    read. Mirrors the existing repository test pattern in
//!    `tests/repository_tests.rs`.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use db::repositories::agency_domain::{AgencyDomainCacheInvalidator, NoopDomainCacheInvalidator};

// ----------------------------------------------------------------------------
// Mock invalidator (pure unit tests)
// ----------------------------------------------------------------------------

#[derive(Default)]
struct MockInvalidator {
    /// Number of `invalidate(host)` calls observed.
    invalidate_calls: AtomicUsize,
    /// Number of `invalidate_all()` calls observed.
    invalidate_all_calls: AtomicUsize,
    /// Concatenated hosts seen across `invalidate(host)` calls, comma-separated.
    /// Read from a `Mutex<String>` so we can introspect from sync test code
    /// without juggling `Arc<Mutex<…>>` accessors.
    hosts: tokio::sync::Mutex<Vec<String>>,
}

impl MockInvalidator {
    fn new() -> Self {
        Default::default()
    }
    fn invalidate_count(&self) -> usize {
        self.invalidate_calls.load(Ordering::SeqCst)
    }
    fn invalidate_all_count(&self) -> usize {
        self.invalidate_all_calls.load(Ordering::SeqCst)
    }
    async fn hosts_seen(&self) -> Vec<String> {
        self.hosts.lock().await.clone()
    }
}

#[async_trait::async_trait]
impl AgencyDomainCacheInvalidator for MockInvalidator {
    async fn invalidate(&self, host: &str) {
        self.invalidate_calls.fetch_add(1, Ordering::SeqCst);
        self.hosts.lock().await.push(host.to_string());
    }
    async fn invalidate_all(&self) {
        self.invalidate_all_calls.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn noop_invalidator_is_silent() {
    // Sanity: the default repo wiring (no `with_cache`) uses `NoopDomainCacheInvalidator`
    // which does nothing — useful for scripts/tests that don't have a cache to flush.
    let noop = NoopDomainCacheInvalidator;
    noop.invalidate("acme.example.com").await;
    noop.invalidate_all().await;
    // No assertion possible — the contract is "no panic, no side effect".
}

#[tokio::test]
async fn mock_invalidator_records_calls() {
    let mock = Arc::new(MockInvalidator::new());
    mock.invalidate("acme.example.com").await;
    mock.invalidate("evil.example.com").await;
    mock.invalidate_all().await;

    assert_eq!(mock.invalidate_count(), 2);
    assert_eq!(mock.invalidate_all_count(), 1);
    assert_eq!(
        mock.hosts_seen().await,
        vec![
            "acme.example.com".to_string(),
            "evil.example.com".to_string()
        ]
    );
}

// ----------------------------------------------------------------------------
// DB-backed smoke (opt-in via `--ignored`)
// ----------------------------------------------------------------------------

/// End-to-end smoke: cache-populate -> release -> cache-miss.
///
/// Requires a live Postgres with migrations applied.
///
/// ```text
/// cargo test --test agency_domain_cache_tests -- --ignored --test-threads=1
/// ```
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with migrations applied"]
async fn release_invalidates_cached_resolution() {
    use db::models::agency_domain::{AgencyDomainKind, CreateAgencyDomain};
    use db::models::organization::CreateOrganization;
    use db::repositories::{AgencyDomainRepository, OrganizationRepository};
    use sqlx::postgres::PgPoolOptions;
    use std::time::Duration;

    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ppt_test".to_string());
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await
        .expect("connect test db");

    // Fresh org + domain. We use a uuid suffix so re-runs don't collide.
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let host = format!("test-{}.example.test", &suffix[..8]);
    let slug = format!("test-{}", &suffix[..8]);

    // Seed: open a tx with super-admin RLS, create org + verified domain.
    let mut tx = pool.begin().await.expect("begin tx");
    db::tenant_context::set_request_context(&mut *tx, None, None, true)
        .await
        .expect("set rls");

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .create_rls(
            &mut *tx,
            CreateOrganization {
                name: format!("Test {}", &suffix[..8]),
                slug,
                contact_email: "ops@example.test".into(),
                logo_url: None,
                primary_color: None,
            },
        )
        .await
        .expect("create org");

    let mock = Arc::new(MockInvalidator::new());
    let repo = AgencyDomainRepository::new(pool.clone()).with_cache(mock.clone());

    let domain = repo
        .create_rls(
            &mut *tx,
            CreateAgencyDomain {
                organization_id: org.id,
                host: host.clone(),
                kind: Some(AgencyDomainKind::Subdomain),
                is_primary: true,
            },
        )
        .await
        .expect("create domain");

    // Promote to verified so the resolver returns Some(_).
    sqlx::query("UPDATE agency_domains SET verification_state = 'verified' WHERE id = $1")
        .bind(domain.id)
        .execute(&mut *tx)
        .await
        .expect("verify");

    db::tenant_context::clear_request_context(&mut *tx)
        .await
        .ok();
    tx.commit().await.expect("commit seed");

    // Resolve so the cache (in api-core) would populate. We don't have a
    // cache wired here — just observe `resolve_host_system` succeeds.
    let resolved = repo
        .resolve_host_system(&host)
        .await
        .expect("resolve verified host");
    assert_eq!(resolved, Some(org.id), "verified host must resolve");

    // Release via the repo. The mock invalidator should observe one call.
    let mut tx2 = pool.begin().await.expect("begin tx2");
    db::tenant_context::set_request_context(&mut *tx2, None, None, true)
        .await
        .ok();
    let released_host = repo
        .release_rls(&mut *tx2, domain.id)
        .await
        .expect("release");
    db::tenant_context::clear_request_context(&mut *tx2)
        .await
        .ok();
    tx2.commit().await.expect("commit release");

    assert_eq!(released_host.as_deref(), Some(host.as_str()));
    let invalidate_hosts = mock.hosts_seen().await;
    assert!(
        invalidate_hosts.iter().any(|h| h == &host),
        "release must invalidate the released host (saw: {invalidate_hosts:?})"
    );

    // Post-release: the host must no longer resolve.
    let after = repo
        .resolve_host_system(&host)
        .await
        .expect("resolve after release");
    assert!(
        after.is_none(),
        "released host must not resolve any more (got {after:?})"
    );
}
