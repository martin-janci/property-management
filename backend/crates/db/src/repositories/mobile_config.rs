//! Mobile config repository.
//!
//! Backs the admin-console mobile-config Save flow
//! (`PATCH /api/v1/admin/mobile-config`). Reads/writes the platform-global
//! singleton `mobile_config` row (migration 00230): force-update version
//! floors plus React Native feature flags.
//!
//! Uses runtime `sqlx::query_as` (no compile-time-checked `query!` macros), so
//! it compiles without offline query metadata — matching the sibling
//! `health_monitoring` repository.

use crate::DbPool;
use chrono::{DateTime, Utc};
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// The platform-global mobile configuration singleton row.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct MobileConfig {
    /// Minimum supported iOS app version (force-update floor). `None` = unset.
    pub min_ios_version: Option<String>,
    /// Minimum supported Android app version (force-update floor). `None` = unset.
    pub min_android_version: Option<String>,
    /// React Native feature flags, a JSON object of `flag-key -> value`.
    #[schema(value_type = Object)]
    pub flags: serde_json::Value,
    /// When the config was last written.
    pub updated_at: DateTime<Utc>,
    /// Operator who last wrote the config (`None` after operator deletion).
    pub updated_by: Option<Uuid>,
}

/// A partial update to the mobile config. Only `Some` fields are applied.
///
/// Version fields follow PATCH semantics: `None` leaves the stored value
/// unchanged; `Some("")` (or whitespace-only) clears it to `NULL`; any other
/// `Some(v)` sets it. `flags`, when present, is a JSON object whose keys are
/// merged into the stored flags (existing keys not present in the patch are
/// preserved).
#[derive(Debug, Default, Clone)]
pub struct MobileConfigPatch {
    pub min_ios_version: Option<String>,
    pub min_android_version: Option<String>,
    pub flags: Option<serde_json::Value>,
}

/// Repository for the mobile-config singleton.
#[derive(Clone)]
pub struct MobileConfigRepository {
    pool: DbPool,
}

/// Normalize a version string: trim; treat empty/whitespace-only as "clear"
/// (`None`).
fn normalize_version(v: String) -> Option<String> {
    let t = v.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

impl MobileConfigRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Read the current mobile config. The singleton row is seeded by the
    /// migration, but this returns `None` if it is somehow absent so callers
    /// can decide on a default.
    pub async fn get(&self) -> Result<Option<MobileConfig>, SqlxError> {
        sqlx::query_as::<_, MobileConfig>(
            r#"
            SELECT min_ios_version, min_android_version, flags, updated_at, updated_by
            FROM mobile_config
            WHERE id = TRUE
            "#,
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Apply a partial update and return the full, updated config.
    ///
    /// Runs in a transaction with `SELECT … FOR UPDATE` so concurrent patches
    /// (e.g. the two admin sub-forms saving at once) merge without lost
    /// updates.
    pub async fn patch(
        &self,
        patch: MobileConfigPatch,
        actor: Option<Uuid>,
    ) -> Result<MobileConfig, SqlxError> {
        let mut tx = self.pool.begin().await?;

        let current = sqlx::query_as::<_, MobileConfig>(
            r#"
            SELECT min_ios_version, min_android_version, flags, updated_at, updated_by
            FROM mobile_config
            WHERE id = TRUE
            FOR UPDATE
            "#,
        )
        .fetch_optional(&mut *tx)
        .await?;

        let (cur_ios, cur_android, mut flags) = match current {
            Some(c) => (
                c.min_ios_version,
                c.min_android_version,
                c.flags.as_object().cloned().unwrap_or_default(),
            ),
            None => (None, None, serde_json::Map::new()),
        };

        let min_ios = match patch.min_ios_version {
            Some(v) => normalize_version(v),
            None => cur_ios,
        };
        let min_android = match patch.min_android_version {
            Some(v) => normalize_version(v),
            None => cur_android,
        };

        // Merge provided flag keys into the stored object.
        if let Some(serde_json::Value::Object(incoming)) = patch.flags {
            for (k, v) in incoming {
                flags.insert(k, v);
            }
        }
        let flags_value = serde_json::Value::Object(flags);

        let row = sqlx::query_as::<_, MobileConfig>(
            r#"
            INSERT INTO mobile_config
                (id, min_ios_version, min_android_version, flags, updated_by, updated_at)
            VALUES (TRUE, $1, $2, $3, $4, NOW())
            ON CONFLICT (id) DO UPDATE SET
                min_ios_version = EXCLUDED.min_ios_version,
                min_android_version = EXCLUDED.min_android_version,
                flags = EXCLUDED.flags,
                updated_by = EXCLUDED.updated_by,
                updated_at = NOW()
            RETURNING min_ios_version, min_android_version, flags, updated_at, updated_by
            "#,
        )
        .bind(min_ios)
        .bind(min_android)
        .bind(flags_value)
        .bind(actor)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row)
    }
}
