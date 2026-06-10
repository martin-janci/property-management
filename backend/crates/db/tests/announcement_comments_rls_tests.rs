//! RLS tenant-isolation tests for `announcement_comments` (Epic 6, Story 6.3).
//!
//! Coverage 6-3: proves the `announcement_comments_tenant_isolation` policy
//! (migration 00016) blocks cross-tenant access to threaded comments. The
//! policy inherits isolation from the parent `announcements` row:
//!
//! ```sql
//! USING ( is_super_admin()
//!         OR EXISTS (SELECT 1 FROM announcements a
//!                    WHERE a.id = announcement_comments.announcement_id
//!                      AND a.organization_id = get_current_org_id()) )
//! ```
//!
//! Why this lives at the DB layer (and is `#[ignore]`d): Postgres exempts
//! SUPERUSERS from RLS entirely, so the policy is only observable when the
//! connection authenticates as a *non-superuser* role. The `#[sqlx::test]`
//! HTTP harness in `servers/api-server/tests/` connects as the `postgres`
//! superuser, where the comment routes' `RlsConnection` GUC is a silent no-op.
//! These tests therefore connect via `TEST_DATABASE_URL` (the non-superuser
//! `rls_test_runner` role created by the backend CI workflow), exactly like
//! `rls_penetration_tests.rs`. They are `#[ignore]`d so the default
//! `cargo test` run — which has no such role — skips them; CI runs them with:
//!
//! ```bash
//! TEST_DATABASE_URL=postgres://rls_test_runner:rls_test_runner@localhost:5432/ppt_test \
//!   cargo test -p db --test announcement_comments_rls_tests -- --ignored --test-threads=1
//! ```
//!
//! Additive: new file, no existing test touched.

use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;
use uuid::Uuid;

/// Single-connection pool so the per-session RLS GUC set by
/// `set_request_context` persists across every query in a test.
async fn test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ppt_test".to_string());

    PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await
        .expect("connect to TEST_DATABASE_URL")
}

async fn set_context(pool: &PgPool, org_id: Option<Uuid>, user_id: Option<Uuid>, super_admin: bool) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(org_id)
        .bind(user_id)
        .bind(super_admin)
        .execute(pool)
        .await
        .expect("set_request_context");
}

async fn clear_context(pool: &PgPool) {
    sqlx::query("SELECT clear_request_context()")
        .execute(pool)
        .await
        .expect("clear_request_context");
}

async fn create_org(pool: &PgPool, label: &str) -> Uuid {
    let slug = format!("{label}-{}", Uuid::new_v4());
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("AnnComment RLS {label}"))
    .bind(&slug)
    .bind(format!("{slug}@test.com"))
    .fetch_one(pool)
    .await
    .expect("create org")
}

async fn create_user(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'AnnComment RLS User', 'active', NOW()) RETURNING id"#,
    )
    .bind(format!("anncomment-rls-{}@test.com", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("create user")
}

async fn create_announcement(pool: &PgPool, org_id: Uuid, author_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO announcements
               (organization_id, author_id, title, content, target_type, status, published_at)
           VALUES ($1, $2, 'RLS', 'body', 'all'::announcement_target_type,
                   'published'::announcement_status, NOW())
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(author_id)
    .fetch_one(pool)
    .await
    .expect("create announcement")
}

async fn create_comment(pool: &PgPool, announcement_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO announcement_comments (announcement_id, user_id, content)
           VALUES ($1, $2, 'org-scoped comment') RETURNING id"#,
    )
    .bind(announcement_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("create comment")
}

/// Delete any rows this suite created. Runs under super-admin context to bypass
/// RLS. Scoped to the `@test.com` user domain + the orgs we own via their users.
async fn cleanup(pool: &PgPool) {
    set_context(pool, None, None, true).await;
    // Comments / announcements cascade from organizations + users.
    sqlx::query("DELETE FROM organizations WHERE contact_email LIKE '%@test.com'")
        .execute(pool)
        .await
        .expect("cleanup orgs");
    sqlx::query("DELETE FROM users WHERE email LIKE '%@test.com'")
        .execute(pool)
        .await
        .expect("cleanup users");
    clear_context(pool).await;
}

// ---------------------------------------------------------------------------
// (1) SELECT isolation: org B cannot read org A's comments; org A can read its
//     own (policy does not over-filter).
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires non-superuser TEST_DATABASE_URL (rls_test_runner); run with --ignored"]
async fn comment_select_is_tenant_isolated() {
    let pool = test_pool().await;
    cleanup(&pool).await;

    // Seed under super-admin (bypasses RLS for setup).
    set_context(&pool, None, None, true).await;
    let org_a = create_org(&pool, "select-a").await;
    let org_b = create_org(&pool, "select-b").await;
    let author_a = create_user(&pool).await;
    let ann_a = create_announcement(&pool, org_a, author_a).await;
    let comment_a = create_comment(&pool, ann_a, author_a).await;

    // Org A context: sees its own comment.
    set_context(&pool, Some(org_a), Some(author_a), false).await;
    let visible: Vec<Uuid> = sqlx::query("SELECT id FROM announcement_comments")
        .fetch_all(&pool)
        .await
        .expect("select as org A")
        .into_iter()
        .map(|r| r.get::<Uuid, _>("id"))
        .collect();
    assert!(
        visible.contains(&comment_a),
        "org A must see its own comment under RLS"
    );

    // Org B context: the comment is invisible.
    set_context(&pool, Some(org_b), Some(author_a), false).await;
    let leaked: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM announcement_comments WHERE id = $1")
            .bind(comment_a)
            .fetch_optional(&pool)
            .await
            .expect("select as org B");
    assert!(
        leaked.is_none(),
        "org B must NOT be able to read org A's comment (RLS leak)"
    );

    clear_context(&pool).await;
    cleanup(&pool).await;
}

// ---------------------------------------------------------------------------
// (2) INSERT isolation (WITH CHECK): org B cannot create a comment on org A's
//     announcement.
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires non-superuser TEST_DATABASE_URL (rls_test_runner); run with --ignored"]
async fn comment_insert_is_tenant_isolated() {
    let pool = test_pool().await;
    cleanup(&pool).await;

    set_context(&pool, None, None, true).await;
    let org_a = create_org(&pool, "insert-a").await;
    let org_b = create_org(&pool, "insert-b").await;
    let author_a = create_user(&pool).await;
    let user_b = create_user(&pool).await;
    let ann_a = create_announcement(&pool, org_a, author_a).await;

    // Org B tries to insert a comment onto org A's announcement → blocked by the
    // policy's WITH CHECK clause.
    set_context(&pool, Some(org_b), Some(user_b), false).await;
    let result = sqlx::query(
        r#"INSERT INTO announcement_comments (announcement_id, user_id, content)
           VALUES ($1, $2, 'intruder')"#,
    )
    .bind(ann_a)
    .bind(user_b)
    .execute(&pool)
    .await;
    assert!(
        result.is_err(),
        "org B must NOT be able to insert a comment on org A's announcement (WITH CHECK)"
    );

    clear_context(&pool).await;
    cleanup(&pool).await;
}

// ---------------------------------------------------------------------------
// (3) UPDATE/soft-delete isolation: org B cannot soft-delete org A's comment.
//     (The handler's delete path is an UPDATE setting deleted_at.)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires non-superuser TEST_DATABASE_URL (rls_test_runner); run with --ignored"]
async fn comment_soft_delete_is_tenant_isolated() {
    let pool = test_pool().await;
    cleanup(&pool).await;

    set_context(&pool, None, None, true).await;
    let org_a = create_org(&pool, "delete-a").await;
    let org_b = create_org(&pool, "delete-b").await;
    let author_a = create_user(&pool).await;
    let user_b = create_user(&pool).await;
    let ann_a = create_announcement(&pool, org_a, author_a).await;
    let comment_a = create_comment(&pool, ann_a, author_a).await;

    // Org B attempts the soft-delete UPDATE. RLS makes the row invisible, so the
    // UPDATE matches zero rows (no error, but nothing changes).
    set_context(&pool, Some(org_b), Some(user_b), false).await;
    let affected = sqlx::query(
        r#"UPDATE announcement_comments
           SET deleted_at = NOW(), deleted_by = $2
           WHERE id = $1 AND deleted_at IS NULL"#,
    )
    .bind(comment_a)
    .bind(user_b)
    .execute(&pool)
    .await
    .expect("update runs")
    .rows_affected();
    assert_eq!(
        affected, 0,
        "org B's soft-delete must affect zero rows (org A's comment is invisible under RLS)"
    );

    // Confirm org A's comment is still live.
    set_context(&pool, Some(org_a), Some(author_a), false).await;
    let still_live: Option<bool> = sqlx::query_scalar(
        "SELECT deleted_at IS NULL FROM announcement_comments WHERE id = $1",
    )
    .bind(comment_a)
    .fetch_optional(&pool)
    .await
    .expect("select as org A");
    assert_eq!(
        still_live,
        Some(true),
        "org A's comment must remain undeleted after org B's blocked attempt"
    );

    clear_context(&pool).await;
    cleanup(&pool).await;
}
