//! Regression test: queued syndication jobs must actually be drained.
//!
//! `SyndicationService` enqueued `background_jobs` rows on the `syndication`
//! queue but nothing ever claimed them, so they stayed `pending` forever. The
//! drain claims each due job, runs `process_syndication_job`, and completes or
//! fails the job so it leaves the `pending` state.
//!
//! Needs a migrated `PgPool` (`sqlx::test`); skipped offline, run in CI.

#![allow(dead_code)]

use api_server::services::{syndication::SYNDICATION_QUEUE, SyndicationService};
use db::models::{
    syndication_job_type, BackgroundJobStatus, CreateBackgroundJob, SyndicationJobPayload,
};
use db::repositories::{BackgroundJobRepository, ListingRepository};
use sqlx::PgPool;
use uuid::Uuid;

/// A queued job (here for a listing that does not exist, so processing fails
/// deterministically) must be claimed and moved out of `pending`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn drain_queue_claims_and_finalizes_pending_jobs(pool: PgPool) {
    let jobs = BackgroundJobRepository::new(pool.clone());
    let service = SyndicationService::new(jobs.clone(), ListingRepository::new(pool.clone()));

    let payload = SyndicationJobPayload {
        listing_id: Uuid::new_v4(),
        portal: "reality_portal".to_string(),
        operation: syndication_job_type::PUBLISH.to_string(),
        previous_status: None,
        new_status: None,
        organization_id: None,
    };
    let job = jobs
        .create(
            CreateBackgroundJob {
                job_type: syndication_job_type::PUBLISH.to_string(),
                priority: Some(5),
                payload: serde_json::to_value(&payload).unwrap(),
                scheduled_at: None,
                queue: Some(SYNDICATION_QUEUE.to_string()),
                max_attempts: Some(1),
                org_id: None,
            },
            None,
        )
        .await
        .expect("enqueue job");

    let processed = service.drain_queue(10).await.expect("drain");
    assert_eq!(processed, 1, "the queued job must be claimed and processed");

    let after = jobs
        .find_by_id(job.id)
        .await
        .expect("find")
        .expect("job exists");
    assert_ne!(
        after.status,
        BackgroundJobStatus::Pending,
        "a drained job must not remain pending"
    );
    assert_eq!(after.status, BackgroundJobStatus::Failed);

    // Queue is now empty: a second drain is a no-op.
    assert_eq!(service.drain_queue(10).await.expect("drain 2"), 0);
}
