//! Report worker (gap audit 3.1): turns queued report work into files.
//!
//! Two sources of work, both drained on every tick:
//!
//! 1. **Scheduled executions.** The scheduler (`fire_due_report_schedules`)
//!    and the "Run now" / "Retry" endpoints record `pending` rows in
//!    `report_executions`. The worker claims each one, generates the report
//!    with [`ReportGenerator`], stores it in S3, marks the execution
//!    `completed` (or `failed` with a reason) and e-mails the file to the
//!    schedule's recipients.
//! 2. **Large exports.** `POST /api/v1/reports/export` queues a
//!    `report_export` job on the `reports` background-job queue when a report
//!    is too big to return inline. The worker generates and stores the file
//!    and completes the job with its `file_key`; the status endpoint mints a
//!    presigned download URL from it.
//!
//! RLS: `report_schedules` / `report_executions` carry FORCE RLS. The worker
//! reads due work cross-org under the global-read context (SELECT-only legs,
//! migration 00218) and performs each execution's writes under that
//! execution's own tenant context, on one dedicated connection that is
//! cleared (or discarded) before it returns to the pool.

use chrono::NaiveDate;
use db::models::PendingReportExecution;
use db::repositories::{BackgroundJobRepository, ReportScheduleRepository};
use db::DbPool;
use integrations::StorageService;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::email::EmailService;
use super::report_generator::{
    GeneratedReport, ReportFormat, ReportGenerator, ReportKind, ReportParams,
};

/// Background-job queue for large report exports.
pub const REPORT_QUEUE: &str = "reports";
/// Background-job type for large report exports.
pub const REPORT_EXPORT_JOB: &str = "report_export";
const WORKER_ID: &str = "api-server-report-worker";
/// A `running` execution older than this is assumed orphaned and re-claimed.
const STALE_RUNNING_MINUTES: i64 = 30;
/// Attach files up to this size to e-mails; larger ones are linked.
const MAX_ATTACHMENT_BYTES: usize = 10 * 1024 * 1024;

/// Worker configuration.
#[derive(Debug, Clone)]
pub struct ReportWorkerConfig {
    pub enabled: bool,
    pub poll_interval_secs: u64,
    pub batch_limit: i64,
}

impl Default for ReportWorkerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_secs: 30,
            batch_limit: 10,
        }
    }
}

impl ReportWorkerConfig {
    /// Build from `REPORT_WORKER_ENABLED` / `REPORT_WORKER_INTERVAL_SECS`.
    pub fn from_env() -> Self {
        let d = Self::default();
        Self {
            enabled: std::env::var("REPORT_WORKER_ENABLED")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(d.enabled),
            poll_interval_secs: std::env::var("REPORT_WORKER_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .filter(|&n| n > 0)
                .unwrap_or(d.poll_interval_secs),
            ..d
        }
    }
}

/// Payload of a `report_export` background job (written by `export_report`).
#[derive(Debug, Deserialize)]
pub struct ReportExportPayload {
    pub report_type: String,
    pub format: String,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
}

/// Storage key for a scheduled execution's file.
pub fn execution_file_key(org: Uuid, schedule: Uuid, execution: Uuid, ext: &str) -> String {
    format!(
        "reports/{}/schedules/{}/{}.{}",
        org, schedule, execution, ext
    )
}

/// Storage key for an export job's file.
pub fn export_file_key(org: Uuid, job: Uuid, ext: &str) -> String {
    format!("reports/{}/exports/{}.{}", org, job, ext)
}

/// Report period for a scheduled run: the cadence window ending today
/// (daily → yesterday..today, weekly → last 7 days, monthly → last month).
pub fn schedule_period(frequency: &str, today: NaiveDate) -> (NaiveDate, NaiveDate) {
    let from = match frequency {
        "daily" => today - chrono::Duration::days(1),
        "weekly" => today - chrono::Duration::days(7),
        _ => today
            .checked_sub_months(chrono::Months::new(1))
            .unwrap_or_else(|| today - chrono::Duration::days(30)),
    };
    (from, today)
}

/// Error recorded on a failed execution.
#[derive(Debug)]
struct RunError {
    code: &'static str,
    message: String,
}

impl RunError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// The report worker.
#[derive(Clone)]
pub struct ReportWorker {
    pool: DbPool,
    generator: ReportGenerator,
    schedules: ReportScheduleRepository,
    jobs: BackgroundJobRepository,
    storage: Option<StorageService>,
    email: EmailService,
}

impl ReportWorker {
    pub fn new(pool: DbPool, storage: Option<StorageService>, email: EmailService) -> Self {
        Self {
            generator: ReportGenerator::from_pool(pool.clone()),
            schedules: ReportScheduleRepository::new(pool.clone()),
            jobs: BackgroundJobRepository::new(pool.clone()),
            pool,
            storage,
            email,
        }
    }

    /// Spawn the polling loop.
    pub fn start(self, config: ReportWorkerConfig) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            if !config.enabled {
                tracing::info!("Report worker disabled - not starting");
                return;
            }
            tracing::info!(
                poll_interval_secs = config.poll_interval_secs,
                storage = self.storage.is_some(),
                "Report worker started"
            );
            let mut ticker =
                tokio::time::interval(std::time::Duration::from_secs(config.poll_interval_secs));
            loop {
                ticker.tick().await;
                self.tick(config.batch_limit).await;
            }
        })
    }

    /// One pass over both queues. Errors are logged, never propagated, so a
    /// bad row can't stop the loop.
    pub async fn tick(&self, batch_limit: i64) {
        match self.process_scheduled_executions(batch_limit).await {
            Ok(0) => {}
            Ok(n) => tracing::info!(processed = n, "Processed scheduled report executions"),
            Err(e) => tracing::error!(error = %e, "Scheduled report pass failed"),
        }
        match self.process_export_jobs(batch_limit as usize).await {
            Ok(0) => {}
            Ok(n) => tracing::info!(processed = n, "Processed report export jobs"),
            Err(e) => tracing::error!(error = %e, "Report export pass failed"),
        }
    }

    /// Generate and store one report, returning the file and its storage key.
    async fn generate_and_store(
        &self,
        params: &ReportParams,
        format: ReportFormat,
        key_for: impl FnOnce(&str) -> String,
    ) -> Result<(GeneratedReport, String), RunError> {
        let storage = self.storage.as_ref().ok_or_else(|| {
            RunError::new(
                "STORAGE_NOT_CONFIGURED",
                "File storage (S3) is not configured on the server",
            )
        })?;
        let report = self
            .generator
            .generate(params, format)
            .await
            .map_err(|e| RunError::new("REPORT_GENERATION_FAILED", e))?;
        let key = key_for(format.extension());
        storage
            .upload_system_artifact(&key, report.bytes.clone(), report.content_type)
            .await
            .map_err(|e| RunError::new("STORAGE_UPLOAD_FAILED", e.to_string()))?;
        Ok((report, key))
    }

    // ------------------------------------------------------------------
    // Scheduled executions
    // ------------------------------------------------------------------

    /// Claim and process pending executions. Returns how many were claimed.
    pub async fn process_scheduled_executions(&self, limit: i64) -> Result<usize, String> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| format!("acquire: {}", e))?;

        if let Err(e) = db::tenant_context::set_global_read_context(&mut *conn, true).await {
            db::tenant_context::clear_or_discard(conn, "report_worker").await;
            return Err(format!("global read: {}", e));
        }
        let pending = self
            .schedules
            .list_pending_executions(&mut *conn, limit, STALE_RUNNING_MINUTES)
            .await;
        if let Err(e) = db::tenant_context::set_global_read_context(&mut *conn, false).await {
            db::tenant_context::clear_or_discard(conn, "report_worker").await;
            return Err(format!("clear global read: {}", e));
        }
        let pending = match pending {
            Ok(rows) => rows,
            Err(e) => {
                db::tenant_context::clear_or_discard(conn, "report_worker").await;
                return Err(e.to_string());
            }
        };

        let mut processed = 0;
        for execution in &pending {
            if let Err(e) =
                db::tenant_context::set_tenant_context(&mut *conn, execution.organization_id).await
            {
                tracing::error!(execution_id = %execution.execution_id, error = %e, "Failed to set tenant context");
                continue;
            }
            match self
                .schedules
                .claim_execution(&mut *conn, execution.execution_id, STALE_RUNNING_MINUTES)
                .await
            {
                Ok(true) => {}
                Ok(false) => continue, // another worker took it
                Err(e) => {
                    tracing::error!(execution_id = %execution.execution_id, error = %e, "Failed to claim execution");
                    continue;
                }
            }
            processed += 1;

            let outcome = self.run_execution(execution).await;
            let finalize = match &outcome {
                Ok((report, key)) => {
                    self.schedules
                        .complete_execution(
                            &mut *conn,
                            execution.execution_id,
                            key,
                            &report.file_name,
                            report.bytes.len() as i64,
                        )
                        .await
                }
                Err(err) => {
                    tracing::warn!(
                        execution_id = %execution.execution_id,
                        code = err.code,
                        error = %err.message,
                        "Report execution failed"
                    );
                    self.schedules
                        .fail_execution(&mut *conn, execution.execution_id, err.code, &err.message)
                        .await
                }
            };
            if let Err(e) = finalize {
                tracing::error!(execution_id = %execution.execution_id, error = %e, "Failed to finalize execution");
                continue;
            }
            if let Ok((report, _)) = outcome {
                self.deliver(execution, &report).await;
            }
        }

        db::tenant_context::clear_or_discard(conn, "report_worker").await;
        Ok(processed)
    }

    async fn run_execution(
        &self,
        execution: &PendingReportExecution,
    ) -> Result<(GeneratedReport, String), RunError> {
        let kind = ReportKind::from_builtin_id(execution.report_id).ok_or_else(|| {
            RunError::new(
                "UNKNOWN_REPORT",
                format!("Schedule references unknown report {}", execution.report_id),
            )
        })?;
        let format = ReportFormat::parse(&execution.format).ok_or_else(|| {
            RunError::new(
                "INVALID_FORMAT",
                format!("Unsupported format {}", execution.format),
            )
        })?;
        let tz: chrono_tz::Tz = execution.timezone.parse().unwrap_or(chrono_tz::UTC);
        let today = chrono::Utc::now().with_timezone(&tz).date_naive();
        let (from, to) = schedule_period(&execution.frequency, today);
        let params = ReportParams::new(kind, execution.organization_id, None, Some(from), Some(to));
        self.generate_and_store(&params, format, |ext| {
            execution_file_key(
                execution.organization_id,
                execution.schedule_id,
                execution.execution_id,
                ext,
            )
        })
        .await
    }

    /// E-mail the generated file to the schedule's recipients. Delivery
    /// problems are logged; the file stays downloadable from the history.
    async fn deliver(&self, execution: &PendingReportExecution, report: &GeneratedReport) {
        let recipients = execution.recipient_list();
        if recipients.is_empty() {
            return;
        }
        let link = format!(
            "{}/reports/schedules/{}",
            self.email.base_url().trim_end_matches('/'),
            execution.schedule_id
        );
        let today = chrono::Utc::now().date_naive();
        let subject = format!("{} ({})", execution.schedule_name, today);
        let attach = report.bytes.len() <= MAX_ATTACHMENT_BYTES;
        let body = if attach {
            format!(
                "Your scheduled report \"{}\" is attached.\n\nRun history: {}\n",
                execution.schedule_name, link
            )
        } else {
            format!(
                "Your scheduled report \"{}\" is ready. It is too large to attach; download it here: {}\n",
                execution.schedule_name, link
            )
        };
        for to in &recipients {
            let result = if attach {
                self.email
                    .send_email_with_attachment(
                        to,
                        &subject,
                        &body,
                        &report.file_name,
                        report.content_type,
                        report.bytes.clone(),
                    )
                    .await
            } else {
                self.email
                    .send_action_email(to, &[], &[], &subject, &body, false)
                    .await
            };
            if let Err(e) = result {
                tracing::warn!(
                    execution_id = %execution.execution_id,
                    to = %to,
                    error = %e,
                    "Failed to e-mail scheduled report"
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // Large exports
    // ------------------------------------------------------------------

    /// Claim and process queued `report_export` jobs.
    pub async fn process_export_jobs(&self, limit: usize) -> Result<usize, String> {
        let mut processed = 0;
        while processed < limit {
            let Some(job) = self
                .jobs
                .claim_job(REPORT_QUEUE, WORKER_ID, Some(&[REPORT_EXPORT_JOB]))
                .await
                .map_err(|e| format!("claim: {}", e))?
            else {
                break;
            };
            processed += 1;

            let outcome = match serde_json::from_value::<ReportExportPayload>(job.payload.clone()) {
                Ok(payload) => self.run_export(job.id, &payload).await,
                Err(e) => Err(RunError::new(
                    "INVALID_PAYLOAD",
                    format!("Invalid report export payload: {}", e),
                )),
            };
            let finalize = match outcome {
                Ok((report, key)) => {
                    self.jobs
                        .complete_job(
                            job.id,
                            Some(json!({
                                "file_key": key,
                                "file_name": report.file_name,
                                "content_type": report.content_type,
                                "file_size": report.bytes.len(),
                            })),
                        )
                        .await
                }
                Err(err) => {
                    tracing::warn!(job_id = %job.id, code = err.code, error = %err.message, "Report export failed");
                    self.jobs
                        .fail_job(job.id, &err.message, Some(json!({ "code": err.code })))
                        .await
                }
            };
            if let Err(e) = finalize {
                tracing::error!(job_id = %job.id, error = %e, "Failed to finalize report export job");
            }
        }
        Ok(processed)
    }

    async fn run_export(
        &self,
        job_id: Uuid,
        payload: &ReportExportPayload,
    ) -> Result<(GeneratedReport, String), RunError> {
        let kind = ReportKind::parse(&payload.report_type).ok_or_else(|| {
            RunError::new(
                "INVALID_REPORT_TYPE",
                format!("Unknown report type {}", payload.report_type),
            )
        })?;
        let format = ReportFormat::parse(&payload.format).ok_or_else(|| {
            RunError::new(
                "INVALID_FORMAT",
                format!("Unsupported format {}", payload.format),
            )
        })?;
        let params = ReportParams::new(
            kind,
            payload.organization_id,
            payload.building_id,
            payload.from_date,
            payload.to_date,
        );
        let org = payload.organization_id;
        self.generate_and_store(&params, format, |ext| export_file_key(org, job_id, ext))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn schedule_period_follows_cadence() {
        let today = d("2026-03-31");
        assert_eq!(schedule_period("daily", today), (d("2026-03-30"), today));
        assert_eq!(schedule_period("weekly", today), (d("2026-03-24"), today));
        // Month-end clamps (no 31 February).
        assert_eq!(schedule_period("monthly", today), (d("2026-02-28"), today));
        assert_eq!(
            schedule_period("monthly", d("2026-03-15")),
            (d("2026-02-15"), d("2026-03-15"))
        );
    }

    #[test]
    fn storage_keys_are_tenant_prefixed() {
        let org = Uuid::nil();
        let key = execution_file_key(org, org, org, "pdf");
        assert!(key.starts_with(&format!("reports/{}/schedules/", org)));
        assert!(key.ends_with(".pdf"));
        assert!(export_file_key(org, org, "csv").contains("/exports/"));
    }
}

#[cfg(test)]
mod db_tests {
    use super::*;

    async fn org(pool: &sqlx::PgPool, slug: &str) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO organizations (name, slug, contact_email, status) \
             VALUES ($1, $1, 'r@org.test', 'active') RETURNING id",
        )
        .bind(slug)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn schedule(pool: &sqlx::PgPool, org: Uuid) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO report_schedules (report_id, organization_id, name, frequency, format) \
             VALUES ('a0000000-0000-4000-8000-000000000001', $1, 'Weekly faults', 'weekly', 'csv') \
             RETURNING id",
        )
        .bind(org)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn run_now_and_delete_are_scoped_to_the_organization(pool: sqlx::PgPool) {
        let mine = org(&pool, "mine").await;
        let other = org(&pool, "other").await;
        let id = schedule(&pool, mine).await;
        let repo = ReportScheduleRepository::new(pool.clone());

        assert!(repo
            .queue_run_scoped(&pool, id, other)
            .await
            .unwrap()
            .is_none());
        assert!(!repo.delete_scoped(&pool, id, other).await.unwrap());

        let queued = repo
            .queue_run_scoped(&pool, id, mine)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(queued.status, "pending");
        assert!(repo.delete_scoped(&pool, id, mine).await.unwrap());
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn executions_fail_cleanly_without_storage(pool: sqlx::PgPool) {
        let mine = org(&pool, "mine").await;
        let id = schedule(&pool, mine).await;
        let repo = ReportScheduleRepository::new(pool.clone());
        let queued = repo
            .queue_run_scoped(&pool, id, mine)
            .await
            .unwrap()
            .unwrap();

        let worker = ReportWorker::new(
            pool.clone(),
            None,
            EmailService::new("http://localhost".into(), false),
        );
        worker.process_scheduled_executions(10).await.unwrap();

        let (status, code): (String, Option<String>) =
            sqlx::query_as("SELECT status, error_code FROM report_executions WHERE id = $1")
                .bind(queued.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "failed");
        assert_eq!(code.as_deref(), Some("STORAGE_NOT_CONFIGURED"));
    }
}
