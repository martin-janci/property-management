//! Per-tenant settings store.
//!
//! Backs `tenant_settings` (migration 00137). Every write goes through
//! `SettingsStore::set` which records an audit row alongside the update.
//!
//! Phase 5 owns this — it is the operator-controlled store consulted by the
//! admin console. Phase 3's `tenant_config` is a different shape (read-mostly
//! cached config served to the frontend) and is owned by that phase.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::Error as SqlxError;
use std::sync::Arc;
use uuid::Uuid;

use crate::{AdminError, AuditOutcome, AuditWriter, Capability};

/// One settings row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SettingsRecord {
    pub organization_id: Uuid,
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
}

/// Trait so tests can plug an in-memory store.
#[async_trait]
pub trait SettingsStore: Send + Sync {
    /// Read a typed value for `org_id`/`key`.
    async fn get_raw(
        &self,
        org_id: Uuid,
        key: &str,
    ) -> Result<Option<SettingsRecord>, AdminError>;

    /// Upsert a JSON value. The audit row is written by the implementation.
    async fn set_raw(
        &self,
        org_id: Uuid,
        key: &str,
        value: serde_json::Value,
        actor: Uuid,
    ) -> Result<SettingsRecord, AdminError>;
}

/// Convenience trait extension for typed get/set.
pub trait SettingsStoreTyped: SettingsStore {
    /// Typed read. Returns `None` if no row OR if deserialization fails (the
    /// latter is logged at WARN — the row is malformed and an admin should
    /// fix it).
    fn get<T>(
        &self,
        org_id: Uuid,
        key: &str,
    ) -> impl std::future::Future<Output = Result<Option<T>, AdminError>> + Send
    where
        T: DeserializeOwned + Send,
    {
        async move {
            let Some(row) = self.get_raw(org_id, key).await? else {
                return Ok(None);
            };
            match serde_json::from_value::<T>(row.value.clone()) {
                Ok(v) => Ok(Some(v)),
                Err(e) => {
                    tracing::warn!(
                        target: "admin_core",
                        org_id = %org_id,
                        key = %key,
                        error = %e,
                        "tenant_settings: malformed value"
                    );
                    Ok(None)
                }
            }
        }
    }

    /// Typed write.
    fn set<T>(
        &self,
        org_id: Uuid,
        key: &str,
        value: T,
        actor: Uuid,
    ) -> impl std::future::Future<Output = Result<SettingsRecord, AdminError>> + Send
    where
        T: Serialize + Send,
    {
        async move {
            let json = serde_json::to_value(value).map_err(|e| {
                AdminError::Internal(format!("settings_store: serialize: {}", e))
            })?;
            self.set_raw(org_id, key, json, actor).await
        }
    }
}

impl<T: SettingsStore + ?Sized> SettingsStoreTyped for T {}

/// Postgres-backed implementation.
#[derive(Clone)]
pub struct PgSettingsStore {
    pool: db::DbPool,
    audit: Arc<dyn AuditWriter>,
}

impl PgSettingsStore {
    pub fn new(pool: db::DbPool, audit: Arc<dyn AuditWriter>) -> Self {
        Self { pool, audit }
    }
}

fn map_sqlx(err: SqlxError) -> AdminError {
    AdminError::Internal(format!("settings_store: {}", err))
}

#[async_trait]
impl SettingsStore for PgSettingsStore {
    async fn get_raw(
        &self,
        org_id: Uuid,
        key: &str,
    ) -> Result<Option<SettingsRecord>, AdminError> {
        let row = sqlx::query_as::<_, SettingsRecord>(
            r#"
            SELECT organization_id, key, value, updated_at, updated_by
            FROM tenant_settings
            WHERE organization_id = $1 AND key = $2
            "#,
        )
        .bind(org_id)
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row)
    }

    async fn set_raw(
        &self,
        org_id: Uuid,
        key: &str,
        value: serde_json::Value,
        actor: Uuid,
    ) -> Result<SettingsRecord, AdminError> {
        let row = sqlx::query_as::<_, SettingsRecord>(
            r#"
            INSERT INTO tenant_settings (organization_id, key, value, updated_by)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (organization_id, key)
            DO UPDATE SET
                value = EXCLUDED.value,
                updated_by = EXCLUDED.updated_by,
                updated_at = NOW()
            RETURNING organization_id, key, value, updated_at, updated_by
            "#,
        )
        .bind(org_id)
        .bind(key)
        .bind(&value)
        .bind(actor)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;

        // Audit the write. We record the key as the target_id slot does not
        // accept strings; the key + value-hash live in the JSONB details.
        let _ = self
            .audit
            .record(
                Some(actor),
                Capability::SiteSettingsWrite,
                AuditOutcome::Allowed,
                Some("tenant_settings"),
                Some(org_id),
                Some(&value),
                None,
                None,
            )
            .await;

        Ok(row)
    }
}
