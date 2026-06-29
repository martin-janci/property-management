//! Form repository for Epic 54.
//!
//! Handles all database operations for forms, fields, and submissions.
//!
//! # RLS Integration (PAP-67 / PAP-76)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the form tables (`forms`, `form_fields`,
//! `form_submissions`, `form_downloads`). Under `FORCE` the api-server's owner
//! connection is no longer exempt, so a query issued on a connection WITHOUT
//! `app.current_org_id` set collapses to deny-all (own-org reads return empty,
//! writes fail) — and on a BYPASSRLS/superuser pool the same query would run
//! completely unscoped.
//!
//! Every method therefore takes an **executor whose connection already has RLS
//! context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. Multi-statement methods
//! (those that fire more than one query, e.g. a count plus a page fetch, or an
//! INSERT that fans out into child-row INSERTs) take a `&mut PgConnection` so
//! every statement runs on the *same* context-bearing connection. The
//! repository holds **no pool**, so there is no way to issue a query that
//! bypasses RLS. This mirrors the `work_order.rs` (PAP-179) /
//! `budget.rs` (PAP-180) precedent.
//!
//! # Module layout
//!
//! The repository surface is large (form CRUD, fields, submissions, statistics,
//! download tracking and available-forms lookups). To keep each area readable
//! and to reduce the churn surface of any single file, the
//! `impl FormRepository` block is split across cohesive sub-modules. Every
//! sub-module adds methods to the *same* [`FormRepository`] struct defined here
//! (same pattern as the `document/` / `sensor/` / `subscription/` repository
//! splits):
//!
//! - [`forms`]       — form CRUD, listing, publish/archive
//! - [`fields`]      — form-field CRUD & reordering
//! - [`submissions`] — submission create/get/list/review
//! - [`statistics`]  — organization form statistics
//! - [`downloads`]   — download tracking
//! - [`available`]   — published forms available to a user

use sqlx::PgPool;

mod available;
mod downloads;
mod fields;
mod forms;
mod statistics;
mod submissions;

/// Repository for form-related database operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`) query.
#[derive(Clone)]
pub struct FormRepository;

impl FormRepository {
    /// Creates a new form repository.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set connection
    /// supplied by the handler's `RlsConnection`).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }
}
