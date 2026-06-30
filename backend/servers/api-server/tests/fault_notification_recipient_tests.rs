//! BIT-185 — Fault lifecycle notification recipient tests.
//!
//! Verifies that the correct recipients are resolved for each fault lifecycle
//! event.  Because `TestApp` mounts routes without `host_tenant_middleware`
//! (so `require_tenant_id` always short-circuits at 403 TENANT_REQUIRED),
//! these tests operate at the repository + pipeline layer — the same boundary
//! as `faults_authz_gap_tests.rs` and `notification_pipeline_dispatch_tests.rs`.
//!
//! | Event | Expected recipients |
//! |-------|---------------------|
//! | create | org managers (not the reporter) |
//! | assign | assignee + reporter (not the assigning manager) |
//! | status change | reporter only (not the manager) |
//! | resolve | reporter only (not the resolving manager) |
//! | reopen | org managers (not the reopener) |

#![allow(dead_code)]

use api_server::services::{EmailService, NotificationPipeline, PipelineConfig};
use common::notifications::{Notification, NotificationCategory};
use db::repositories::MembershipRepository;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', $2, 'active', NOW()) RETURNING id"#,
    )
    .bind(format!(
        "fault-notif-{tag}-{}@test.internal",
        Uuid::new_v4()
    ))
    .bind(format!("FaultNotif-{tag}"))
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_org(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ('FaultNotif Org', $1, $2, 'active') RETURNING id"#,
    )
    .bind(format!("fault-notif-org-{}", Uuid::new_v4()))
    .bind(format!("fault-notif-{}@test.internal", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

/// Seed into `user_memberships` — the table queried by `MembershipRepository`.
/// Note: `common::seed_membership` writes to `organization_members` (legacy),
/// which is NOT the table `list_manager_ids` / `is_manager_in_org` queries.
async fn seed_user_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid, role: &str) {
    sqlx::query(
        r#"INSERT INTO user_memberships (user_id, organization_id, role)
           VALUES ($1, $2, $3)
           ON CONFLICT DO NOTHING"#,
    )
    .bind(user_id)
    .bind(org_id)
    .bind(role)
    .execute(pool)
    .await
    .expect("seed user_membership");
}

fn build_pipeline(pool: &PgPool) -> NotificationPipeline {
    NotificationPipeline::new(
        pool.clone(),
        EmailService::development(),
        None,
        PipelineConfig::default(),
    )
}

/// Count `notification_groups` rows for `user_id` where entity_type = "faults".
async fn count_fault_notif_groups(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM notification_groups
           WHERE user_id = $1 AND entity_type = 'faults'"#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("count notification_groups")
}

// ---------------------------------------------------------------------------
// R1: list_manager_ids returns manager-tier members only
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_manager_ids_returns_only_managers(pool: PgPool) {
    let org = seed_org(&pool).await;
    let manager = seed_user(&pool, "mgr").await;
    let resident = seed_user(&pool, "res").await;

    seed_user_membership(&pool, org, manager, "Manager").await;
    seed_user_membership(&pool, org, resident, "Resident").await;

    let ids = MembershipRepository::new(pool.clone())
        .list_manager_ids(org)
        .await
        .expect("list_manager_ids");

    assert!(
        ids.contains(&manager),
        "list_manager_ids must include the Manager-role member"
    );
    assert!(
        !ids.contains(&resident),
        "list_manager_ids must NOT include a Resident-role member"
    );
}

// ---------------------------------------------------------------------------
// R2: create — managers notified; reporter excluded (self-notify guard)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_fault_notifies_managers_not_reporter(pool: PgPool) {
    let org = seed_org(&pool).await;
    let reporter = seed_user(&pool, "reporter-create").await;
    let manager = seed_user(&pool, "manager-create").await;

    seed_user_membership(&pool, org, manager, "Manager").await;

    let fault_id = Uuid::new_v4();
    let pipeline = build_pipeline(&pool);

    let manager_ids = MembershipRepository::new(pool.clone())
        .list_manager_ids(org)
        .await
        .expect("list_manager_ids");

    let recipients: Vec<Uuid> = manager_ids
        .into_iter()
        .filter(|id| *id != reporter)
        .collect();

    assert!(
        recipients.contains(&manager),
        "create: manager must be in recipient list"
    );
    assert!(
        !recipients.contains(&reporter),
        "create: reporter must be excluded (self-notify guard)"
    );

    let notification = Notification::new(
        Uuid::nil(),
        NotificationCategory::Faults,
        "New fault reported: Leaking pipe",
        "A new fault has been reported and requires triage.",
    )
    .with_action_url(format!("/faults/{fault_id}"))
    .with_data(serde_json::json!({"fault_id": fault_id, "organization_id": org}));

    pipeline
        .dispatch_to_users(&recipients, &notification, Some(fault_id), None)
        .await;

    assert_eq!(
        count_fault_notif_groups(&pool, manager).await,
        1,
        "create: manager must receive exactly one in-app fault notification"
    );
    assert_eq!(
        count_fault_notif_groups(&pool, reporter).await,
        0,
        "create: reporter must receive no notification (self-notify guard)"
    );
}

// ---------------------------------------------------------------------------
// R3: assign — assignee + reporter notified; assigning manager excluded
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn assign_fault_notifies_assignee_and_reporter_not_assigner(pool: PgPool) {
    let _org = seed_org(&pool).await;
    let reporter = seed_user(&pool, "reporter-assign").await;
    let assignee = seed_user(&pool, "assignee-assign").await;
    let assigner = seed_user(&pool, "assigner-assign").await;

    let fault_id = Uuid::new_v4();
    let pipeline = build_pipeline(&pool);

    // Mirror assign_fault recipient logic (assigner = principal.user_id)
    let assigned_to: Option<Uuid> = Some(assignee);
    let mut recipients: Vec<Uuid> = Vec::new();
    if assigned_to != Some(assigner) {
        if let Some(a) = assigned_to {
            recipients.push(a);
        }
    }
    if reporter != assigner && Some(reporter) != assigned_to {
        recipients.push(reporter);
    }

    let notification = Notification::new(
        Uuid::nil(),
        NotificationCategory::Faults,
        "Fault assigned: Leaking pipe",
        "The fault has been assigned and is now in progress.",
    )
    .with_action_url(format!("/faults/{fault_id}"))
    .with_data(serde_json::json!({"fault_id": fault_id}));

    pipeline
        .dispatch_to_users(&recipients, &notification, Some(fault_id), None)
        .await;

    assert_eq!(
        count_fault_notif_groups(&pool, assignee).await,
        1,
        "assign: assignee must receive notification"
    );
    assert_eq!(
        count_fault_notif_groups(&pool, reporter).await,
        1,
        "assign: reporter must receive notification"
    );
    assert_eq!(
        count_fault_notif_groups(&pool, assigner).await,
        0,
        "assign: assigning manager must not receive self-notification"
    );
}

// ---------------------------------------------------------------------------
// R4: status change — reporter notified; updating manager excluded
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn status_change_notifies_reporter_not_manager(pool: PgPool) {
    let _org = seed_org(&pool).await;
    let reporter = seed_user(&pool, "reporter-status").await;
    let manager = seed_user(&pool, "manager-status").await;

    let fault_id = Uuid::new_v4();
    let pipeline = build_pipeline(&pool);

    // Guard: reporter != manager (same check the handler applies)
    assert_ne!(reporter, manager, "test precondition: distinct users");

    let notification = Notification::new(
        Uuid::nil(),
        NotificationCategory::Faults,
        "Fault status updated: Leaking pipe",
        "Your fault report status has been updated to 'in_progress'.",
    )
    .with_action_url(format!("/faults/{fault_id}"))
    .with_data(serde_json::json!({"fault_id": fault_id, "status": "in_progress"}));

    // reporter != manager → dispatch to reporter only
    pipeline
        .dispatch(reporter, &notification, Some(fault_id), None)
        .await
        .expect("dispatch");

    assert_eq!(
        count_fault_notif_groups(&pool, reporter).await,
        1,
        "status change: reporter must receive notification"
    );
    assert_eq!(
        count_fault_notif_groups(&pool, manager).await,
        0,
        "status change: manager must not receive notification"
    );
}

// ---------------------------------------------------------------------------
// R5: resolve — reporter notified; resolving manager excluded
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resolve_fault_notifies_reporter_not_manager(pool: PgPool) {
    let _org = seed_org(&pool).await;
    let reporter = seed_user(&pool, "reporter-resolve").await;
    let manager = seed_user(&pool, "manager-resolve").await;

    let fault_id = Uuid::new_v4();
    let pipeline = build_pipeline(&pool);

    assert_ne!(reporter, manager, "test precondition: distinct users");

    let notification = Notification::new(
        Uuid::nil(),
        NotificationCategory::Faults,
        "Fault resolved: Leaking pipe",
        "Your fault report has been resolved. Please confirm if the issue was fixed.",
    )
    .with_action_url(format!("/faults/{fault_id}"))
    .with_data(serde_json::json!({"fault_id": fault_id}));

    pipeline
        .dispatch(reporter, &notification, Some(fault_id), None)
        .await
        .expect("dispatch");

    assert_eq!(
        count_fault_notif_groups(&pool, reporter).await,
        1,
        "resolve: reporter must receive notification"
    );
    assert_eq!(
        count_fault_notif_groups(&pool, manager).await,
        0,
        "resolve: resolving manager must not receive notification"
    );
}

// ---------------------------------------------------------------------------
// R6: reopen — org managers notified; reopener excluded
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reopen_fault_notifies_managers_not_reopener(pool: PgPool) {
    let org = seed_org(&pool).await;
    let reopener = seed_user(&pool, "reopener").await;
    let manager = seed_user(&pool, "manager-reopen").await;

    seed_user_membership(&pool, org, manager, "Manager").await;

    let fault_id = Uuid::new_v4();
    let pipeline = build_pipeline(&pool);

    let manager_ids = MembershipRepository::new(pool.clone())
        .list_manager_ids(org)
        .await
        .expect("list_manager_ids");

    let recipients: Vec<Uuid> = manager_ids
        .into_iter()
        .filter(|id| *id != reopener)
        .collect();

    assert!(
        recipients.contains(&manager),
        "reopen: manager must be in recipient list"
    );
    assert!(
        !recipients.contains(&reopener),
        "reopen: reopener must be excluded (self-notify guard)"
    );

    let notification = Notification::new(
        Uuid::nil(),
        NotificationCategory::Faults,
        "Fault reopened: Leaking pipe",
        "A resolved fault has been reopened and requires attention.",
    )
    .with_action_url(format!("/faults/{fault_id}"))
    .with_data(serde_json::json!({"fault_id": fault_id, "organization_id": org}));

    pipeline
        .dispatch_to_users(&recipients, &notification, Some(fault_id), None)
        .await;

    assert_eq!(
        count_fault_notif_groups(&pool, manager).await,
        1,
        "reopen: manager must receive in-app notification"
    );
    assert_eq!(
        count_fault_notif_groups(&pool, reopener).await,
        0,
        "reopen: reopener must not receive self-notification"
    );
}

// ---------------------------------------------------------------------------
// R7: triage — reporter + assignee notified; triaging manager excluded (#1793)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn triage_fault_notifies_reporter_and_assignee_not_manager(pool: PgPool) {
    let _org = seed_org(&pool).await;
    let reporter = seed_user(&pool, "reporter-triage").await;
    let assignee = seed_user(&pool, "assignee-triage").await;
    let manager = seed_user(&pool, "manager-triage").await;

    let fault_id = Uuid::new_v4();
    let pipeline = build_pipeline(&pool);

    // Mirror triage_fault recipient logic (manager = principal.user_id):
    // reporter + assignee (if triage set one), excluding the actor.
    let assigned_to: Option<Uuid> = Some(assignee);
    let mut recipients: Vec<Uuid> = Vec::new();
    if reporter != manager {
        recipients.push(reporter);
    }
    if let Some(a) = assigned_to {
        if a != manager && !recipients.contains(&a) {
            recipients.push(a);
        }
    }

    let notification = Notification::new(
        Uuid::nil(),
        NotificationCategory::Faults,
        "Fault triaged: Leaking pipe",
        "The fault has been triaged and prioritized.",
    )
    .with_action_url(format!("/faults/{fault_id}"))
    .with_data(serde_json::json!({"fault_id": fault_id}));

    pipeline
        .dispatch_to_users(&recipients, &notification, Some(fault_id), None)
        .await;

    assert_eq!(
        count_fault_notif_groups(&pool, reporter).await,
        1,
        "triage: reporter must receive notification"
    );
    assert_eq!(
        count_fault_notif_groups(&pool, assignee).await,
        1,
        "triage: assignee must receive notification"
    );
    assert_eq!(
        count_fault_notif_groups(&pool, manager).await,
        0,
        "triage: triaging manager must not receive self-notification"
    );
}

// ---------------------------------------------------------------------------
// R8: confirm — assignee + managers notified; confirming reporter excluded (#1793)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn confirm_fault_notifies_assignee_and_managers_not_reporter(pool: PgPool) {
    let org = seed_org(&pool).await;
    let reporter = seed_user(&pool, "reporter-confirm").await;
    let assignee = seed_user(&pool, "assignee-confirm").await;
    let manager = seed_user(&pool, "manager-confirm").await;

    seed_user_membership(&pool, org, manager, "Manager").await;

    let fault_id = Uuid::new_v4();
    let pipeline = build_pipeline(&pool);

    // Mirror confirm_fault recipient logic (reporter = principal.user_id):
    // assignee + org managers, excluding the actor.
    let assigned_to: Option<Uuid> = Some(assignee);
    let mut recipients: Vec<Uuid> = Vec::new();
    if let Some(a) = assigned_to {
        if a != reporter {
            recipients.push(a);
        }
    }
    let manager_ids = MembershipRepository::new(pool.clone())
        .list_manager_ids(org)
        .await
        .expect("list_manager_ids");
    for mid in manager_ids {
        if mid != reporter && !recipients.contains(&mid) {
            recipients.push(mid);
        }
    }

    assert!(
        recipients.contains(&assignee),
        "confirm: assignee must be a recipient"
    );
    assert!(
        recipients.contains(&manager),
        "confirm: org manager must be a recipient"
    );

    let notification = Notification::new(
        Uuid::nil(),
        NotificationCategory::Faults,
        "Fault resolution confirmed: Leaking pipe",
        "The reporter has confirmed the fault resolution.",
    )
    .with_action_url(format!("/faults/{fault_id}"))
    .with_data(serde_json::json!({"fault_id": fault_id, "organization_id": org}));

    pipeline
        .dispatch_to_users(&recipients, &notification, Some(fault_id), None)
        .await;

    assert_eq!(
        count_fault_notif_groups(&pool, assignee).await,
        1,
        "confirm: assignee must receive notification"
    );
    assert_eq!(
        count_fault_notif_groups(&pool, manager).await,
        1,
        "confirm: org manager must receive notification"
    );
    assert_eq!(
        count_fault_notif_groups(&pool, reporter).await,
        0,
        "confirm: confirming reporter must not receive self-notification"
    );
}
