//! Repo-level regression for #3308 — the vote sub-resource reads must fold the
//! owning vote's `organization_id` into the query itself, not lean solely on a
//! handler-side `vote_in_org` pre-check.
//!
//! Before #3308, `get_questions` / `get_user_response` / `list_comments` /
//! `list_replies` / `check_eligibility` ran on the shared pool with no
//! org/RLS predicate: tenant isolation rested entirely on each handler first
//! calling `vote_in_org`. A handler that forgot that pre-check leaked another
//! org's data. The `*_for_org` variants fold `organization_id` (join to
//! `votes`) — and, for replies, the parent's `vote_id` — into the SQL, so a
//! wrong-org lookup returns nothing / `RowNotFound` even with the pre-check
//! absent.
//!
//! This test exercises the repository methods directly on the pool (the
//! `#[sqlx::test]` superuser bypasses RLS), which isolates the *SQL org
//! predicate* — exactly the defense-in-depth the issue asks for: passing org
//! B's id for an org-A vote must resolve to empty.

use db::models::{CreateVote, CreateVoteComment, CreateVoteQuestion, QuestionOption};
use db::repositories::VoteRepository;
use sqlx::PgPool;
use uuid::Uuid;

struct OrgFixture {
    org: Uuid,
    owner: Uuid,
    unit: Uuid,
    vote: Uuid,
    top_comment: Uuid,
}

async fn super_admin_ctx(pool: &PgPool) {
    sqlx::query("SELECT set_request_context(NULL, NULL, TRUE)")
        .execute(pool)
        .await
        .expect("set_request_context super-admin");
}

/// Seed one org with a building, an owner+unit+response, a vote carrying a
/// question, a top-level comment and one reply to it.
async fn seed_org_fixture(pool: &PgPool, repo: &VoteRepository, slug: &str) -> OrgFixture {
    let org: Uuid = sqlx::query_scalar(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ($1, $1, $2, 'active') RETURNING id",
    )
    .bind(slug)
    .bind(format!("{slug}@vote3308.test"))
    .fetch_one(pool)
    .await
    .expect("seed org");

    let owner: Uuid = sqlx::query_scalar(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at) \
         VALUES ($1, 'h', 'Owner', 'active', NOW()) RETURNING id",
    )
    .bind(format!("{slug}-owner@vote3308.test"))
    .fetch_one(pool)
    .await
    .expect("seed user");

    let building: Uuid = sqlx::query_scalar(
        "INSERT INTO buildings (organization_id, name, street, city, postal_code, country) \
         VALUES ($1, 'B', 'S', 'C', '00000', 'SK') RETURNING id",
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed building");

    let unit: Uuid = sqlx::query_scalar(
        "INSERT INTO units (building_id, designation) VALUES ($1, '1A') RETURNING id",
    )
    .bind(building)
    .fetch_one(pool)
    .await
    .expect("seed unit");

    sqlx::query(
        "INSERT INTO unit_residents (unit_id, user_id, resident_type) VALUES ($1, $2, 'owner')",
    )
    .bind(unit)
    .bind(owner)
    .execute(pool)
    .await
    .expect("seed resident");

    let vote = repo
        .create_vote(CreateVote {
            organization_id: org,
            building_id: building,
            title: format!("{slug} vote"),
            description: None,
            start_at: None,
            end_at: chrono::Utc::now() + chrono::Duration::days(7),
            quorum_type: db::models::vote::quorum_type::SIMPLE_MAJORITY.to_string(),
            quorum_percentage: Some(50),
            allow_delegation: Some(false),
            anonymous_voting: Some(false),
            created_by: owner,
        })
        .await
        .expect("seed vote");

    repo.add_question(CreateVoteQuestion {
        vote_id: vote.id,
        question_text: "Q?".to_string(),
        description: None,
        question_type: db::models::vote::question_type::YES_NO.to_string(),
        options: vec![QuestionOption {
            id: Uuid::new_v4(),
            text: "Yes".to_string(),
            order: 0,
        }],
        display_order: Some(0),
        is_required: Some(true),
    })
    .await
    .expect("seed question");

    let top_comment = repo
        .add_comment(CreateVoteComment {
            vote_id: vote.id,
            user_id: owner,
            parent_id: None,
            content: "top".to_string(),
            ai_consent: false,
        })
        .await
        .expect("seed top comment");

    repo.add_comment(CreateVoteComment {
        vote_id: vote.id,
        user_id: owner,
        parent_id: Some(top_comment.id),
        content: "reply".to_string(),
        ai_consent: false,
    })
    .await
    .expect("seed reply");

    sqlx::query(
        "INSERT INTO vote_responses \
         (vote_id, user_id, unit_id, delegation_id, is_delegated, answers, vote_weight, response_hash) \
         VALUES ($1, $2, $3, NULL, false, '{}'::jsonb, 1.0, $4)",
    )
    .bind(vote.id)
    .bind(owner)
    .bind(unit)
    .bind(format!("hash-{slug}"))
    .execute(pool)
    .await
    .expect("seed response");

    OrgFixture {
        org,
        owner,
        unit,
        vote: vote.id,
        top_comment: top_comment.id,
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn vote_sub_resource_reads_are_org_scoped(pool: PgPool) {
    let repo = VoteRepository::new(pool.clone());
    super_admin_ctx(&pool).await;

    let a = seed_org_fixture(&pool, &repo, "vote3308a").await;
    let b = seed_org_fixture(&pool, &repo, "vote3308b").await;

    // --- get_questions_for_org ------------------------------------------------
    let own = repo
        .get_questions_for_org(a.vote, a.org, &pool)
        .await
        .expect("questions own-org");
    assert_eq!(own.len(), 1, "own-org vote returns its question");

    let wrong_org = repo
        .get_questions_for_org(a.vote, b.org, &pool)
        .await
        .expect("questions wrong-org");
    assert!(
        wrong_org.is_empty(),
        "#3308: org B's id against org A's vote must return no questions — the \
         SQL org predicate, not just vote_in_org, is the boundary"
    );

    // --- get_user_response_for_org -------------------------------------------
    assert!(
        repo.get_user_response_for_org(a.vote, a.unit, a.org, &pool)
            .await
            .expect("response own-org")
            .is_some(),
        "own-org response is visible"
    );
    assert!(
        repo.get_user_response_for_org(a.vote, a.unit, b.org, &pool)
            .await
            .expect("response wrong-org")
            .is_none(),
        "#3308: wrong-org lookup must not return another org's ballot"
    );

    // --- list_comments_for_org -----------------------------------------------
    assert_eq!(
        repo.list_comments_for_org(a.vote, a.org, false, &pool)
            .await
            .expect("comments own-org")
            .len(),
        1,
        "own-org vote returns its top-level comment"
    );
    assert!(
        repo.list_comments_for_org(a.vote, b.org, false, &pool)
            .await
            .expect("comments wrong-org")
            .is_empty(),
        "#3308: wrong-org lookup must not return another org's comments"
    );

    // --- list_replies_for_org ------------------------------------------------
    assert_eq!(
        repo.list_replies_for_org(a.top_comment, a.vote, a.org, false, &pool)
            .await
            .expect("replies own-org")
            .len(),
        1,
        "own-org thread returns its reply"
    );
    assert!(
        repo.list_replies_for_org(a.top_comment, a.vote, b.org, false, &pool)
            .await
            .expect("replies wrong-org")
            .is_empty(),
        "#3308: wrong-org lookup must not return another org's replies"
    );
    assert!(
        repo.list_replies_for_org(a.top_comment, b.vote, a.org, false, &pool)
            .await
            .expect("replies cross-vote")
            .is_empty(),
        "#3308: a comment id paired with a different vote must return nothing"
    );

    // --- check_eligibility_for_org -------------------------------------------
    let mut conn = pool.acquire().await.expect("acquire");
    assert!(
        repo.check_eligibility_for_org(a.vote, a.owner, a.org, &mut conn)
            .await
            .is_ok(),
        "own-org eligibility resolves"
    );
    assert!(
        matches!(
            repo.check_eligibility_for_org(a.vote, a.owner, b.org, &mut conn)
                .await,
            Err(sqlx::Error::RowNotFound)
        ),
        "#3308: eligibility for a vote under another org must be RowNotFound, \
         not a pool-bypass computation on the vote"
    );
}
