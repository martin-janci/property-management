//! CROSS-CUTTING audit-entry builder (EPIC-ACC-16.7 / 01.10). OWNED BY: BE-Platform.
//!
//! Mutating handlers across every epic build an audit entry here and persist it
//! via `db::repositories::acc_platform::AccPlatformRepository::record_audit_rls`
//! inside the SAME transaction as the mutation (who/what/when/old->new).
//! Keep the builder API STABLE — other coders depend on it.
//!
//! accounting-core depends on `db` for the model type, so the builder yields a
//! ready-to-insert [`db::models::acc_platform::AccAuditLog`] (its `id`/`created_at`
//! are placeholders the DB overrides on insert via defaults).
//!
//! ## Tamper-evidence (UC-ACC-16.7)
//! The `acc_audit_log` table is append-only at the RLS layer (00196 grants only
//! SELECT + INSERT, no UPDATE/DELETE policy, with FORCE RLS), so an entry once
//! written cannot be altered or removed even by the owning tenant. This builder
//! only *constructs* the row; durability + immutability are the DB's job. The
//! `diff` value carries the old→new snapshot (`{"old": …, "new": …}` by
//! convention) so a reviewer can reconstruct exactly what changed.

use crate::errors::Result;
use db::models::acc_platform::AccAuditLog;
use uuid::Uuid;

/// Convention key for the previous state inside an audit `diff`.
pub const DIFF_OLD: &str = "old";
/// Convention key for the new state inside an audit `diff`.
pub const DIFF_NEW: &str = "new";

/// Fluent builder for an audit entry. Use [`AuditBuilder::new`] then `.diff()`
/// and finally [`AuditBuilder::build`].
#[derive(Debug, Clone)]
pub struct AuditBuilder {
    tenant_id: Uuid,
    actor_id: Option<Uuid>,
    action: String,
    entity_type: String,
    entity_id: Option<Uuid>,
    diff: serde_json::Value,
}

impl AuditBuilder {
    /// Start an audit entry for `action` on `entity_type` within `tenant_id`.
    /// `action` is e.g. "create" | "update" | "delete" | "issue" | "send".
    /// (UC-ACC-16.7: records who=`actor_id`, what=`action`+`entity_type`.)
    pub fn new(
        tenant_id: Uuid,
        actor_id: Option<Uuid>,
        action: impl Into<String>,
        entity_type: impl Into<String>,
    ) -> Self {
        Self {
            tenant_id,
            actor_id,
            action: action.into(),
            entity_type: entity_type.into(),
            entity_id: None,
            // Default to an empty object so the NOT NULL JSONB column is always
            // satisfied even when the caller records no field-level diff.
            diff: serde_json::json!({}),
        }
    }

    /// Attach the affected entity id (UC-ACC-16.7: identifies *what* changed).
    pub fn entity_id(mut self, id: Uuid) -> Self {
        self.entity_id = Some(id);
        self
    }

    /// Attach an old->new JSON diff. By convention shape it as
    /// `{"old": …, "new": …}` (see [`diff_of`]) so the audit-log view can render
    /// a before/after. (UC-ACC-16.7: who/what/when/**old→new**.)
    pub fn diff(mut self, diff: serde_json::Value) -> Self {
        self.diff = diff;
        self
    }

    /// Finalize into an insertable [`AccAuditLog`]. `id`/`created_at` are
    /// placeholders here; the DB overrides them via column defaults
    /// (`gen_random_uuid()` / `NOW()`) on INSERT.
    pub fn build(self) -> AccAuditLog {
        AccAuditLog {
            // DB-defaulted on insert; placeholder keeps the struct total.
            id: Uuid::nil(),
            tenant_id: self.tenant_id,
            actor_id: self.actor_id,
            action: self.action,
            entity_type: self.entity_type,
            entity_id: self.entity_id,
            diff: self.diff,
            // DB-defaulted to NOW() on insert; placeholder value is discarded.
            created_at: chrono::Utc::now(),
        }
    }
}

/// Convenience: build an audit entry in one call. `diff` should already be the
/// old→new JSON (use [`diff_of`] to build it). The caller persists the result
/// via `AccPlatformRepository::record_audit_rls` in the mutation's transaction.
/// (UC-ACC-16.7)
pub fn record(
    tenant_id: Uuid,
    actor_id: Option<Uuid>,
    action: impl Into<String>,
    entity_type: impl Into<String>,
    entity_id: Option<Uuid>,
    diff: serde_json::Value,
) -> Result<AccAuditLog> {
    let mut b = AuditBuilder::new(tenant_id, actor_id, action, entity_type).diff(diff);
    if let Some(id) = entity_id {
        b = b.entity_id(id);
    }
    Ok(b.build())
}

/// Helper to shape an old→new diff in the canonical `{"old": …, "new": …}` form.
/// `old` is `None` for creates; `new` is `None` for deletes. (UC-ACC-16.7)
pub fn diff_of(
    old: Option<serde_json::Value>,
    new: Option<serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        DIFF_OLD: old.unwrap_or(serde_json::Value::Null),
        DIFF_NEW: new.unwrap_or(serde_json::Value::Null),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // UC-ACC-16.7: an entry records who/what/entity and the old→new diff.
    #[test]
    fn builder_records_who_what_diff() {
        let tenant = Uuid::new_v4();
        let actor = Uuid::new_v4();
        let entity = Uuid::new_v4();
        let entry = AuditBuilder::new(tenant, Some(actor), "update", "invoice")
            .entity_id(entity)
            .diff(diff_of(
                Some(serde_json::json!({ "status": "draft" })),
                Some(serde_json::json!({ "status": "issued" })),
            ))
            .build();

        assert_eq!(entry.tenant_id, tenant);
        assert_eq!(entry.actor_id, Some(actor));
        assert_eq!(entry.action, "update");
        assert_eq!(entry.entity_type, "invoice");
        assert_eq!(entry.entity_id, Some(entity));
        assert_eq!(entry.diff[DIFF_OLD]["status"], "draft");
        assert_eq!(entry.diff[DIFF_NEW]["status"], "issued");
    }

    // No-diff case still produces a valid (NOT NULL) JSONB object.
    #[test]
    fn default_diff_is_object() {
        let entry = AuditBuilder::new(Uuid::new_v4(), None, "delete", "contact").build();
        assert!(entry.diff.is_object());
        assert!(entry.entity_id.is_none());
    }

    // The one-call convenience mirrors the builder.
    #[test]
    fn record_matches_builder() {
        let tenant = Uuid::new_v4();
        let entity = Uuid::new_v4();
        let entry = record(
            tenant,
            None,
            "create",
            "share_link",
            Some(entity),
            diff_of(None, Some(serde_json::json!({ "token": "***" }))),
        )
        .unwrap();
        assert_eq!(entry.action, "create");
        assert_eq!(entry.entity_id, Some(entity));
        assert!(entry.diff[DIFF_OLD].is_null());
        assert_eq!(entry.diff[DIFF_NEW]["token"], "***");
    }
}
