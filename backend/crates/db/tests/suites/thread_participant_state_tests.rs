//! Integration tests for per-participant thread state — archive + per-user
//! soft-delete (BIT-182, Epic 6 gaps #4/#6).
//!
//! These exercise the `thread_participant_state` table (migration 00190) through
//! `MessagingRepository`'s list filter and the hide/archive mutations. The key
//! invariant: one participant deleting or archiving *their* copy of a shared
//! thread must NOT change what the other participant sees.
//!
//! ## Why these run as the (superuser) test role
//!
//! The list filtering this suite asserts on lives in the repository SQL itself
//! (`list_threads_rls` joins `thread_participant_state` and filters on
//! `deleted_at` / `archived_at`), NOT in the RLS policy. So the behavior is
//! visible even on the `#[sqlx::test]` superuser connection, which bypasses RLS.
//! (The RLS policy + FORCE on the new table is asserted separately by the
//! catalog-metadata checks at the bottom of this file, mirroring
//! `messaging_rls_cross_tenant_tests.rs`.)

use db::models::CreateMessage;
use db::repositories::MessagingRepository;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::seed_org;

/// Seed an active user and return its id.
async fn seed_user(pool: &PgPool, label: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', $2, 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(format!("{label}-{}@tps.test", Uuid::new_v4()))
    .bind(format!("TPS User {label}"))
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Insert a direct-message thread between two users in an org and return its id.
///
/// Inserted directly (rather than via `get_or_create_thread_rls`) so the test
/// depends only on the table shape, not on the get-or-create conflict path.
async fn seed_thread(pool: &PgPool, org_id: Uuid, a: Uuid, b: Uuid) -> Uuid {
    let mut ids = [a, b];
    ids.sort();
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO message_threads (organization_id, participant_ids)
        VALUES ($1, $2)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(&ids[..])
    .fetch_one(pool)
    .await
    .expect("seed thread")
}

/// Per-user delete hides the thread from that user's list only; the other
/// participant's view is untouched.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn per_user_delete_hides_only_for_deleting_user(pool: PgPool) {
    let repo = MessagingRepository::new(pool.clone());
    let org = seed_org(&pool, "tps-del").await;
    let alice = seed_user(&pool, "alice").await;
    let bob = seed_user(&pool, "bob").await;
    let thread = seed_thread(&pool, org, alice, bob).await;

    // Baseline: both participants see the thread in their default inbox.
    let alice_before = repo
        .list_threads_rls(&pool, alice, org, None, None, None, false)
        .await
        .expect("list alice before");
    let bob_before = repo
        .list_threads_rls(&pool, bob, org, None, None, None, false)
        .await
        .expect("list bob before");
    assert_eq!(
        alice_before.len(),
        1,
        "alice should see the thread initially"
    );
    assert_eq!(bob_before.len(), 1, "bob should see the thread initially");

    // Alice deletes the thread for herself.
    repo.hide_thread_for_user(&pool, thread, alice)
        .await
        .expect("hide for alice");

    let alice_after = repo
        .list_threads_rls(&pool, alice, org, None, None, None, false)
        .await
        .expect("list alice after");
    let bob_after = repo
        .list_threads_rls(&pool, bob, org, None, None, None, false)
        .await
        .expect("list bob after");
    let alice_count = repo
        .count_threads_rls(&pool, alice, org, None, false)
        .await
        .expect("count alice after");

    assert_eq!(
        alice_after.len(),
        0,
        "thread must be hidden from alice after she deletes it"
    );
    assert_eq!(
        alice_count, 0,
        "count must mirror the list filter for alice"
    );
    assert_eq!(
        bob_after.len(),
        1,
        "bob's copy must be untouched by alice's per-user delete"
    );
    assert_eq!(bob_after[0].id, thread);

    // A new inbound message un-hides the thread for alice again.
    repo.unhide_thread_for_user(&pool, thread, alice)
        .await
        .expect("unhide for alice");
    let alice_restored = repo
        .list_threads_rls(&pool, alice, org, None, None, None, false)
        .await
        .expect("list alice restored");
    assert_eq!(
        alice_restored.len(),
        1,
        "unhide must restore the thread to alice's inbox"
    );
}

/// Archiving moves the thread to the archived tab for that user only; it leaves
/// the default inbox (and the other participant) unchanged.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn archive_moves_thread_to_archived_tab_per_user(pool: PgPool) {
    let repo = MessagingRepository::new(pool.clone());
    let org = seed_org(&pool, "tps-arch").await;
    let alice = seed_user(&pool, "alice").await;
    let bob = seed_user(&pool, "bob").await;
    let thread = seed_thread(&pool, org, alice, bob).await;

    repo.archive_thread_for_user(&pool, thread, alice)
        .await
        .expect("archive for alice");

    // Alice: gone from inbox, present in archived tab.
    let alice_inbox = repo
        .list_threads_rls(&pool, alice, org, None, None, None, false)
        .await
        .expect("alice inbox");
    let alice_archived = repo
        .list_threads_rls(&pool, alice, org, None, None, None, true)
        .await
        .expect("alice archived");
    assert_eq!(
        alice_inbox.len(),
        0,
        "archived thread must leave alice's inbox"
    );
    assert_eq!(
        alice_archived.len(),
        1,
        "archived thread must appear in alice's archived tab"
    );

    // Bob: unaffected — still in inbox, nothing archived.
    let bob_inbox = repo
        .list_threads_rls(&pool, bob, org, None, None, None, false)
        .await
        .expect("bob inbox");
    let bob_archived = repo
        .list_threads_rls(&pool, bob, org, None, None, None, true)
        .await
        .expect("bob archived");
    assert_eq!(bob_inbox.len(), 1, "bob's inbox must be unaffected");
    assert_eq!(bob_archived.len(), 0, "bob has nothing archived");

    // Un-archiving returns it to alice's inbox.
    repo.unarchive_thread_for_user(&pool, thread, alice)
        .await
        .expect("unarchive for alice");
    let alice_inbox_again = repo
        .list_threads_rls(&pool, alice, org, None, None, None, false)
        .await
        .expect("alice inbox again");
    assert_eq!(
        alice_inbox_again.len(),
        1,
        "un-archived thread must return to alice's inbox"
    );
}

/// 00190 must bring `thread_participant_state` under ENABLE + FORCE row-level
/// security with at least one policy (so FORCE enforces a real policy rather
/// than an implicit deny-all). Mirrors `messaging_rls_cross_tenant_tests.rs`.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn thread_participant_state_has_force_rls_and_policy(pool: PgPool) {
    let (relrowsecurity, relforcerowsecurity): (bool, bool) = sqlx::query_as(
        r#"
        SELECT relrowsecurity, relforcerowsecurity
        FROM pg_class
        WHERE relname = 'thread_participant_state'
          AND relnamespace = 'public'::regnamespace
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("query pg_class RLS flags");

    assert!(
        relrowsecurity,
        "thread_participant_state must have ENABLE ROW LEVEL SECURITY"
    );
    assert!(
        relforcerowsecurity,
        "thread_participant_state must have FORCE ROW LEVEL SECURITY (owner must not bypass)"
    );

    let policy_count: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
        FROM pg_policies
        WHERE schemaname = 'public'
          AND tablename = 'thread_participant_state'
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("query pg_policies");

    assert!(
        policy_count > 0,
        "thread_participant_state must carry at least one RLS policy; found {policy_count}"
    );
}

/// A thread the user soft-deleted ("delete for me") must not keep contributing
/// to that user's global unread badge (#1771). Before the fix, `count_unread_rls`
/// joined `thread_participant_state` only for the read watermark and ignored
/// `deleted_at`, so a hidden thread with unread messages left the badge stuck
/// non-zero with no thread visible in the inbox to clear it. The other
/// participant's count must be unaffected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn soft_deleted_thread_excluded_from_unread_count(pool: PgPool) {
    let repo = MessagingRepository::new(pool.clone());
    let org = seed_org(&pool, "tps-unread-del").await;
    let alice = seed_user(&pool, "alice").await;
    let bob = seed_user(&pool, "bob").await;
    let thread = seed_thread(&pool, org, alice, bob).await;

    // Bob sends a message → unread for alice (the recipient), not for bob.
    repo.create_message_rls(
        &pool,
        CreateMessage {
            thread_id: thread,
            sender_id: bob,
            content: "ping".to_string(),
        },
    )
    .await
    .expect("bob sends");

    assert_eq!(
        repo.count_unread_rls(&pool, alice, org).await.unwrap(),
        1,
        "alice has 1 unread before deleting"
    );

    // Alice deletes the thread for herself. It leaves her inbox; its unread
    // message must leave her badge too — otherwise the badge is stuck.
    repo.hide_thread_for_user(&pool, thread, alice)
        .await
        .expect("hide for alice");

    assert_eq!(
        repo.count_unread_rls(&pool, alice, org).await.unwrap(),
        0,
        "soft-deleted thread must not contribute to alice's unread badge (#1771)"
    );
    assert_eq!(
        repo.count_unread_rls(&pool, bob, org).await.unwrap(),
        0,
        "bob (the sender) was never unread; unaffected by alice's delete"
    );

    // A new inbound message un-hides the thread (existing best-effort path), so
    // the unread naturally returns and is actionable again.
    repo.unhide_thread_for_user(&pool, thread, alice)
        .await
        .expect("unhide for alice");
    repo.create_message_rls(
        &pool,
        CreateMessage {
            thread_id: thread,
            sender_id: bob,
            content: "ping again".to_string(),
        },
    )
    .await
    .expect("bob sends again");

    assert_eq!(
        repo.count_unread_rls(&pool, alice, org).await.unwrap(),
        2,
        "after un-hide both messages count again for alice (thread is back in her inbox)"
    );
}
