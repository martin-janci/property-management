//! Background job repository (Epic 71, Story 71.3).
//!
//! Provides database operations for background job queue management.

use crate::models::infrastructure::{
    BackgroundJob, BackgroundJobExecution, BackgroundJobQuery, BackgroundJobQueueStats,
    CreateBackgroundJob,
};
use crate::DbPool;
use chrono::{DateTime, Utc};
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for background job operations.
#[derive(Clone)]
pub struct BackgroundJobRepository {
    pool: DbPool,
}

impl BackgroundJobRepository {
    /// Create a new BackgroundJobRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new background job.
    pub async fn create(
        &self,
        data: CreateBackgroundJob,
        created_by: Option<Uuid>,
    ) -> Result<BackgroundJob, SqlxError> {
        let job = sqlx::query_as::<_, BackgroundJob>(
            r#"
            INSERT INTO background_jobs (
                job_type, priority, payload, scheduled_at, queue, max_attempts, org_id, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(&data.job_type)
        .bind(data.priority.unwrap_or(0))
        .bind(&data.payload)
        .bind(data.scheduled_at.unwrap_or_else(Utc::now))
        .bind(data.queue.as_deref().unwrap_or("default"))
        .bind(data.max_attempts.unwrap_or(3))
        .bind(data.org_id)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(job)
    }

    /// Find a job by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<BackgroundJob>, SqlxError> {
        let job = sqlx::query_as::<_, BackgroundJob>("SELECT * FROM background_jobs WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(job)
    }

    /// Claim a job for processing (atomic operation).
    ///
    /// This uses PostgreSQL's SKIP LOCKED to prevent contention between workers.
    pub async fn claim_job(
        &self,
        queue: &str,
        worker_id: &str,
        job_types: Option<&[&str]>,
    ) -> Result<Option<BackgroundJob>, SqlxError> {
        let job_types_arr: Option<Vec<String>> =
            job_types.map(|jt| jt.iter().map(|s| s.to_string()).collect());

        // `claim_background_job` returns the composite `background_jobs` type.
        // When no job is claimable the function returns a NULL composite, which
        // `SELECT * FROM ...` expands into a single all-NULL row. Without the
        // `WHERE id IS NOT NULL` guard that row reaches the decoder and fails
        // with `UnexpectedNullError` on the non-nullable `id`; with it, an empty
        // claim collapses to zero rows so `fetch_optional` yields `None`.
        let job = sqlx::query_as::<_, BackgroundJob>(
            "SELECT * FROM claim_background_job($1, $2, $3) WHERE id IS NOT NULL",
        )
        .bind(queue)
        .bind(worker_id)
        .bind(job_types_arr)
        .fetch_optional(&self.pool)
        .await?;

        Ok(job)
    }

    /// Complete a job successfully.
    pub async fn complete_job(
        &self,
        job_id: Uuid,
        result: Option<serde_json::Value>,
    ) -> Result<Option<BackgroundJob>, SqlxError> {
        let job =
            sqlx::query_as::<_, BackgroundJob>("SELECT * FROM complete_background_job($1, $2)")
                .bind(job_id)
                .bind(result)
                .fetch_optional(&self.pool)
                .await?;

        Ok(job)
    }

    /// Fail a job with an error.
    pub async fn fail_job(
        &self,
        job_id: Uuid,
        error_message: &str,
        error_details: Option<serde_json::Value>,
    ) -> Result<Option<BackgroundJob>, SqlxError> {
        let job =
            sqlx::query_as::<_, BackgroundJob>("SELECT * FROM fail_background_job($1, $2, $3)")
                .bind(job_id)
                .bind(error_message)
                .bind(error_details)
                .fetch_optional(&self.pool)
                .await?;

        Ok(job)
    }

    /// Cancel a job.
    pub async fn cancel_job(&self, job_id: Uuid) -> Result<Option<BackgroundJob>, SqlxError> {
        let job = sqlx::query_as::<_, BackgroundJob>(
            r#"
            UPDATE background_jobs
            SET status = 'cancelled', completed_at = NOW()
            WHERE id = $1 AND status IN ('pending', 'scheduled', 'retrying')
            RETURNING *
            "#,
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(job)
    }

    /// Retry a failed job.
    pub async fn retry_job(
        &self,
        job_id: Uuid,
        scheduled_at: Option<DateTime<Utc>>,
        reset_attempts: bool,
    ) -> Result<Option<BackgroundJob>, SqlxError> {
        let job = sqlx::query_as::<_, BackgroundJob>(
            r#"
            UPDATE background_jobs
            SET status = 'pending',
                scheduled_at = COALESCE($2, NOW()),
                attempts = CASE WHEN $3 THEN 0 ELSE attempts END,
                error_message = NULL,
                error_details = NULL,
                completed_at = NULL
            WHERE id = $1 AND status IN ('failed', 'cancelled')
            RETURNING *
            "#,
        )
        .bind(job_id)
        .bind(scheduled_at)
        .bind(reset_attempts)
        .fetch_optional(&self.pool)
        .await?;

        Ok(job)
    }

    /// List jobs with query filters.
    pub async fn list(
        &self,
        query: BackgroundJobQuery,
    ) -> Result<(Vec<BackgroundJob>, i64), SqlxError> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        // Build dynamic WHERE clause
        let mut conditions = vec!["1=1".to_string()];
        let mut param_idx = 0;

        if query.job_type.is_some() {
            param_idx += 1;
            conditions.push(format!("job_type = ${}", param_idx));
        }
        if query.status.is_some() {
            param_idx += 1;
            conditions.push(format!("status = ${}", param_idx));
        }
        if query.queue.is_some() {
            param_idx += 1;
            conditions.push(format!("queue = ${}", param_idx));
        }
        if query.org_id.is_some() {
            param_idx += 1;
            conditions.push(format!("org_id = ${}", param_idx));
        }
        if query.from_time.is_some() {
            param_idx += 1;
            conditions.push(format!("created_at >= ${}", param_idx));
        }
        if query.to_time.is_some() {
            param_idx += 1;
            conditions.push(format!("created_at <= ${}", param_idx));
        }

        let where_clause = conditions.join(" AND ");

        // Count query
        let count_query = format!(
            "SELECT COUNT(*) FROM background_jobs WHERE {}",
            where_clause
        );
        let mut count_q = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(count_query));

        if let Some(ref job_type) = query.job_type {
            count_q = count_q.bind(job_type);
        }
        if let Some(ref status) = query.status {
            count_q = count_q.bind(status);
        }
        if let Some(ref queue) = query.queue {
            count_q = count_q.bind(queue);
        }
        if let Some(org_id) = query.org_id {
            count_q = count_q.bind(org_id);
        }
        if let Some(from_time) = query.from_time {
            count_q = count_q.bind(from_time);
        }
        if let Some(to_time) = query.to_time {
            count_q = count_q.bind(to_time);
        }

        let total = count_q.fetch_one(&self.pool).await?;

        // Data query - use parameterized LIMIT/OFFSET for safety
        param_idx += 1;
        let limit_param = param_idx;
        param_idx += 1;
        let offset_param = param_idx;

        let data_query = format!(
            r#"
            SELECT * FROM background_jobs
            WHERE {}
            ORDER BY priority DESC, created_at DESC
            LIMIT ${} OFFSET ${}
            "#,
            where_clause, limit_param, offset_param
        );

        let mut data_q = sqlx::query_as::<_, BackgroundJob>(sqlx::AssertSqlSafe(data_query));

        if let Some(ref job_type) = query.job_type {
            data_q = data_q.bind(job_type);
        }
        if let Some(ref status) = query.status {
            data_q = data_q.bind(status);
        }
        if let Some(ref queue) = query.queue {
            data_q = data_q.bind(queue);
        }
        if let Some(org_id) = query.org_id {
            data_q = data_q.bind(org_id);
        }
        if let Some(from_time) = query.from_time {
            data_q = data_q.bind(from_time);
        }
        if let Some(to_time) = query.to_time {
            data_q = data_q.bind(to_time);
        }

        // Bind LIMIT and OFFSET parameters
        data_q = data_q.bind(limit);
        data_q = data_q.bind(offset);

        let jobs = data_q.fetch_all(&self.pool).await?;

        Ok((jobs, total))
    }

    /// Get queue statistics.
    pub async fn get_queue_stats(&self, queue: &str) -> Result<BackgroundJobQueueStats, SqlxError> {
        let stats = sqlx::query_as::<_, BackgroundJobQueueStatsRow>(
            "SELECT * FROM get_background_job_queue_stats($1)",
        )
        .bind(queue)
        .fetch_one(&self.pool)
        .await?;

        Ok(BackgroundJobQueueStats {
            queue: stats.queue,
            pending_count: stats.pending_count,
            running_count: stats.running_count,
            failed_count_24h: stats.failed_count_24h,
            completed_count_24h: stats.completed_count_24h,
            avg_duration_ms: stats.avg_duration_ms,
            p95_duration_ms: stats.p95_duration_ms,
            retrying_count: stats.retrying_count,
            oldest_pending_age_seconds: stats.oldest_pending_age_seconds,
        })
    }

    /// Get all queue statistics.
    pub async fn get_all_queue_stats(&self) -> Result<Vec<BackgroundJobQueueStats>, SqlxError> {
        let queues = sqlx::query_scalar::<_, String>("SELECT DISTINCT queue FROM background_jobs")
            .fetch_all(&self.pool)
            .await?;

        let mut stats = Vec::new();
        for queue in queues {
            stats.push(self.get_queue_stats(&queue).await?);
        }

        Ok(stats)
    }

    /// Get job execution history.
    pub async fn get_executions(
        &self,
        job_id: Uuid,
    ) -> Result<Vec<BackgroundJobExecution>, SqlxError> {
        let executions = sqlx::query_as::<_, BackgroundJobExecution>(
            r#"
            SELECT * FROM background_job_executions
            WHERE job_id = $1
            ORDER BY attempt DESC
            "#,
        )
        .bind(job_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(executions)
    }

    /// Clean up old completed/failed jobs.
    pub async fn cleanup_old_jobs(&self, retention_days: i32) -> Result<i64, SqlxError> {
        let result = sqlx::query(
            r#"
            DELETE FROM background_jobs
            WHERE status IN ('completed', 'failed', 'cancelled')
              AND completed_at < NOW() - ($1 || ' days')::INTERVAL
            "#,
        )
        .bind(retention_days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Get stuck jobs (running for too long).
    pub async fn get_stuck_jobs(
        &self,
        timeout_minutes: i32,
    ) -> Result<Vec<BackgroundJob>, SqlxError> {
        let jobs = sqlx::query_as::<_, BackgroundJob>(
            r#"
            SELECT * FROM background_jobs
            WHERE status = 'running'
              AND started_at < NOW() - ($1 || ' minutes')::INTERVAL
            ORDER BY started_at ASC
            "#,
        )
        .bind(timeout_minutes)
        .fetch_all(&self.pool)
        .await?;

        Ok(jobs)
    }

    /// Reset stuck jobs, honoring the per-job retry budget.
    ///
    /// A job is "stuck" when it has been `running` longer than
    /// `timeout_minutes`. `claim_background_job` already incremented `attempts`
    /// when the job was claimed, so the comparison is against the attempts
    /// already consumed:
    ///
    /// - jobs that still have budget (`attempts < max_attempts`) are reset to
    ///   `pending` so a healthy worker can pick them up again;
    /// - jobs that have exhausted their budget (`attempts >= max_attempts`) are
    ///   routed to the terminal `timed_out` state with `completed_at` set, so a
    ///   job that hangs on every run can no longer loop forever.
    ///
    /// Returns the total number of stuck jobs that were transitioned (reset +
    /// timed out).
    pub async fn reset_stuck_jobs(&self, timeout_minutes: i32) -> Result<i64, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE background_jobs
            SET status = CASE
                    WHEN attempts < max_attempts THEN 'pending'::background_job_status
                    ELSE 'timed_out'::background_job_status
                END,
                worker_id = CASE WHEN attempts < max_attempts THEN NULL ELSE worker_id END,
                started_at = CASE WHEN attempts < max_attempts THEN NULL ELSE started_at END,
                completed_at = CASE WHEN attempts < max_attempts THEN completed_at ELSE NOW() END,
                error_message = 'Job reset due to timeout'
            WHERE status = 'running'
              AND started_at < NOW() - ($1 || ' minutes')::INTERVAL
            "#,
        )
        .bind(timeout_minutes)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }
}

/// Internal row type for queue stats query.
#[derive(sqlx::FromRow)]
struct BackgroundJobQueueStatsRow {
    queue: String,
    pending_count: i64,
    running_count: i64,
    failed_count_24h: i64,
    completed_count_24h: i64,
    avg_duration_ms: Option<f64>,
    p95_duration_ms: Option<f64>,
    retrying_count: i64,
    oldest_pending_age_seconds: Option<i64>,
}
