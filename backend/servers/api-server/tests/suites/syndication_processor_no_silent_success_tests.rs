//! Regression test: the syndication processor must NOT silently fake success.
//!
//! There is no real outbound portal API integration anywhere in the backend
//! (`integrations::portals::PortalClient` is itself a stub). The processor used
//! to *pretend* it had published: for a PUBLISH job it minted a fabricated
//! `external_id` and wrote `syndication_status::SYNCED`, which made the
//! dashboard report a listing as live on SReality/Bezrealitky/etc. when nothing
//! had ever been sent — a data-integrity hazard. The processor must instead
//! surface the unimplemented work explicitly: mark the syndication `FAILED`
//! with an actionable `last_error`, invent no external ID, and return an error.
//!
//! Needs a migrated `PgPool` (`sqlx::test`); skipped offline, run in CI.

#![allow(dead_code)]

use api_server::services::SyndicationService;
use db::models::{syndication_job_type, syndication_status, SyndicationJobPayload};
use db::repositories::{BackgroundJobRepository, ListingRepository};
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_listing_with_syndication(pool: &PgPool, portal: &str) -> Uuid {
    let uid = Uuid::new_v4();

    let org_id: Uuid = sqlx::query_scalar(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ($1,$2,$3,'active') RETURNING id",
    )
    .bind(format!("Syndication Test {uid}"))
    .bind(format!("syndication-test-{uid}"))
    .bind(format!("{uid}@syndication-test.internal"))
    .fetch_one(pool)
    .await
    .expect("seed_org");

    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind) \
         VALUES ($1,'hash','Syndication Tester','active',NOW(),'staff') RETURNING id",
    )
    .bind(format!("{uid}@syndication-test.internal"))
    .fetch_one(pool)
    .await
    .expect("seed_user");

    let listing_id: Uuid = sqlx::query_scalar(
        "INSERT INTO listings \
             (organization_id, transaction_type, title, property_type, \
              street, city, postal_code, country, price, currency, \
              is_negotiable, status, created_by) \
         VALUES ($1,'sale','Syndication Listing','apartment', \
                 'Main St 1','Testville','00000','SK',150000,'EUR',false,'active',$2) \
         RETURNING id",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed_listing");

    sqlx::query("INSERT INTO listing_syndications (listing_id, portal, status) VALUES ($1,$2,$3)")
        .bind(listing_id)
        .bind(portal)
        .bind(syndication_status::PENDING)
        .execute(pool)
        .await
        .expect("seed_syndication");

    listing_id
}

/// A PUBLISH job for a real listing must not be marked `SYNCED` with a
/// fabricated external ID. With no outbound integration, the processor must
/// fail explicitly and record `FAILED` + a `last_error`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publish_job_does_not_fabricate_synced_status(pool: PgPool) {
    let listing_repo = ListingRepository::new(pool.clone());
    let service = SyndicationService::new(
        BackgroundJobRepository::new(pool.clone()),
        listing_repo.clone(),
    );

    let portal = "sreality";
    let listing_id = seed_listing_with_syndication(&pool, portal).await;

    let payload = SyndicationJobPayload {
        listing_id,
        portal: portal.to_string(),
        operation: syndication_job_type::PUBLISH.to_string(),
        previous_status: None,
        new_status: Some("active".to_string()),
        organization_id: None,
    };

    // The processor must NOT report success for work it cannot actually do.
    let result = service.process_syndication_job(&payload).await;
    assert!(
        result.is_err(),
        "processor must fail explicitly when no outbound portal integration exists, got Ok({result:?})"
    );

    // And the persisted syndication state must be honest: FAILED, no fabricated
    // external ID, and an explanatory last_error.
    let syndications = listing_repo
        .get_syndications(listing_id)
        .await
        .expect("load syndications");
    let row = syndications
        .into_iter()
        .find(|s| s.portal == portal)
        .expect("syndication row exists");

    assert_eq!(
        row.status,
        syndication_status::FAILED,
        "syndication must not be marked SYNCED for a portal that was never contacted"
    );
    assert!(
        row.external_id.is_none(),
        "processor must not invent an external_id, got {:?}",
        row.external_id
    );
    assert!(
        row.last_error.is_some(),
        "an explicit failure reason must be recorded in last_error"
    );
}
