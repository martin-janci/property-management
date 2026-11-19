//! Regression: the vote `update` write must be tenant-scoped on its own, not
//! merely gated by a preceding org-scoped read (code-review finding
//! `code-review-api-handlers-vote-update-no-tenant-rls`).
//!
//! `update_vote` (routes/voting.rs) 404s cross-tenant callers at the
//! `find_poll_by_id_for_org` read gate, so an end-to-end HTTP test can't tell
//! whether the WRITE itself is scoped — the read catches it first. These tests
//! therefore drive the repository write directly (bypassing that read gate) to
//! prove the UPDATE folds the owning org into its `WHERE` clause: org A may not
//! mutate org B's draft vote, and the attempt leaves org B's row untouched.

use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use db::models::{CreateVote, UpdateVote};
use db::repositories::VoteRepository;

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("Vote Update Org {slug}"))
    .bind(format!("vote-upd-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@vote-upd.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Vote Update User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(format!("{slug}-{}@vote-upd.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .fetch_one(pool)
    .await
    .expect("seed building")
}

/// Create a draft vote owned by `org_id` and return its id.
async fn seed_draft_vote(repo: &VoteRepository, pool: &PgPool, org_id: Uuid, title: &str) -> Uuid {
    let building_id = seed_building(pool, org_id, "b").await;
    let created_by = seed_user(pool, "creator").await;
    let vote = repo
        .create_poll_rls(
            pool,
            CreateVote {
                organization_id: org_id,
                building_id,
                title: title.to_string(),
                description: None,
                start_at: None,
                end_at: Utc::now() + Duration::days(7),
                quorum_type: "simple_majority".to_string(),
                quorum_percentage: None,
                allow_delegation: None,
                anonymous_voting: None,
                created_by,
            },
        )
        .await
        .expect("create draft vote");
    assert_eq!(vote.status, "draft", "seeded vote should be draft");
    vote.id
}

fn update_title(title: &str) -> UpdateVote {
    UpdateVote {
        title: Some(title.to_string()),
        description: None,
        start_at: None,
        end_at: None,
        quorum_type: None,
        quorum_percentage: None,
        allow_delegation: None,
        anonymous_voting: None,
    }
}

/// Org A attempts to update a draft vote owned by org B via the repository
/// write directly (the HTTP read gate is intentionally bypassed). The write
/// must match zero rows (`None`) and leave org B's row unchanged.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_for_org_rejects_cross_tenant_write(pool: PgPool) {
    let repo = VoteRepository::new(pool.clone());

    let org_a = seed_org(&pool, "a").await;
    let org_b = seed_org(&pool, "b").await;
    let original_title = "Org B roof budget";
    let vote_b = seed_draft_vote(&repo, &pool, org_b, original_title).await;

    // Org A's tenant context tries to rename org B's draft vote.
    let result = repo
        .update_for_org(vote_b, org_a, update_title("HIJACKED by org A"), &pool)
        .await
        .expect("update_for_org query should execute");

    assert!(
        result.is_none(),
        "cross-tenant update must match zero rows (got Some)"
    );

    // Org B's row is untouched.
    let title_now: String = sqlx::query_scalar("SELECT title FROM votes WHERE id = $1")
        .bind(vote_b)
        .fetch_one(&pool)
        .await
        .expect("fetch org B vote title");
    assert_eq!(
        title_now, original_title,
        "org B's draft vote must be unchanged by the cross-tenant write"
    );
}

/// Control: the owning org CAN update its own draft vote through the same
/// method, so the tenant filter rejects only foreign writes.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_for_org_allows_owning_tenant_write(pool: PgPool) {
    let repo = VoteRepository::new(pool.clone());

    let org = seed_org(&pool, "owner").await;
    let vote_id = seed_draft_vote(&repo, &pool, org, "Original title").await;

    let updated = repo
        .update_for_org(vote_id, org, update_title("Revised title"), &pool)
        .await
        .expect("update_for_org query should execute")
        .expect("owning-tenant update must match the row");

    assert_eq!(updated.title, "Revised title");
}
