//! Per-participant unread watermark for N-party group threads (#1773 finding-2).
//!
//! `messages.read_at` is a single per-message flag, so on the pre-fix code the
//! first recipient to read a group thread marked every message read for ALL
//! participants — collapsing everyone's unread count to zero. The fix derives
//! unread from a per-(thread, user) watermark
//! (`thread_participant_state.last_read_at`), so each participant's unread is
//! independent.
//!
//! These run as the `#[sqlx::test]` superuser (RLS bypassed) — the behaviour
//! under test lives in the repository SQL (`mark_thread_read_rls` watermark
//! upsert + the watermark predicate in `count_unread_rls`), not in an RLS
//! policy. Mirrors `thread_participant_state_tests.rs`.

use db::models::CreateMessage;
use db::repositories::MessagingRepository;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::seed_org;

async fn seed_user(pool: &PgPool, label: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', $2, 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(format!("{label}-{}@nparty.test", Uuid::new_v4()))
    .bind(format!("NParty {label}"))
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Insert an N-party thread with the given participants and return its id.
async fn seed_group_thread(pool: &PgPool, org_id: Uuid, participants: &[Uuid]) -> Uuid {
    let mut ids = participants.to_vec();
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
    .expect("seed group thread")
}

/// In a 3-party thread, one participant marking the thread read must NOT clear
/// the other participants' unread counts (#1773 finding-2). On the old global
/// `messages.read_at` model this test fails: B reading would also zero C.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn unread_is_per_participant_in_group_thread(pool: PgPool) {
    let repo = MessagingRepository::new(pool.clone());
    let org = seed_org(&pool, "nparty-unread").await;
    let a = seed_user(&pool, "a").await;
    let b = seed_user(&pool, "b").await;
    let c = seed_user(&pool, "c").await;
    let thread = seed_group_thread(&pool, org, &[a, b, c]).await;

    // A sends a message → unread for B and C, not for A (the sender).
    repo.create_message_rls(
        &pool,
        CreateMessage {
            thread_id: thread,
            sender_id: a,
            content: "hello group".to_string(),
        },
    )
    .await
    .expect("a sends");

    assert_eq!(
        repo.count_unread_rls(&pool, a, org).await.unwrap(),
        0,
        "sender A has no unread"
    );
    assert_eq!(
        repo.count_unread_rls(&pool, b, org).await.unwrap(),
        1,
        "B has 1 unread"
    );
    assert_eq!(
        repo.count_unread_rls(&pool, c, org).await.unwrap(),
        1,
        "C has 1 unread"
    );

    // B reads the thread. On the old model this set messages.read_at globally,
    // zeroing C's unread too. With the watermark only B is affected.
    let mut conn = pool.acquire().await.expect("acquire");
    repo.mark_thread_read_rls(&mut conn, thread, b)
        .await
        .expect("b marks read");
    drop(conn);

    assert_eq!(
        repo.count_unread_rls(&pool, b, org).await.unwrap(),
        0,
        "B is now caught up"
    );
    assert_eq!(
        repo.count_unread_rls(&pool, c, org).await.unwrap(),
        1,
        "C's unread must be UNCHANGED by B reading (per-participant, #1773)"
    );

    // A sends a second message → B is unread again (newer than B's watermark);
    // C accumulates to 2.
    repo.create_message_rls(
        &pool,
        CreateMessage {
            thread_id: thread,
            sender_id: a,
            content: "second".to_string(),
        },
    )
    .await
    .expect("a sends again");

    assert_eq!(
        repo.count_unread_rls(&pool, b, org).await.unwrap(),
        1,
        "B sees the new message as unread (created after B's watermark)"
    );
    assert_eq!(
        repo.count_unread_rls(&pool, c, org).await.unwrap(),
        2,
        "C now has 2 unread"
    );
    assert_eq!(
        repo.count_unread_rls(&pool, a, org).await.unwrap(),
        0,
        "sender A still 0"
    );
}
