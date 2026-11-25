//! Resident person-month submissions (UC-10).
//!
//! These methods are split out of [`super::person_month`] because they query
//! `person_month_submissions`, which is `FORCE ROW LEVEL SECURITY` (migration
//! 00251). Every one of them runs on the caller-supplied RLS connection so the
//! request's tenant context (`app.current_user_id` / `app.current_org_id`) is
//! in scope — the repository's own raw pool is never used for these tables.
//! Keeping them out of `person_month.rs` also keeps that file's raw-pool
//! `person_months` readers (not FORCE-RLS) separate from the FORCE-RLS
//! submission SQL, so the RLS enforcement lint can tell the two apart.

use crate::models::person_month::{PersonMonthSubmission, PersonMonthSubmissionWithUnit};
use crate::repositories::PersonMonthRepository;
use sqlx::{postgres::PgConnection, Error as SqlxError};
use uuid::Uuid;

impl PersonMonthRepository {
    /// Whether `user_id` is a current (`end_date IS NULL`) resident of `unit_id`.
    ///
    /// Runs on the caller-supplied RLS connection so the request's tenant
    /// context is in scope (`unit_residents` is RLS-scoped). Pass
    /// `&mut **rls.conn()` from the handler.
    pub async fn is_active_resident(
        &self,
        conn: &mut PgConnection,
        unit_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, SqlxError> {
        sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM unit_residents \
             WHERE unit_id = $1 AND user_id = $2 AND end_date IS NULL)",
        )
        .bind(unit_id)
        .bind(user_id)
        .fetch_one(conn)
        .await
    }

    /// Create a pending submission, replacing any pending one for the same
    /// unit and month.
    ///
    /// Runs on the caller-supplied RLS connection so the request's tenant
    /// context is in scope — `person_month_submissions` is `FORCE ROW LEVEL
    /// SECURITY` (migration 00251), so the INSERT is checked against the
    /// `person_month_submissions_org` policy. Pass `&mut **rls.conn()`.
    #[allow(clippy::too_many_arguments)]
    pub async fn submit(
        &self,
        conn: &mut PgConnection,
        unit_id: Uuid,
        year: i32,
        month: i32,
        count: i32,
        note: Option<String>,
        user_id: Uuid,
    ) -> Result<PersonMonthSubmission, SqlxError> {
        sqlx::query_as::<_, PersonMonthSubmission>(
            r#"
            INSERT INTO person_month_submissions (unit_id, year, month, count, note, submitted_by)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (unit_id, year, month) WHERE status = 'pending'
            DO UPDATE SET
                count = EXCLUDED.count,
                note = EXCLUDED.note,
                submitted_by = EXCLUDED.submitted_by,
                submitted_at = NOW()
            RETURNING *
            "#,
        )
        .bind(unit_id)
        .bind(year)
        .bind(month)
        .bind(count)
        .bind(note)
        .bind(user_id)
        .fetch_one(conn)
        .await
    }

    /// Submissions for one unit in one year, newest first.
    ///
    /// Runs on the caller-supplied RLS connection (see [`Self::submit`]). Pass
    /// `&mut **rls.conn()`.
    pub async fn list_unit_submissions(
        &self,
        conn: &mut PgConnection,
        unit_id: Uuid,
        year: i32,
    ) -> Result<Vec<PersonMonthSubmission>, SqlxError> {
        sqlx::query_as::<_, PersonMonthSubmission>(
            "SELECT * FROM person_month_submissions WHERE unit_id = $1 AND year = $2 \
             ORDER BY submitted_at DESC",
        )
        .bind(unit_id)
        .bind(year)
        .fetch_all(conn)
        .await
    }

    /// Submissions for every unit of a building, optionally by status, with
    /// the unit designation, the submitter's email and the currently recorded
    /// count for comparison.
    ///
    /// Runs on the caller-supplied RLS connection (see [`Self::submit`]). Pass
    /// `&mut **rls.conn()`.
    pub async fn list_building_submissions(
        &self,
        conn: &mut PgConnection,
        building_id: Uuid,
        status: Option<&str>,
    ) -> Result<Vec<PersonMonthSubmissionWithUnit>, SqlxError> {
        sqlx::query_as::<_, PersonMonthSubmissionWithUnit>(
            r#"
            SELECT s.id, s.unit_id, u.designation AS unit_designation, s.year, s.month,
                   s.count, pm.count AS current_count, s.note, s.status, s.submitted_by,
                   usr.email AS submitter_email, s.submitted_at
            FROM person_month_submissions s
            JOIN units u ON u.id = s.unit_id
            LEFT JOIN person_months pm
                   ON pm.unit_id = s.unit_id AND pm.year = s.year AND pm.month = s.month
            LEFT JOIN users usr ON usr.id = s.submitted_by
            WHERE u.building_id = $1
              AND ($2::text IS NULL OR s.status = $2)
            ORDER BY s.submitted_at DESC
            LIMIT 500
            "#,
        )
        .bind(building_id)
        .bind(status)
        .fetch_all(conn)
        .await
    }

    /// Find a submission by id.
    ///
    /// Runs on the caller-supplied RLS connection (see [`Self::submit`]). Pass
    /// `&mut **rls.conn()`.
    pub async fn find_submission(
        &self,
        conn: &mut PgConnection,
        id: Uuid,
    ) -> Result<Option<PersonMonthSubmission>, SqlxError> {
        sqlx::query_as::<_, PersonMonthSubmission>(
            "SELECT * FROM person_month_submissions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(conn)
        .await
    }

    /// Move a pending submission to `status` (approved/rejected). Returns
    /// `None` when it is no longer pending, so a decision is applied once.
    /// Runs on the caller's connection so it can share a transaction with the
    /// `person_months` write on approval.
    pub async fn decide_submission(
        &self,
        conn: &mut PgConnection,
        id: Uuid,
        status: &str,
        decided_by: Uuid,
        decision_note: Option<String>,
    ) -> Result<Option<PersonMonthSubmission>, SqlxError> {
        sqlx::query_as::<_, PersonMonthSubmission>(
            r#"
            UPDATE person_month_submissions
            SET status = $2, decided_by = $3, decided_at = NOW(), decision_note = $4
            WHERE id = $1 AND status = 'pending'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(decided_by)
        .bind(decision_note)
        .fetch_optional(conn)
        .await
    }
}
