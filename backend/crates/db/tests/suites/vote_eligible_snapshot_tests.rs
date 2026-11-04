//! Round-3 audit #25 / #5: vote eligible-voter snapshot and vote detail
//! questions.
//!
//! - #25: every vote showed "0 z 0 oprávnených" because the seed inserted
//!   published votes without `eligible_count`, so quorum could never be met.
//!   `VoteRepository::publish` snapshots it; the seed now does the same.
//! - #5: `find_by_id_with_details_for_org` (backing `GET /voting/{id}`)
//!   returned no `questions`, so the ballot page crashed.

use db::models::{CreateVoteQuestion, PublishVote, QuestionOption};
use db::repositories::VoteRepository;
use db::seed::SeedFactories;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

async fn super_admin_ctx(conn: &mut PgConnection) {
    sqlx::query("SELECT set_request_context(NULL, NULL, TRUE)")
        .execute(conn)
        .await
        .expect("set_request_context");
}

struct Fixture {
    org: Uuid,
    building: Uuid,
    owner_a: Uuid,
    owner_b: Uuid,
    tenant: Uuid,
}

/// Org + building with 3 units: two with a current owner, one with only a
/// tenant, plus one unit whose owner moved out (end_date set). Eligible = 2.
async fn fixture(pool: &PgPool, slug: &str) -> Fixture {
    let mut c = pool.acquire().await.unwrap();
    super_admin_ctx(&mut c).await;

    let org: Uuid = sqlx::query_scalar(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ($1, $1, $2, 'active') RETURNING id",
    )
    .bind(slug)
    .bind(format!("{slug}@elig.test"))
    .fetch_one(&mut *c)
    .await
    .unwrap();

    let mut users = Vec::new();
    for i in 0..4 {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (email, password_hash, name, status, email_verified_at) \
             VALUES ($1, 'h', 'U', 'active', NOW()) RETURNING id",
        )
        .bind(format!("{slug}-u{i}@elig.test"))
        .fetch_one(&mut *c)
        .await
        .unwrap();
        users.push(id);
    }

    let building: Uuid = sqlx::query_scalar(
        "INSERT INTO buildings (organization_id, name, street, city, postal_code, country) \
         VALUES ($1, 'B', 'S', 'C', '00000', 'SK') RETURNING id",
    )
    .bind(org)
    .fetch_one(&mut *c)
    .await
    .unwrap();

    let mut units = Vec::new();
    for d in ["1", "2", "3", "4"] {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO units (building_id, designation) VALUES ($1, $2) RETURNING id",
        )
        .bind(building)
        .bind(d)
        .fetch_one(&mut *c)
        .await
        .unwrap();
        units.push(id);
    }

    let residents = [
        (units[0], users[0], "owner", None::<&str>),
        (units[1], users[1], "owner", None),
        (units[2], users[2], "tenant", None),
        (units[3], users[3], "owner", Some("2020-01-01")),
    ];
    for (unit, user, kind, end) in residents {
        sqlx::query(
            "INSERT INTO unit_residents (unit_id, user_id, resident_type, start_date, end_date) \
             VALUES ($1, $2, $3::resident_type, DATE '2019-01-01', $4::date)",
        )
        .bind(unit)
        .bind(user)
        .bind(kind)
        .bind(end)
        .execute(&mut *c)
        .await
        .unwrap();
    }

    Fixture {
        org,
        building,
        owner_a: users[0],
        owner_b: users[1],
        tenant: users[2],
    }
}

async fn eligible_count(pool: &PgPool, vote: Uuid) -> Option<i32> {
    sqlx::query_scalar("SELECT eligible_count FROM votes WHERE id = $1")
        .bind(vote)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn seeded_published_votes_snapshot_eligible_count(pool: PgPool) {
    let f = fixture(&pool, "seed-elig").await;
    let factories = SeedFactories::new(&pool);

    let active = factories
        .create_vote(
            f.org,
            f.building,
            "A",
            None,
            "active",
            f.owner_a,
            "simple_majority",
        )
        .await
        .expect("seed active vote");
    let closed = factories
        .create_vote(
            f.org,
            f.building,
            "C",
            None,
            "closed",
            f.owner_a,
            "simple_majority",
        )
        .await
        .expect("seed closed vote");
    let draft = factories
        .create_vote(
            f.org,
            f.building,
            "D",
            None,
            "draft",
            f.owner_a,
            "simple_majority",
        )
        .await
        .expect("seed draft vote");

    assert_eq!(eligible_count(&pool, active).await, Some(2));
    assert_eq!(eligible_count(&pool, closed).await, Some(2));
    assert_eq!(eligible_count(&pool, draft).await, Some(0));
}

#[sqlx::test(migrations = "./migrations")]
async fn publish_snapshots_eligible_count(pool: PgPool) {
    let f = fixture(&pool, "pub-elig").await;
    let draft = SeedFactories::new(&pool)
        .create_vote(
            f.org,
            f.building,
            "D",
            None,
            "draft",
            f.owner_a,
            "simple_majority",
        )
        .await
        .unwrap();
    assert_eq!(eligible_count(&pool, draft).await, Some(0));

    let repo = VoteRepository::new(pool.clone());
    let vote = repo
        .publish(draft, f.owner_b, PublishVote { start_at: None })
        .await
        .expect("publish");
    assert_eq!(vote.status, "active");
    assert_eq!(
        vote.eligible_count,
        Some(2),
        "two units with a current owner"
    );
    let _ = f.tenant;
}

#[sqlx::test(migrations = "./migrations")]
async fn vote_details_include_ordered_questions(pool: PgPool) {
    let f = fixture(&pool, "detail-q").await;
    let vote = SeedFactories::new(&pool)
        .create_vote(
            f.org,
            f.building,
            "Q",
            None,
            "draft",
            f.owner_a,
            "simple_majority",
        )
        .await
        .unwrap();

    let repo = VoteRepository::new(pool.clone());
    let opt = |text: &str, order| QuestionOption {
        id: Uuid::new_v4(),
        text: text.to_string(),
        order,
    };
    for (order, text) in [(1, "Second"), (0, "First")] {
        repo.add_question(CreateVoteQuestion {
            vote_id: vote,
            question_text: text.to_string(),
            description: None,
            question_type: "single_choice".to_string(),
            options: vec![opt("A", 0), opt("B", 1)],
            display_order: Some(order),
            is_required: Some(true),
        })
        .await
        .expect("add question");
    }

    let details = repo
        .find_by_id_with_details_for_org(vote, f.org)
        .await
        .unwrap()
        .expect("vote visible to its org");
    assert_eq!(details.question_count, 2);
    let texts: Vec<_> = details
        .questions
        .iter()
        .map(|q| q.question_text.as_str())
        .collect();
    assert_eq!(texts, ["First", "Second"]);
    assert_eq!(
        details.questions[0].options.as_array().map(Vec::len),
        Some(2)
    );

    // Serialized contract: `questions` sits next to the flattened vote fields.
    let json = serde_json::to_value(&details).unwrap();
    assert_eq!(json["questions"].as_array().map(Vec::len), Some(2));
    assert_eq!(json["id"], vote.to_string());

    // Another org still gets nothing.
    assert!(repo
        .find_by_id_with_details_for_org(vote, Uuid::new_v4())
        .await
        .unwrap()
        .is_none());
}
