//! Restore a tenant export tarball into a NEW org id.
//!
//! Phase 5.5 — Tenant Lifecycle & Operability.
//! Defense for brainstorming leak #16: restore is **never in place**. The
//! restored tenant always lands as a fresh `organization_id` so an
//! accidental restore cannot trample over a still-active org.
//!
//! Strategy
//! --------
//! 1. Read `metadata.json` to discover the original org id.
//! 2. Mint a NEW `Uuid::new_v4()` for the restored org. Build a remap
//!    `old_org_id -> new_org_id`.
//! 3. Restore `tables/organizations.ndjson` first, rewriting `id` to the
//!    new uuid (and making sure `slug` doesn't collide — append a suffix
//!    on conflict).
//! 4. For every per-table NDJSON, rewrite the `organization_id` /  `org_id`
//!    column to the new uuid and INSERT row-by-row. Other UUID columns
//!    (FKs to children of the same tenant) keep their original values —
//!    the export-then-restore round-trip preserves intra-tenant FKs.

use crate::errors::{TenantOpsError, TenantOpsResult};
use crate::export::TenantExportMetadata;
use crate::manifest::TenantDataManifest;
use db::DbPool;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreReport {
    pub original_organization_id: Uuid,
    pub new_organization_id: Uuid,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: chrono::DateTime<chrono::Utc>,
    pub rows_restored: u64,
    pub tables_restored: u32,
    pub manifest_version_at_export: u32,
    pub manifest_version_now: u32,
}

/// Restore a tarball into a fresh org id.
pub async fn restore_tenant_export(
    pool: &DbPool,
    tarball_path: impl AsRef<Path>,
    current_manifest: &TenantDataManifest,
) -> TenantOpsResult<RestoreReport> {
    let started_at = chrono::Utc::now();

    // Read everything into memory — exports are small (one tenant).
    let raw = std::fs::read(tarball_path.as_ref())?;
    let gz = GzDecoder::new(&raw[..]);
    let mut tar = tar::Archive::new(gz);

    let mut metadata: Option<TenantExportMetadata> = None;
    let mut export_manifest: Option<TenantDataManifest> = None;
    let mut tables: HashMap<String, Vec<u8>> = HashMap::new();

    for entry in tar.entries().map_err(|e| TenantOpsError::Tarball(e.to_string()))? {
        let mut entry = entry.map_err(|e| TenantOpsError::Tarball(e.to_string()))?;
        let path = entry
            .path()
            .map_err(|e| TenantOpsError::Tarball(e.to_string()))?
            .to_path_buf();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;

        let pstr = path.to_string_lossy().to_string();
        if pstr == "metadata.json" {
            metadata = Some(serde_json::from_slice(&buf)?);
        } else if pstr == "manifest.json" {
            export_manifest = Some(serde_json::from_slice(&buf)?);
        } else if let Some(rest) = pstr.strip_prefix("tables/") {
            if let Some(name) = rest.strip_suffix(".ndjson") {
                tables.insert(name.to_string(), buf);
            }
        }
    }

    let metadata = metadata.ok_or_else(|| {
        TenantOpsError::RestoreInvalid("missing metadata.json".into())
    })?;
    let export_manifest = export_manifest.ok_or_else(|| {
        TenantOpsError::RestoreInvalid("missing manifest.json".into())
    })?;

    let original_org_id = metadata.organization_id;
    let new_org_id = Uuid::new_v4();

    let mut conn = pool.acquire().await?;
    sqlx::query("SELECT set_request_context(NULL::uuid, NULL::uuid, TRUE)")
        .execute(&mut *conn)
        .await?;

    // 1. Restore `organizations` first, rewriting id to new_org_id and
    //    de-duping the slug.
    let mut rows_restored: u64 = 0;
    let mut tables_restored: u32 = 0;

    let org_bytes = tables
        .remove("organizations")
        .ok_or_else(|| TenantOpsError::RestoreInvalid("missing organizations.ndjson".into()))?;
    let org_rows = parse_ndjson(&org_bytes)?;
    if org_rows.len() != 1 {
        return Err(TenantOpsError::RestoreInvalid(format!(
            "organizations.ndjson must have exactly 1 row, found {}",
            org_rows.len()
        )));
    }
    let mut org_row = org_rows.into_iter().next().unwrap();
    if let Value::Object(ref mut m) = org_row {
        m.insert("id".to_string(), Value::String(new_org_id.to_string()));
        if let Some(Value::String(slug)) = m.get("slug").cloned() {
            // Append a 6-char hex suffix to avoid UNIQUE collision.
            let sfx = &new_org_id.to_string()[..6];
            m.insert(
                "slug".to_string(),
                Value::String(format!("{}-restored-{}", slug, sfx)),
            );
        }
        // Clear deleted_at on restore — restored orgs are alive.
        m.insert("deleted_at".to_string(), Value::Null);
    }
    insert_jsonb_row(&mut *conn, "organizations", &org_row).await?;
    rows_restored += 1;
    tables_restored += 1;

    // 2. Restore each per-table NDJSON, rewriting the org column.
    for tbl in current_manifest.own_org_tables() {
        let bytes = match tables.remove(&tbl.table) {
            Some(b) => b,
            None => continue, // table was empty in the export
        };
        let rows = parse_ndjson(&bytes)?;
        for mut row in rows {
            if let Value::Object(ref mut m) = row {
                m.insert(
                    tbl.org_column.clone(),
                    Value::String(new_org_id.to_string()),
                );
            }
            insert_jsonb_row(&mut *conn, &tbl.table, &row).await?;
            rows_restored += 1;
        }
        tables_restored += 1;
    }

    Ok(RestoreReport {
        original_organization_id: original_org_id,
        new_organization_id: new_org_id,
        started_at,
        finished_at: chrono::Utc::now(),
        rows_restored,
        tables_restored,
        manifest_version_at_export: export_manifest.manifest_version,
        manifest_version_now: current_manifest.manifest_version,
    })
}

fn parse_ndjson(bytes: &[u8]) -> TenantOpsResult<Vec<Value>> {
    let s = std::str::from_utf8(bytes)
        .map_err(|e| TenantOpsError::RestoreInvalid(format!("ndjson utf8: {e}")))?;
    let mut out = Vec::new();
    for line in s.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line)?;
        out.push(v);
    }
    Ok(out)
}

/// INSERT one JSONB row into a table. Uses `jsonb_populate_record` to
/// avoid having to know the table's column types statically.
async fn insert_jsonb_row<'c, E>(
    conn: E,
    table: &str,
    row: &Value,
) -> TenantOpsResult<()>
where
    E: sqlx::PgExecutor<'c>,
{
    let sql = format!(
        "INSERT INTO {} SELECT * FROM jsonb_populate_record(NULL::{}, $1::jsonb)",
        sanitize_ident(table),
        sanitize_ident(table)
    );
    sqlx::query(&sql)
        .bind(row.to_string())
        .execute(conn)
        .await?;
    Ok(())
}

fn sanitize_ident(ident: &str) -> String {
    let cleaned: String = ident
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    assert!(
        !cleaned.is_empty(),
        "empty identifier after sanitization: {:?}",
        ident
    );
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ndjson_handles_blank_lines() {
        let s = b"{\"a\":1}\n\n{\"b\":2}\n";
        let v = parse_ndjson(s).unwrap();
        assert_eq!(v.len(), 2);
    }
}
