# Layout Control Plane Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Layout & Content Manager control plane — DB tables, repository, superadmin + tenant routes in api-server, resolved-layout endpoints in api-server and reality-server, and the TypeSpec design artifact — on top of the `layout-core` contract crate.

**Architecture:** Global platform-admin-owned tables (`layout_configs`, `layout_config_versions`, `layout_kill_flags`, `layout_registry_manifests` — no RLS, precedent `feature_flags`/00030) plus one org-scoped RLS table (`layout_tenant_overrides`). A stateless executor-based `LayoutRepository` in the `db` crate. api-server exposes: platform-admin CRUD/publish/rollback/kill/manifests (auth via `extract_super_admin_token`, unscoped pool), tenant override read/save (org-admin via `TenantExtractor` + `RlsConnection`, gated by `layout_core::validate_tenant_override`), and resolved `GET` (any authenticated org user, `layout_core::resolve`). reality-server exposes a public resolved `GET` (no tenant layer, `acquire_public_conn`). Configs are stored as JSONB and validated by `layout-core` at the publish/save gates.

**Tech Stack:** Rust (axum 0.8, sqlx 0.9 runtime queries, utoipa annotations), `layout-core` crate (this repo, PR #2424), TypeSpec (design artifact only).

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-19-layout-content-manager-design.md` §5 (kill), §6.3 (gates), §7 (placement table). Plan 1 (layout-core) is complete on branch `feature/layout-core-contract`.
- **Branch:** create `feature/layout-control-plane` from `feature/layout-core-contract` (stacked on PR #2424; the crate is a dependency).
- **Migration number 00220** — single migration for all five tables. `./scripts/check-migration-versions.sh` must pass (CI guard against duplicate numbers).
- **Runtime sqlx only** (`sqlx::query` / `query_as::<_, T>` with `.bind`); NO `sqlx::query!` macros (repo has no `.sqlx` offline cache; compile must stay DB-free). Because column typos compile fine here, every repo method MUST be covered by a `#[sqlx::test]` integration test.
- RLS: `layout_tenant_overrides` gets `ENABLE` + `FORCE ROW LEVEL SECURITY` + the standard `tenant_isolation` policy using `is_super_admin() OR organization_id = get_current_org_id()`. The other four tables get NO RLS, with a header comment justifying it (platform-admin-owned global config; precedent `feature_flags` 00030).
- Repository style: stateless (`#[derive(Clone, Default)]`, no pool field), every method takes `E: Executor<'e, Database = Postgres>`; transaction-needing methods (`publish`, `rollback`) take `&mut PgConnection`. Errors: `sqlx::Error` (crate re-export `SqlxError`). Models in `backend/crates/db/src/models/layout.rs`.
- Kill state lives in `layout_kill_flags`, OUTSIDE config versions — publish/rollback must never change kill state (spec §5).
- Screen IDs contain `/` (e.g. `reality/listing-detail`): admin/tenant endpoints carry `screen` in the query string or JSON body; only resolved endpoints use an axum catch-all `/{*screen}` route.
- utoipa: annotate every handler with `#[utoipa::path]`, but do NOT register in either server's `ApiDoc` (consistent with forms/announcements; registration is a follow-up).
- **ADAPT rule:** code blocks in route tasks are the exact intent; the implementer may adapt ONLY import paths and extractor/state field names to match the mirrored files (`src/routes/platform_admin/features.rs`, `src/routes/announcements/lifecycle.rs`, `src/routes/forms/crud.rs`) if a name differs — never the logic, routes, or validation calls. Any such adaptation must be listed in the task report.
- Commit scopes per repo convention: `feat(db)`, `feat(api-server)`, `feat(reality-server)`, `docs(api)`, `docs(repo-map)`.
- Test DB: `#[sqlx::test(migrator = "db::MIGRATOR")]` needs a local Postgres; use `DATABASE_URL=postgres://postgres:postgres@localhost:5433/postgres` (local PG runs on :5433; adjust credentials only if that URL fails, and report what worked).
- Pre-push gate: `cd backend && cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p layout-core && DATABASE_URL=… cargo test -p db --test layout_repo_tests`.

## File Structure

```
backend/crates/db/migrations/00220_create_layout_tables.sql
backend/crates/db/src/models/layout.rs          # row structs
backend/crates/db/src/models/mod.rs             # + pub mod layout;
backend/crates/db/src/repositories/layout.rs    # LayoutRepository (stateless)
backend/crates/db/src/repositories/mod.rs       # + pub mod / pub use
backend/crates/db/tests/layout_repo_tests.rs    # sqlx::test integration tests
backend/servers/api-server/src/routes/layout/mod.rs       # routers + module wiring
backend/servers/api-server/src/routes/layout/types.rs     # request/response DTOs
backend/servers/api-server/src/routes/layout/admin.rs     # platform-admin handlers
backend/servers/api-server/src/routes/layout/tenant.rs    # tenant override handlers
backend/servers/api-server/src/routes/layout/resolved.rs  # resolved GET
backend/servers/api-server/src/routes/mod.rs    # + pub mod layout;
backend/servers/api-server/src/lib.rs           # route_table() nests
backend/servers/reality-server/src/routes/layout.rs  # public resolved GET
backend/servers/reality-server/src/routes/mod.rs     # + pub mod layout;
backend/servers/reality-server/src/main.rs           # nest
docs/api/typespec/domains/layout.tsp            # design artifact
docs/api/typespec/main.tsp                      # + import
docs/api/generated/openapi.yaml                 # regenerated
docs/repo-map.md                                # layout rows in route/repo tables
```

---

### Task 1: Migration 00220 + models + repository core (draft CRUD)

**Files:**
- Create: `backend/crates/db/migrations/00220_create_layout_tables.sql`
- Create: `backend/crates/db/src/models/layout.rs`
- Modify: `backend/crates/db/src/models/mod.rs` (add `pub mod layout;` alphabetically)
- Create: `backend/crates/db/src/repositories/layout.rs`
- Modify: `backend/crates/db/src/repositories/mod.rs` (add `pub mod layout;` + `pub use layout::LayoutRepository;` in the existing alphabetical style)
- Test: `backend/crates/db/tests/layout_repo_tests.rs`

**Interfaces:**
- Consumes: `db::MIGRATOR`, existing migration helpers (`is_super_admin()`, `get_current_org_id()` from 00006).
- Produces (later tasks rely on these exact names): models `LayoutConfigRow`, `LayoutConfigVersionRow`, `LayoutTenantOverrideRow`, `LayoutKillFlagRow`, `LayoutRegistryManifestRow`; `LayoutRepository::{new, get_config, list_configs, upsert_draft, set_rails}` with signatures as written below.

- [ ] **Step 1: Verify the migration number is still free**

Run: `ls backend/crates/db/migrations/ | sort | tail -3`
Expected: highest existing prefix is `00219`. If a `00220_*` appeared (dev moved), renumber this plan's migration to the next free number and use that number everywhere below; report the substitution.

- [ ] **Step 2: Write the migration**

`backend/crates/db/migrations/00220_create_layout_tables.sql`:

```sql
-- Layout & Content Manager control plane (spec: docs/superpowers/specs/2026-07-19-layout-content-manager-design.md §7)
--
-- layout_configs / layout_config_versions / layout_kill_flags / layout_registry_manifests
-- are platform-admin-owned GLOBAL tables: no RLS by design (precedent: feature_flags,
-- migration 00030). Access control is enforced at the application layer (superadmin
-- routes); the resolved read path serves them to all authenticated users.
--
-- layout_tenant_overrides is org-scoped: ENABLE + FORCE RLS with the standard
-- tenant-isolation policy.

CREATE TABLE IF NOT EXISTS layout_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    screen TEXT NOT NULL UNIQUE,
    draft JSONB NOT NULL DEFAULT '{}'::jsonb,
    published JSONB,
    published_version INTEGER NOT NULL DEFAULT 0,
    rails JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS layout_config_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    screen TEXT NOT NULL,
    version INTEGER NOT NULL,
    config JSONB NOT NULL,
    published_by UUID,
    published_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (screen, version)
);

CREATE INDEX IF NOT EXISTS idx_layout_config_versions_screen
    ON layout_config_versions (screen, version DESC);

CREATE TABLE IF NOT EXISTS layout_kill_flags (
    screen TEXT NOT NULL,
    section_type TEXT NOT NULL,
    killed_by UUID,
    killed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (screen, section_type)
);

CREATE TABLE IF NOT EXISTS layout_registry_manifests (
    platform TEXT PRIMARY KEY,
    manifest JSONB NOT NULL,
    updated_by UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS layout_tenant_overrides (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    screen TEXT NOT NULL,
    override_config JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, screen)
);

ALTER TABLE layout_tenant_overrides ENABLE ROW LEVEL SECURITY;
ALTER TABLE layout_tenant_overrides FORCE ROW LEVEL SECURITY;
CREATE POLICY layout_tenant_overrides_tenant_isolation ON layout_tenant_overrides
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());
```

Run: `./scripts/check-migration-versions.sh`
Expected: passes (no collision).

- [ ] **Step 3: Write the models**

`backend/crates/db/src/models/layout.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LayoutConfigRow {
    pub id: Uuid,
    pub screen: String,
    pub draft: serde_json::Value,
    pub published: Option<serde_json::Value>,
    pub published_version: i32,
    pub rails: serde_json::Value,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LayoutConfigVersionRow {
    pub id: Uuid,
    pub screen: String,
    pub version: i32,
    pub config: serde_json::Value,
    pub published_by: Option<Uuid>,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LayoutTenantOverrideRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub screen: String,
    pub override_config: serde_json::Value,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LayoutKillFlagRow {
    pub screen: String,
    pub section_type: String,
    pub killed_by: Option<Uuid>,
    pub killed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LayoutRegistryManifestRow {
    pub platform: String,
    pub manifest: serde_json::Value,
    pub updated_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
}
```

Register in `backend/crates/db/src/models/mod.rs`: add `pub mod layout;` in alphabetical position.

- [ ] **Step 4: Write the failing repo test (draft round-trip)**

`backend/crates/db/tests/layout_repo_tests.rs`:

```rust
use db::repositories::LayoutRepository;
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn draft_upsert_and_get_round_trip(pool: PgPool) {
    let repo = LayoutRepository::new();
    let mut conn = pool.acquire().await.unwrap();

    assert!(repo
        .get_config(&mut *conn, "reality/listing-detail")
        .await
        .unwrap()
        .is_none());

    let draft = json!({"screen": "reality/listing-detail", "version": 0,
                       "sections": [{"type": "gallery.v1"}]});
    let row = repo
        .upsert_draft(&mut *conn, "reality/listing-detail", &draft, None)
        .await
        .unwrap();
    assert_eq!(row.screen, "reality/listing-detail");
    assert_eq!(row.draft, draft);
    assert_eq!(row.published, None);
    assert_eq!(row.published_version, 0);

    // upsert overwrites the draft, keeps identity
    let draft2 = json!({"screen": "reality/listing-detail", "version": 0, "sections": []});
    let row2 = repo
        .upsert_draft(&mut *conn, "reality/listing-detail", &draft2, None)
        .await
        .unwrap();
    assert_eq!(row2.id, row.id);
    assert_eq!(row2.draft, draft2);

    let rails = json!({"reorderable": true});
    let row3 = repo
        .set_rails(&mut *conn, "reality/listing-detail", &rails, None)
        .await
        .unwrap();
    assert_eq!(row3.rails, rails);

    let all = repo.list_configs(&mut *conn).await.unwrap();
    assert_eq!(all.len(), 1);
}
```

- [ ] **Step 5: Run test to verify it fails**

Run: `cd backend && DATABASE_URL=postgres://postgres:postgres@localhost:5433/postgres cargo test -p db --test layout_repo_tests`
Expected: FAIL to compile — `LayoutRepository` not defined.

- [ ] **Step 6: Implement the repository core**

`backend/crates/db/src/repositories/layout.rs`:

```rust
use crate::models::layout::{
    LayoutConfigRow, LayoutConfigVersionRow, LayoutKillFlagRow, LayoutRegistryManifestRow,
    LayoutTenantOverrideRow,
};
use sqlx::{Error as SqlxError, Executor, PgConnection, Postgres};
use uuid::Uuid;

/// Stateless repository for the layout control plane. Global tables
/// (configs/versions/kills/manifests) are platform-admin-owned and carry no
/// RLS — call them with the unscoped pool. `layout_tenant_overrides` is
/// FORCE-RLS org-scoped — call those methods with an RLS-contexted executor.
#[derive(Clone, Default)]
pub struct LayoutRepository;

impl LayoutRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_config<'e, E>(
        &self,
        executor: E,
        screen: &str,
    ) -> Result<Option<LayoutConfigRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutConfigRow>(
            "SELECT id, screen, draft, published, published_version, rails, updated_by,
                    created_at, updated_at
             FROM layout_configs WHERE screen = $1",
        )
        .bind(screen)
        .fetch_optional(executor)
        .await
    }

    pub async fn list_configs<'e, E>(&self, executor: E) -> Result<Vec<LayoutConfigRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutConfigRow>(
            "SELECT id, screen, draft, published, published_version, rails, updated_by,
                    created_at, updated_at
             FROM layout_configs ORDER BY screen",
        )
        .fetch_all(executor)
        .await
    }

    pub async fn upsert_draft<'e, E>(
        &self,
        executor: E,
        screen: &str,
        draft: &serde_json::Value,
        updated_by: Option<Uuid>,
    ) -> Result<LayoutConfigRow, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutConfigRow>(
            "INSERT INTO layout_configs (screen, draft, updated_by)
             VALUES ($1, $2, $3)
             ON CONFLICT (screen)
             DO UPDATE SET draft = EXCLUDED.draft, updated_by = EXCLUDED.updated_by,
                           updated_at = now()
             RETURNING id, screen, draft, published, published_version, rails, updated_by,
                       created_at, updated_at",
        )
        .bind(screen)
        .bind(draft)
        .bind(updated_by)
        .fetch_one(executor)
        .await
    }

    pub async fn set_rails<'e, E>(
        &self,
        executor: E,
        screen: &str,
        rails: &serde_json::Value,
        updated_by: Option<Uuid>,
    ) -> Result<LayoutConfigRow, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutConfigRow>(
            "INSERT INTO layout_configs (screen, rails, updated_by)
             VALUES ($1, $2, $3)
             ON CONFLICT (screen)
             DO UPDATE SET rails = EXCLUDED.rails, updated_by = EXCLUDED.updated_by,
                           updated_at = now()
             RETURNING id, screen, draft, published, published_version, rails, updated_by,
                       created_at, updated_at",
        )
        .bind(screen)
        .bind(rails)
        .bind(updated_by)
        .fetch_one(executor)
        .await
    }
}
```

Register in `backend/crates/db/src/repositories/mod.rs`: `pub mod layout;` + `pub use layout::LayoutRepository;` (alphabetical, matching the existing one-mod-one-use style).

- [ ] **Step 7: Run test to verify it passes**

Run: `cd backend && DATABASE_URL=postgres://postgres:postgres@localhost:5433/postgres cargo test -p db --test layout_repo_tests`
Expected: PASS (1 test).

- [ ] **Step 8: Commit**

```bash
git add backend/crates/db scripts 2>/dev/null; git add backend/crates/db
git commit -m "feat(db): layout control-plane tables, models and repository core"
```

---

### Task 2: Repository publish / rollback / versions

**Files:**
- Modify: `backend/crates/db/src/repositories/layout.rs`
- Test: `backend/crates/db/tests/layout_repo_tests.rs`

**Interfaces:**
- Consumes: Task 1 repo + models.
- Produces: `publish(conn: &mut PgConnection, screen, published_by) -> Result<LayoutConfigRow, SqlxError>`; `rollback(conn: &mut PgConnection, screen, to_version: i32, published_by) -> Result<LayoutConfigRow, SqlxError>`; `list_versions<E>(executor, screen) -> Result<Vec<LayoutConfigVersionRow>, SqlxError>`. Publish semantics: copy `draft` → `published`, `published_version += 1`, snapshot into `layout_config_versions`. Rollback: copy the target version's config → `published` AND → `draft`, then snapshot as a NEW version (history immutable, monotonically increasing). `RowNotFound` when the screen/version doesn't exist.

- [ ] **Step 1: Write the failing tests**

Append to `backend/crates/db/tests/layout_repo_tests.rs`:

```rust
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publish_snapshots_versions_and_rollback_restores(pool: PgPool) {
    let repo = LayoutRepository::new();
    let mut conn = pool.acquire().await.unwrap();

    let v1 = json!({"screen": "ppt/dashboard", "version": 0,
                    "sections": [{"type": "kpi.v1"}]});
    repo.upsert_draft(&mut *conn, "ppt/dashboard", &v1, None)
        .await
        .unwrap();

    let published = repo.publish(&mut *conn, "ppt/dashboard", None).await.unwrap();
    assert_eq!(published.published_version, 1);
    assert_eq!(published.published, Some(v1.clone()));

    let v2 = json!({"screen": "ppt/dashboard", "version": 0,
                    "sections": [{"type": "kpi.v1"}, {"type": "news.v1"}]});
    repo.upsert_draft(&mut *conn, "ppt/dashboard", &v2, None)
        .await
        .unwrap();
    let published2 = repo.publish(&mut *conn, "ppt/dashboard", None).await.unwrap();
    assert_eq!(published2.published_version, 2);
    assert_eq!(published2.published, Some(v2.clone()));

    let versions = repo.list_versions(&mut *conn, "ppt/dashboard").await.unwrap();
    assert_eq!(
        versions.iter().map(|v| v.version).collect::<Vec<_>>(),
        vec![2, 1]
    );

    // rollback to v1: published AND draft become v1's config, history grows to 3
    let rolled = repo.rollback(&mut *conn, "ppt/dashboard", 1, None).await.unwrap();
    assert_eq!(rolled.published_version, 3);
    assert_eq!(rolled.published, Some(v1.clone()));
    assert_eq!(rolled.draft, v1);
    let versions = repo.list_versions(&mut *conn, "ppt/dashboard").await.unwrap();
    assert_eq!(versions.len(), 3);

    // unknown screen / version → RowNotFound
    assert!(matches!(
        repo.publish(&mut *conn, "no/such-screen", None).await,
        Err(sqlx::Error::RowNotFound)
    ));
    assert!(matches!(
        repo.rollback(&mut *conn, "ppt/dashboard", 99, None).await,
        Err(sqlx::Error::RowNotFound)
    ));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cd backend && DATABASE_URL=postgres://postgres:postgres@localhost:5433/postgres cargo test -p db --test layout_repo_tests`
Expected: FAIL to compile — `publish`/`rollback`/`list_versions` not defined.

- [ ] **Step 3: Implement**

Append to the `impl LayoutRepository` block:

```rust
    /// Publish the current draft: draft → published, version += 1, snapshot the
    /// version row. Runs in a transaction on the given connection.
    /// Kill flags are intentionally untouched (spec §5).
    pub async fn publish(
        &self,
        conn: &mut PgConnection,
        screen: &str,
        published_by: Option<Uuid>,
    ) -> Result<LayoutConfigRow, SqlxError> {
        let mut tx = sqlx::Connection::begin(conn).await?;
        let row = sqlx::query_as::<_, LayoutConfigRow>(
            "UPDATE layout_configs
             SET published = draft,
                 published_version = published_version + 1,
                 updated_by = $2,
                 updated_at = now()
             WHERE screen = $1
             RETURNING id, screen, draft, published, published_version, rails, updated_by,
                       created_at, updated_at",
        )
        .bind(screen)
        .bind(published_by)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(SqlxError::RowNotFound)?;
        sqlx::query(
            "INSERT INTO layout_config_versions (screen, version, config, published_by)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(screen)
        .bind(row.published_version)
        .bind(row.published.as_ref())
        .bind(published_by)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(row)
    }

    /// Roll back to a prior version: that version's config becomes both the
    /// published config and the draft, recorded as a NEW version (immutable,
    /// monotonically increasing history). Kill flags untouched (spec §5).
    pub async fn rollback(
        &self,
        conn: &mut PgConnection,
        screen: &str,
        to_version: i32,
        published_by: Option<Uuid>,
    ) -> Result<LayoutConfigRow, SqlxError> {
        let mut tx = sqlx::Connection::begin(conn).await?;
        let target = sqlx::query_as::<_, LayoutConfigVersionRow>(
            "SELECT id, screen, version, config, published_by, published_at
             FROM layout_config_versions WHERE screen = $1 AND version = $2",
        )
        .bind(screen)
        .bind(to_version)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(SqlxError::RowNotFound)?;
        let row = sqlx::query_as::<_, LayoutConfigRow>(
            "UPDATE layout_configs
             SET published = $2,
                 draft = $2,
                 published_version = published_version + 1,
                 updated_by = $3,
                 updated_at = now()
             WHERE screen = $1
             RETURNING id, screen, draft, published, published_version, rails, updated_by,
                       created_at, updated_at",
        )
        .bind(screen)
        .bind(&target.config)
        .bind(published_by)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(SqlxError::RowNotFound)?;
        sqlx::query(
            "INSERT INTO layout_config_versions (screen, version, config, published_by)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(screen)
        .bind(row.published_version)
        .bind(&target.config)
        .bind(published_by)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(row)
    }

    pub async fn list_versions<'e, E>(
        &self,
        executor: E,
        screen: &str,
    ) -> Result<Vec<LayoutConfigVersionRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutConfigVersionRow>(
            "SELECT id, screen, version, config, published_by, published_at
             FROM layout_config_versions WHERE screen = $1 ORDER BY version DESC",
        )
        .bind(screen)
        .fetch_all(executor)
        .await
    }
```

- [ ] **Step 4: Run to verify pass**

Run: `cd backend && DATABASE_URL=postgres://postgres:postgres@localhost:5433/postgres cargo test -p db --test layout_repo_tests`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add backend/crates/db
git commit -m "feat(db): layout publish, rollback and version history"
```

---

### Task 3: Repository kills, manifests, tenant overrides (RLS)

**Files:**
- Modify: `backend/crates/db/src/repositories/layout.rs`
- Test: `backend/crates/db/tests/layout_repo_tests.rs`

**Interfaces:**
- Consumes: Tasks 1–2.
- Produces: `kill<E>(executor, screen, section_type, killed_by)`, `unkill<E>(executor, screen, section_type) -> Result<bool, _>` (true if a row was removed), `list_kills<E>(executor, screen) -> Result<Vec<LayoutKillFlagRow>, _>`; `upsert_manifest<E>(executor, platform, manifest, updated_by) -> Result<LayoutRegistryManifestRow, _>`, `get_manifest<E>(executor, platform) -> Result<Option<…>, _>`, `list_manifests<E>(executor) -> Result<Vec<…>, _>`; `get_tenant_override<E>(executor, organization_id, screen) -> Result<Option<LayoutTenantOverrideRow>, _>`, `upsert_tenant_override<E>(executor, organization_id, screen, override_config, updated_by) -> Result<LayoutTenantOverrideRow, _>`. All plain-executor generics (no transactions needed).

- [ ] **Step 1: Write the failing tests**

Append to `layout_repo_tests.rs`. For the RLS test you need real `organizations` rows: **first look at an existing RLS repo test under `backend/crates/db/tests/` (e.g. the org-fixture helper used by other `*_rls_*` tests) and reuse its organization-creation helper verbatim.** If every existing helper is unusable, read `backend/crates/db/migrations/` for the `organizations` table definition and write a minimal `INSERT INTO organizations (…) VALUES (…)` fixture covering its NOT NULL columns; report which route you took.

```rust
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn kills_and_manifests_round_trip(pool: PgPool) {
    let repo = LayoutRepository::new();
    let mut conn = pool.acquire().await.unwrap();

    repo.kill(&mut *conn, "ppt/dashboard", "news.v1", None)
        .await
        .unwrap();
    // idempotent
    repo.kill(&mut *conn, "ppt/dashboard", "news.v1", None)
        .await
        .unwrap();
    let kills = repo.list_kills(&mut *conn, "ppt/dashboard").await.unwrap();
    assert_eq!(kills.len(), 1);
    assert_eq!(kills[0].section_type, "news.v1");

    assert!(repo.unkill(&mut *conn, "ppt/dashboard", "news.v1").await.unwrap());
    assert!(!repo.unkill(&mut *conn, "ppt/dashboard", "news.v1").await.unwrap());
    assert!(repo.list_kills(&mut *conn, "ppt/dashboard").await.unwrap().is_empty());

    let manifest = json!({"platform": "web", "components": {"kpi.v1": {"required": true}}});
    let row = repo
        .upsert_manifest(&mut *conn, "web", &manifest, None)
        .await
        .unwrap();
    assert_eq!(row.manifest, manifest);
    assert!(repo.get_manifest(&mut *conn, "mobile").await.unwrap().is_none());
    assert_eq!(repo.list_manifests(&mut *conn).await.unwrap().len(), 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn tenant_overrides_are_org_isolated(pool: PgPool) {
    let repo = LayoutRepository::new();

    // Create two orgs with the reused fixture helper (see Step 1 note), then:
    // org_a, org_b: Uuid of two organizations rows.

    let mut conn = pool.acquire().await.unwrap();
    db::tenant_context::set_request_context(&mut *conn, Some(org_a), None, false)
        .await
        .unwrap();
    let ov = json!({"sections": {"news.v1": {"visible": false}}});
    repo.upsert_tenant_override(&mut *conn, org_a, "ppt/dashboard", &ov, None)
        .await
        .unwrap();
    assert!(repo
        .get_tenant_override(&mut *conn, org_a, "ppt/dashboard")
        .await
        .unwrap()
        .is_some());

    // switch context to org_b: org_a's override is invisible (RLS)
    db::tenant_context::set_request_context(&mut *conn, Some(org_b), None, false)
        .await
        .unwrap();
    assert!(repo
        .get_tenant_override(&mut *conn, org_a, "ppt/dashboard")
        .await
        .unwrap()
        .is_none());
    db::tenant_context::clear_request_context(&mut *conn).await.unwrap();
}
```

(If `db::tenant_context::set_request_context` has a different public path or arity, mirror the exact call used by the existing RLS tests you took the fixture from — ADAPT rule; report it.)

- [ ] **Step 2: Run to verify failure** — same test command; expected compile failure on missing methods.

- [ ] **Step 3: Implement** — append to `impl LayoutRepository`:

```rust
    pub async fn kill<'e, E>(
        &self,
        executor: E,
        screen: &str,
        section_type: &str,
        killed_by: Option<Uuid>,
    ) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            "INSERT INTO layout_kill_flags (screen, section_type, killed_by)
             VALUES ($1, $2, $3)
             ON CONFLICT (screen, section_type) DO NOTHING",
        )
        .bind(screen)
        .bind(section_type)
        .bind(killed_by)
        .execute(executor)
        .await
        .map(|_| ())
    }

    pub async fn unkill<'e, E>(
        &self,
        executor: E,
        screen: &str,
        section_type: &str,
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let res = sqlx::query(
            "DELETE FROM layout_kill_flags WHERE screen = $1 AND section_type = $2",
        )
        .bind(screen)
        .bind(section_type)
        .execute(executor)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn list_kills<'e, E>(
        &self,
        executor: E,
        screen: &str,
    ) -> Result<Vec<LayoutKillFlagRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutKillFlagRow>(
            "SELECT screen, section_type, killed_by, killed_at
             FROM layout_kill_flags WHERE screen = $1 ORDER BY section_type",
        )
        .bind(screen)
        .fetch_all(executor)
        .await
    }

    pub async fn upsert_manifest<'e, E>(
        &self,
        executor: E,
        platform: &str,
        manifest: &serde_json::Value,
        updated_by: Option<Uuid>,
    ) -> Result<LayoutRegistryManifestRow, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutRegistryManifestRow>(
            "INSERT INTO layout_registry_manifests (platform, manifest, updated_by)
             VALUES ($1, $2, $3)
             ON CONFLICT (platform)
             DO UPDATE SET manifest = EXCLUDED.manifest, updated_by = EXCLUDED.updated_by,
                           updated_at = now()
             RETURNING platform, manifest, updated_by, updated_at",
        )
        .bind(platform)
        .bind(manifest)
        .bind(updated_by)
        .fetch_one(executor)
        .await
    }

    pub async fn get_manifest<'e, E>(
        &self,
        executor: E,
        platform: &str,
    ) -> Result<Option<LayoutRegistryManifestRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutRegistryManifestRow>(
            "SELECT platform, manifest, updated_by, updated_at
             FROM layout_registry_manifests WHERE platform = $1",
        )
        .bind(platform)
        .fetch_optional(executor)
        .await
    }

    pub async fn list_manifests<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<LayoutRegistryManifestRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutRegistryManifestRow>(
            "SELECT platform, manifest, updated_by, updated_at
             FROM layout_registry_manifests ORDER BY platform",
        )
        .fetch_all(executor)
        .await
    }

    pub async fn get_tenant_override<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        screen: &str,
    ) -> Result<Option<LayoutTenantOverrideRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutTenantOverrideRow>(
            "SELECT id, organization_id, screen, override_config, updated_by,
                    created_at, updated_at
             FROM layout_tenant_overrides
             WHERE organization_id = $1 AND screen = $2",
        )
        .bind(organization_id)
        .bind(screen)
        .fetch_optional(executor)
        .await
    }

    pub async fn upsert_tenant_override<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        screen: &str,
        override_config: &serde_json::Value,
        updated_by: Option<Uuid>,
    ) -> Result<LayoutTenantOverrideRow, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutTenantOverrideRow>(
            "INSERT INTO layout_tenant_overrides (organization_id, screen, override_config, updated_by)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (organization_id, screen)
             DO UPDATE SET override_config = EXCLUDED.override_config,
                           updated_by = EXCLUDED.updated_by, updated_at = now()
             RETURNING id, organization_id, screen, override_config, updated_by,
                       created_at, updated_at",
        )
        .bind(organization_id)
        .bind(screen)
        .bind(override_config)
        .bind(updated_by)
        .fetch_one(executor)
        .await
    }
```

- [ ] **Step 4: Run to verify pass** — same command; expected PASS (4 tests).
- [ ] **Step 5: Commit**

```bash
git add backend/crates/db
git commit -m "feat(db): layout kills, registry manifests and RLS tenant overrides"
```

---

### Task 4: api-server platform-admin layout routes (drafts, rails, manifests)

**Files:**
- Create: `backend/servers/api-server/src/routes/layout/mod.rs`
- Create: `backend/servers/api-server/src/routes/layout/types.rs`
- Create: `backend/servers/api-server/src/routes/layout/admin.rs`
- Modify: `backend/servers/api-server/src/routes/mod.rs` (`pub mod layout;`)
- Modify: `backend/servers/api-server/src/lib.rs` (`route_table()`: add `.nest("/api/v1/platform-admin/layout", routes::layout::admin_router())`)
- Add dependency: `backend/servers/api-server/Cargo.toml` → `layout-core = { path = "../../crates/layout-core" }` (and add `layout-core` to `[workspace.dependencies]` in `backend/Cargo.toml` as `layout-core = { path = "crates/layout-core" }`, referencing it with `workspace = true`, matching how sibling crates are declared — mirror the `db` dependency style; ADAPT to the file's actual style).

**Interfaces:**
- Consumes: `LayoutRepository` (Tasks 1–3); `extract_super_admin_token` from `crate::routes::platform_admin` (pub(crate)); `AppState` (`state.db` unscoped pool); `layout_core::{ScreenConfig, RegistryManifest, Rails}` for parse-validation.
- Produces: `admin_router() -> Router<AppState>` with: `GET /screens`, `GET /config?screen=`, `PUT /draft`, `PUT /rails`, `GET /manifests`, `PUT /manifests`. Publish/rollback/kill land in Task 5 (same module). DTOs in `types.rs` are shared with Tasks 5–6.

- [ ] **Step 1: DTOs**

`backend/servers/api-server/src/routes/layout/types.rs`:

```rust
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ScreenQuery {
    pub screen: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PutDraftRequest {
    pub screen: String,
    /// Must deserialize as layout_core::ScreenConfig.
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PutRailsRequest {
    pub screen: String,
    /// Must deserialize as layout_core::Rails.
    pub rails: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PutManifestRequest {
    /// "web" | "mobile"
    pub platform: String,
    /// Must deserialize as layout_core::RegistryManifest with matching platform.
    pub manifest: serde_json::Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ValidationErrorsResponse {
    pub errors: Vec<String>,
}
```

- [ ] **Step 2: Handlers**

`backend/servers/api-server/src/routes/layout/admin.rs` — mirror the auth/error idiom of `src/routes/platform_admin/features.rs` (ADAPT rule applies to the `ErrorResponse` import and `extract_super_admin_token` path only):

```rust
use crate::routes::platform_admin::extract_super_admin_token;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use db::repositories::LayoutRepository;

use super::types::{PutDraftRequest, PutManifestRequest, PutRailsRequest, ScreenQuery,
                   ValidationErrorsResponse};

fn bad_request(errors: Vec<String>) -> (StatusCode, Json<ValidationErrorsResponse>) {
    (StatusCode::UNPROCESSABLE_ENTITY, Json(ValidationErrorsResponse { errors }))
}

#[utoipa::path(get, path = "/api/v1/platform-admin/layout/screens", tag = "Layout Admin",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "All layout configs")))]
pub async fn list_screens(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let _admin = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    let rows = LayoutRepository::new()
        .list_configs(&state.db)
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    Ok(Json(serde_json::to_value(rows).unwrap_or_default()))
}

#[utoipa::path(get, path = "/api/v1/platform-admin/layout/config", tag = "Layout Admin",
    security(("bearer_auth" = [])), params(("screen" = String, Query, description = "Screen id")),
    responses((status = 200, description = "Config with versions and kills"), (status = 404, description = "Unknown screen")))]
pub async fn get_config(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(q): Query<ScreenQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let _admin = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    let repo = LayoutRepository::new();
    let cfg = repo
        .get_config(&state.db, &q.screen)
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?
        .ok_or((StatusCode::NOT_FOUND, Json(ValidationErrorsResponse { errors: vec!["unknown screen".into()] })))?;
    let versions = repo.list_versions(&state.db, &q.screen).await.unwrap_or_default();
    let kills = repo.list_kills(&state.db, &q.screen).await.unwrap_or_default();
    Ok(Json(serde_json::json!({ "config": cfg, "versions": versions, "kills": kills })))
}

#[utoipa::path(put, path = "/api/v1/platform-admin/layout/draft", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = PutDraftRequest,
    responses((status = 200, description = "Draft saved"), (status = 422, description = "Config does not parse as a ScreenConfig")))]
pub async fn put_draft(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<PutDraftRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let (admin_id, _) = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    // shape gate: must parse as the layout-core contract type
    if let Err(e) = serde_json::from_value::<layout_core::ScreenConfig>(req.config.clone()) {
        return Err(bad_request(vec![format!("invalid ScreenConfig: {e}")]));
    }
    let row = LayoutRepository::new()
        .upsert_draft(&state.db, &req.screen, &req.config, Some(admin_id))
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    Ok(Json(serde_json::to_value(row).unwrap_or_default()))
}

#[utoipa::path(put, path = "/api/v1/platform-admin/layout/rails", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = PutRailsRequest,
    responses((status = 200, description = "Rails saved"), (status = 422, description = "Rails do not parse")))]
pub async fn put_rails(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<PutRailsRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let (admin_id, _) = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    if let Err(e) = serde_json::from_value::<layout_core::Rails>(req.rails.clone()) {
        return Err(bad_request(vec![format!("invalid Rails: {e}")]));
    }
    let row = LayoutRepository::new()
        .set_rails(&state.db, &req.screen, &req.rails, Some(admin_id))
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    Ok(Json(serde_json::to_value(row).unwrap_or_default()))
}

#[utoipa::path(get, path = "/api/v1/platform-admin/layout/manifests", tag = "Layout Admin",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "All registry manifests")))]
pub async fn list_manifests(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let _admin = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    let rows = LayoutRepository::new()
        .list_manifests(&state.db)
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    Ok(Json(serde_json::to_value(rows).unwrap_or_default()))
}

#[utoipa::path(put, path = "/api/v1/platform-admin/layout/manifests", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = PutManifestRequest,
    responses((status = 200, description = "Manifest saved"), (status = 422, description = "Manifest invalid")))]
pub async fn put_manifest(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<PutManifestRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let (admin_id, _) = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    let parsed: layout_core::RegistryManifest = serde_json::from_value(req.manifest.clone())
        .map_err(|e| bad_request(vec![format!("invalid RegistryManifest: {e}")]))?;
    let platform_str = match parsed.platform {
        layout_core::Platform::Web => "web",
        layout_core::Platform::Mobile => "mobile",
    };
    if platform_str != req.platform {
        return Err(bad_request(vec![format!(
            "platform mismatch: body says {}, manifest says {platform_str}", req.platform
        )]));
    }
    let row = LayoutRepository::new()
        .upsert_manifest(&state.db, &req.platform, &req.manifest, Some(admin_id))
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    Ok(Json(serde_json::to_value(row).unwrap_or_default()))
}
```

- [ ] **Step 3: Module wiring**

`backend/servers/api-server/src/routes/layout/mod.rs`:

```rust
pub mod admin;
pub mod types;

use crate::state::AppState;
use axum::routing::{get, put};
use axum::Router;

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/screens", get(admin::list_screens))
        .route("/config", get(admin::get_config))
        .route("/draft", put(admin::put_draft))
        .route("/rails", put(admin::put_rails))
        .route("/manifests", get(admin::list_manifests).put(admin::put_manifest))
}
```

Add `pub mod layout;` to `src/routes/mod.rs` and nest in `src/lib.rs` `route_table()`:

```rust
        .nest("/api/v1/platform-admin/layout", routes::layout::admin_router())
```

- [ ] **Step 4: Compile-verify + workspace dep**

Run: `cd backend && cargo check -p api-server`
Expected: clean. Fix only import-path/field-name ADAPT items (report them). Then `cargo clippy -p api-server --all-targets -- -D warnings` clean. (Route behavior is exercised by the layout-core + db test suites; HTTP-level integration tests are deliberately deferred to the editor plan, which brings a full request-path harness.)

- [ ] **Step 5: Commit**

```bash
git add backend/Cargo.toml backend/Cargo.lock backend/servers/api-server
git commit -m "feat(api-server): platform-admin layout draft, rails and manifest routes"
```

---

### Task 5: api-server publish / rollback / kill routes

**Files:**
- Modify: `backend/servers/api-server/src/routes/layout/admin.rs`
- Modify: `backend/servers/api-server/src/routes/layout/types.rs`
- Modify: `backend/servers/api-server/src/routes/layout/mod.rs`

**Interfaces:**
- Consumes: Task 4 module; `layout_core::validate_publish`; `LayoutRepository::{publish, rollback, kill, unkill, list_manifests, get_config}`.
- Produces: routes `POST /publish`, `POST /rollback`, `POST /kill`, `POST /unkill` on `admin_router()`. Publish is THE hard gate (spec §6.3): draft parsed as `ScreenConfig` + validated against ALL stored manifests; any `ValidationError` → 422 with stringified errors, nothing persisted. Kill/unkill bypass the gate (spec §5).

- [ ] **Step 1: DTOs** — append to `types.rs`:

```rust
#[derive(Debug, Deserialize, ToSchema)]
pub struct PublishRequest {
    pub screen: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RollbackRequest {
    pub screen: String,
    pub version: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct KillRequest {
    pub screen: String,
    pub section_type: String,
}
```

- [ ] **Step 2: Handlers** — append to `admin.rs`:

```rust
use super::types::{KillRequest, PublishRequest, RollbackRequest};

#[utoipa::path(post, path = "/api/v1/platform-admin/layout/publish", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = PublishRequest,
    responses((status = 200, description = "Published"),
              (status = 404, description = "Unknown screen"),
              (status = 409, description = "No registry manifests uploaded yet"),
              (status = 422, description = "Validation errors — publish blocked")))]
pub async fn publish(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<PublishRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let (admin_id, _) = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    let repo = LayoutRepository::new();

    let cfg_row = repo
        .get_config(&state.db, &req.screen)
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?
        .ok_or((StatusCode::NOT_FOUND, Json(ValidationErrorsResponse { errors: vec!["unknown screen".into()] })))?;

    let draft: layout_core::ScreenConfig = serde_json::from_value(cfg_row.draft.clone())
        .map_err(|e| bad_request(vec![format!("stored draft is not a valid ScreenConfig: {e}")]))?;

    let manifest_rows = repo
        .list_manifests(&state.db)
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    if manifest_rows.is_empty() {
        return Err((StatusCode::CONFLICT, Json(ValidationErrorsResponse {
            errors: vec!["no registry manifests uploaded; cannot validate publish".into()],
        })));
    }
    let manifests: Vec<layout_core::RegistryManifest> = manifest_rows
        .iter()
        .map(|r| serde_json::from_value(r.manifest.clone()))
        .collect::<Result<_, _>>()
        .map_err(|e| bad_request(vec![format!("stored manifest is invalid: {e}")]))?;

    let errors = layout_core::validate_publish(&draft, &manifests);
    if !errors.is_empty() {
        return Err(bad_request(errors.iter().map(|e| e.to_string()).collect()));
    }

    let mut conn = state.db.acquire().await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    let row = repo
        .publish(&mut *conn, &req.screen, Some(admin_id))
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    Ok(Json(serde_json::to_value(row).unwrap_or_default()))
}

#[utoipa::path(post, path = "/api/v1/platform-admin/layout/rollback", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = RollbackRequest,
    responses((status = 200, description = "Rolled back"), (status = 404, description = "Unknown screen or version")))]
pub async fn rollback(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<RollbackRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let (admin_id, _) = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    let mut conn = state.db.acquire().await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    let row = LayoutRepository::new()
        .rollback(&mut *conn, &req.screen, req.version, Some(admin_id))
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND,
                Json(ValidationErrorsResponse { errors: vec!["unknown screen or version".into()] })),
            other => bad_request(vec![format!("db error: {other}")]),
        })?;
    Ok(Json(serde_json::to_value(row).unwrap_or_default()))
}

#[utoipa::path(post, path = "/api/v1/platform-admin/layout/kill", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = KillRequest,
    responses((status = 204, description = "Section killed — bypasses the publish gate (spec §5)")))]
pub async fn kill(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<KillRequest>,
) -> Result<StatusCode, (StatusCode, Json<ValidationErrorsResponse>)> {
    let (admin_id, _) = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    LayoutRepository::new()
        .kill(&state.db, &req.screen, &req.section_type, Some(admin_id))
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/v1/platform-admin/layout/unkill", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = KillRequest,
    responses((status = 204, description = "Kill flag removed"), (status = 404, description = "No such kill flag")))]
pub async fn unkill(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<KillRequest>,
) -> Result<StatusCode, (StatusCode, Json<ValidationErrorsResponse>)> {
    let _admin = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    let removed = LayoutRepository::new()
        .unkill(&state.db, &req.screen, &req.section_type)
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, Json(ValidationErrorsResponse { errors: vec!["no such kill flag".into()] })))
    }
}
```

- [ ] **Step 3: Routes** — in `mod.rs` `admin_router()`, add:

```rust
        .route("/publish", axum::routing::post(admin::publish))
        .route("/rollback", axum::routing::post(admin::rollback))
        .route("/kill", axum::routing::post(admin::kill))
        .route("/unkill", axum::routing::post(admin::unkill))
```

- [ ] **Step 4: Verify** — `cd backend && cargo check -p api-server && cargo clippy -p api-server --all-targets -- -D warnings`; both clean.
- [ ] **Step 5: Commit**

```bash
git add backend/servers/api-server
git commit -m "feat(api-server): layout publish gate, rollback and kill-switch routes"
```

---

### Task 6: api-server tenant override routes + resolved endpoint

**Files:**
- Create: `backend/servers/api-server/src/routes/layout/tenant.rs`
- Create: `backend/servers/api-server/src/routes/layout/resolved.rs`
- Modify: `backend/servers/api-server/src/routes/layout/mod.rs`
- Modify: `backend/servers/api-server/src/routes/layout/types.rs`
- Modify: `backend/servers/api-server/src/lib.rs` (`route_table()`: add `.nest("/api/v1/layout", routes::layout::router())`)

**Interfaces:**
- Consumes: `AuthUser`, `TenantExtractor`, `RlsConnection` from `api_core` (mirror `src/routes/announcements/lifecycle.rs` for exact import paths and the tenant-id/role field names — ADAPT rule); `layout_core::{resolve, validate_tenant_override, Platform, Rails, ScreenConfig, SectionType, TenantOverride}`.
- Produces: `router() -> Router<AppState>` with `GET /tenant-override` (+`?screen=`), `PUT /tenant-override`, `GET /resolved/{*screen}?platform=`. Tenant writes require `tenant.role.is_admin()` and pass the `validate_tenant_override` gate against the screen's published base + rails. Resolved endpoint: published config + platform manifest + kills from the unscoped pool, tenant override via `RlsConnection`, merged by `layout_core::resolve`.

- [ ] **Step 1: DTOs** — append to `types.rs`:

```rust
#[derive(Debug, Deserialize, ToSchema)]
pub struct PutTenantOverrideRequest {
    pub screen: String,
    /// Must deserialize as layout_core::TenantOverride and pass rails validation.
    pub override_config: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResolvedQuery {
    /// "web" | "mobile" (default "web")
    pub platform: Option<String>,
}
```

- [ ] **Step 2: Tenant handlers**

`backend/servers/api-server/src/routes/layout/tenant.rs` (mirror `announcements/lifecycle.rs` idiom; `rls.release().await` before every return path that acquired it):

```rust
use crate::state::AppState;
use api_core::extractors::{AuthUser, RlsConnection, TenantExtractor};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use db::repositories::LayoutRepository;

use super::types::{PutTenantOverrideRequest, ScreenQuery, ValidationErrorsResponse};

#[utoipa::path(get, path = "/api/v1/layout/tenant-override", tag = "Layout",
    security(("bearer_auth" = [])), params(("screen" = String, Query, description = "Screen id")),
    responses((status = 200, description = "Override + rails + published base for the org")))]
pub async fn get_tenant_override(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Query(q): Query<ScreenQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let repo = LayoutRepository::new();
    let org_id = tenant.tenant_id; // ADAPT: exact field per api_core::TenantExtractor
    let ov = repo
        .get_tenant_override(&mut **rls.conn(), org_id, &q.screen)
        .await;
    rls.release().await;
    let ov = ov.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR,
        Json(ValidationErrorsResponse { errors: vec![format!("db error: {e}")] })))?;
    let cfg = repo
        .get_config(&state.db, &q.screen)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR,
            Json(ValidationErrorsResponse { errors: vec![format!("db error: {e}")] })))?
        .ok_or((StatusCode::NOT_FOUND,
            Json(ValidationErrorsResponse { errors: vec!["unknown screen".into()] })))?;
    Ok(Json(serde_json::json!({
        "override": ov,
        "rails": cfg.rails,
        "published": cfg.published,
    })))
}

#[utoipa::path(put, path = "/api/v1/layout/tenant-override", tag = "Layout",
    security(("bearer_auth" = [])), request_body = PutTenantOverrideRequest,
    responses((status = 200, description = "Override saved"),
              (status = 403, description = "Org admin role required"),
              (status = 404, description = "Screen not published"),
              (status = 422, description = "Out-of-rails edits rejected")))]
pub async fn put_tenant_override(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Json(req): Json<PutTenantOverrideRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    if !tenant.role.is_admin() {
        rls.release().await;
        return Err((StatusCode::FORBIDDEN,
            Json(ValidationErrorsResponse { errors: vec!["org admin role required".into()] })));
    }
    let org_id = tenant.tenant_id; // ADAPT: exact field per api_core::TenantExtractor
    let repo = LayoutRepository::new();

    let parse_err = |e: serde_json::Error, what: &str| (StatusCode::UNPROCESSABLE_ENTITY,
        Json(ValidationErrorsResponse { errors: vec![format!("invalid {what}: {e}")] }));

    let ov: layout_core::TenantOverride = match serde_json::from_value(req.override_config.clone()) {
        Ok(v) => v,
        Err(e) => { rls.release().await; return Err(parse_err(e, "TenantOverride")); }
    };

    let cfg = match repo.get_config(&state.db, &req.screen).await {
        Ok(Some(c)) => c,
        Ok(None) => { rls.release().await; return Err((StatusCode::NOT_FOUND,
            Json(ValidationErrorsResponse { errors: vec!["unknown screen".into()] }))); }
        Err(e) => { rls.release().await; return Err((StatusCode::INTERNAL_SERVER_ERROR,
            Json(ValidationErrorsResponse { errors: vec![format!("db error: {e}")] }))); }
    };
    let Some(published) = cfg.published.clone() else {
        rls.release().await;
        return Err((StatusCode::NOT_FOUND,
            Json(ValidationErrorsResponse { errors: vec!["screen has no published config".into()] })));
    };
    let base: layout_core::ScreenConfig = match serde_json::from_value(published) {
        Ok(v) => v,
        Err(e) => { rls.release().await; return Err(parse_err(e, "stored published config")); }
    };
    let rails: layout_core::Rails = match serde_json::from_value(cfg.rails.clone()) {
        Ok(v) => v,
        Err(e) => { rls.release().await; return Err(parse_err(e, "stored rails")); }
    };

    let errors = layout_core::validate_tenant_override(&ov, &base, &rails);
    if !errors.is_empty() {
        rls.release().await;
        return Err((StatusCode::UNPROCESSABLE_ENTITY, Json(ValidationErrorsResponse {
            errors: errors.iter().map(|e| e.to_string()).collect(),
        })));
    }

    let saved = repo
        .upsert_tenant_override(&mut **rls.conn(), org_id, &req.screen,
                                &req.override_config, None)
        .await;
    rls.release().await;
    let saved = saved.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR,
        Json(ValidationErrorsResponse { errors: vec![format!("db error: {e}")] })))?;
    Ok(Json(serde_json::to_value(saved).unwrap_or_default()))
}
```

- [ ] **Step 3: Resolved handler**

`backend/servers/api-server/src/routes/layout/resolved.rs`:

```rust
use crate::state::AppState;
use api_core::extractors::{AuthUser, RlsConnection, TenantExtractor};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use db::repositories::LayoutRepository;
use std::collections::BTreeSet;

use super::types::{ResolvedQuery, ValidationErrorsResponse};

pub fn parse_platform(s: Option<&str>) -> Result<layout_core::Platform, String> {
    match s.unwrap_or("web") {
        "web" => Ok(layout_core::Platform::Web),
        "mobile" => Ok(layout_core::Platform::Mobile),
        other => Err(format!("unknown platform {other:?} (expected web|mobile)")),
    }
}

#[utoipa::path(get, path = "/api/v1/layout/resolved/{screen}", tag = "Layout",
    security(("bearer_auth" = [])),
    params(("screen" = String, Path, description = "Screen id, e.g. ppt/dashboard"),
           ("platform" = Option<String>, Query, description = "web|mobile, default web")),
    responses((status = 200, description = "Resolved section list"),
              (status = 404, description = "Screen not published or manifest missing")))]
pub async fn get_resolved(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(screen): Path<String>,
    Query(q): Query<ResolvedQuery>,
) -> Result<Json<layout_core::ResolvedScreen>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let err500 = |msg: String| (StatusCode::INTERNAL_SERVER_ERROR,
        Json(ValidationErrorsResponse { errors: vec![msg] }));
    let err404 = |msg: &str| (StatusCode::NOT_FOUND,
        Json(ValidationErrorsResponse { errors: vec![msg.to_string()] }));

    let platform = match parse_platform(q.platform.as_deref()) {
        Ok(p) => p,
        Err(e) => { rls.release().await; return Err((StatusCode::BAD_REQUEST,
            Json(ValidationErrorsResponse { errors: vec![e] }))); }
    };
    let repo = LayoutRepository::new();
    let org_id = tenant.tenant_id; // ADAPT: exact field per api_core::TenantExtractor

    let tenant_ov_row = repo
        .get_tenant_override(&mut **rls.conn(), org_id, &screen)
        .await;
    rls.release().await;
    let tenant_ov_row = tenant_ov_row.map_err(|e| err500(format!("db error: {e}")))?;

    let cfg = repo
        .get_config(&state.db, &screen)
        .await
        .map_err(|e| err500(format!("db error: {e}")))?
        .ok_or_else(|| err404("unknown screen"))?;
    let Some(published) = cfg.published else {
        return Err(err404("screen has no published config"));
    };
    let base: layout_core::ScreenConfig = serde_json::from_value(published)
        .map_err(|e| err500(format!("stored published config invalid: {e}")))?;

    let platform_key = match platform {
        layout_core::Platform::Web => "web",
        layout_core::Platform::Mobile => "mobile",
    };
    let manifest_row = repo
        .get_manifest(&state.db, platform_key)
        .await
        .map_err(|e| err500(format!("db error: {e}")))?
        .ok_or_else(|| err404("no registry manifest for platform"))?;
    let manifest: layout_core::RegistryManifest = serde_json::from_value(manifest_row.manifest)
        .map_err(|e| err500(format!("stored manifest invalid: {e}")))?;

    let kills: BTreeSet<layout_core::SectionType> = repo
        .list_kills(&state.db, &screen)
        .await
        .map_err(|e| err500(format!("db error: {e}")))?
        .into_iter()
        .map(|k| layout_core::SectionType(k.section_type))
        .collect();

    let tenant_ov: Option<layout_core::TenantOverride> = match tenant_ov_row {
        Some(row) => Some(serde_json::from_value(row.override_config)
            .map_err(|e| err500(format!("stored tenant override invalid: {e}")))?),
        None => None,
    };

    let resolved = layout_core::resolve(&base, platform, tenant_ov.as_ref(), &kills, &manifest);
    Ok(Json(resolved))
}
```

Note: `Json<layout_core::ResolvedScreen>` needs `Serialize` only (already derived in layout-core); the `#[utoipa::path]` responses use plain descriptions, no `body =`, so no `ToSchema` is required on layout-core types.

- [ ] **Step 4: Routes** — in `mod.rs` add:

```rust
pub mod resolved;
pub mod tenant;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tenant-override",
               get(tenant::get_tenant_override).put(tenant::put_tenant_override))
        .route("/resolved/{*screen}", get(resolved::get_resolved))
}
```

and in `src/lib.rs` `route_table()`:

```rust
        .nest("/api/v1/layout", routes::layout::router())
```

- [ ] **Step 5: Verify** — `cd backend && cargo check -p api-server && cargo clippy -p api-server --all-targets -- -D warnings`; clean. Report every ADAPT change made.
- [ ] **Step 6: Commit**

```bash
git add backend/servers/api-server
git commit -m "feat(api-server): tenant layout overrides and resolved layout endpoint"
```

---

### Task 7: reality-server public resolved endpoint

**Files:**
- Create: `backend/servers/reality-server/src/routes/layout.rs`
- Modify: `backend/servers/reality-server/src/routes/mod.rs` (`pub mod layout;`)
- Modify: `backend/servers/reality-server/src/main.rs` (add `.nest("/api/v1/layout", routes::layout::router())` alongside the existing nests)
- Add dependency: `backend/servers/reality-server/Cargo.toml` → `layout-core` (same style as Task 4's api-server dep).

**Interfaces:**
- Consumes: `AppState::acquire_public_conn()` (clears any stale tenant context — required by `scripts/check-rls-enforcement.sh`), `LayoutRepository`, `layout_core::resolve`.
- Produces: `GET /api/v1/layout/resolved/{*screen}?platform=` — public, no auth, NO tenant layer (`tenant: None`). Only screens with a published config resolve; others 404.

- [ ] **Step 1: Handler + router**

`backend/servers/reality-server/src/routes/layout.rs`:

```rust
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use db::repositories::LayoutRepository;
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
pub struct ResolvedQuery {
    pub platform: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/resolved/{*screen}", get(get_resolved))
}

#[utoipa::path(get, path = "/api/v1/layout/resolved/{screen}", tag = "Layout",
    params(("screen" = String, Path, description = "Screen id, e.g. reality/listing-detail"),
           ("platform" = Option<String>, Query, description = "web|mobile, default web")),
    responses((status = 200, description = "Resolved section list (public, no tenant layer)"),
              (status = 404, description = "Screen not published or manifest missing")))]
pub async fn get_resolved(
    State(state): State<AppState>,
    Path(screen): Path<String>,
    Query(q): Query<ResolvedQuery>,
) -> Result<Json<layout_core::ResolvedScreen>, (StatusCode, String)> {
    let platform = match q.platform.as_deref().unwrap_or("web") {
        "web" => layout_core::Platform::Web,
        "mobile" => layout_core::Platform::Mobile,
        other => return Err((StatusCode::BAD_REQUEST, format!("unknown platform {other:?}"))),
    };
    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?;
    let repo = LayoutRepository::new();

    let cfg = repo
        .get_config(&mut *conn, &screen)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?
        .ok_or((StatusCode::NOT_FOUND, "unknown screen".to_string()))?;
    let published = cfg
        .published
        .ok_or((StatusCode::NOT_FOUND, "screen has no published config".to_string()))?;
    let base: layout_core::ScreenConfig = serde_json::from_value(published)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("stored config invalid: {e}")))?;

    let platform_key = match platform {
        layout_core::Platform::Web => "web",
        layout_core::Platform::Mobile => "mobile",
    };
    let manifest_row = repo
        .get_manifest(&mut *conn, platform_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?
        .ok_or((StatusCode::NOT_FOUND, "no registry manifest for platform".to_string()))?;
    let manifest: layout_core::RegistryManifest = serde_json::from_value(manifest_row.manifest)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("stored manifest invalid: {e}")))?;

    let kills: BTreeSet<layout_core::SectionType> = repo
        .list_kills(&mut *conn, &screen)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?
        .into_iter()
        .map(|k| layout_core::SectionType(k.section_type))
        .collect();

    // Public portal: no tenant layer (spec §3.2 — the tenant layer simply
    // doesn't contribute on public reality screens).
    let resolved = layout_core::resolve(&base, platform, None, &kills, &manifest);
    Ok(Json(resolved))
}
```

(If `acquire_public_conn` returns a different error/conn shape, mirror its usage in `src/routes/listings.rs` — ADAPT rule; report it.)

- [ ] **Step 2: Verify** — `cd backend && cargo check -p reality-server && cargo clippy -p reality-server --all-targets -- -D warnings`; clean. Run `./scripts/check-rls-enforcement.sh` if it covers reality-server handlers; must pass (we use `acquire_public_conn`, never `state.db` directly).
- [ ] **Step 3: Commit**

```bash
git add backend/Cargo.lock backend/servers/reality-server
git commit -m "feat(reality-server): public resolved layout endpoint"
```

---

### Task 8: TypeSpec design artifact

**Files:**
- Create: `docs/api/typespec/domains/layout.tsp`
- Modify: `docs/api/typespec/main.tsp` (add `import "./domains/layout.tsp";` with the other domain imports)
- Regenerate: `docs/api/generated/openapi.yaml`

**Interfaces:**
- Consumes: shared TypeSpec modules (`models.tsp`, `errors.tsp`).
- Produces: design-artifact coverage of the four public-ish endpoints (resolved GET ×2 audiences, tenant-override GET/PUT). Platform-admin routes stay utoipa-only (matching how other platform-admin surfaces are handled — TypeSpec covers the client-SDK-facing API).

- [ ] **Step 1: Write the domain**

`docs/api/typespec/domains/layout.tsp` (mirror the imports/`using` lines of `domains/listings.tsp` exactly — ADAPT rule for the shared-module import list):

```typespec
import "@typespec/http";
import "../shared/models.tsp";
import "../shared/errors.tsp";

namespace PropertyManagement.Layout;

using TypeSpec.Http;
using PropertyManagement.Shared;

@doc("Which platform's component registry to resolve against.")
enum LayoutPlatform {
  web,
  mobile,
}

@doc("How a resolved section renders. Required sections that are hidden or killed degrade to placeholder; optional ones are omitted entirely.")
enum SectionPresentation {
  visible,
  placeholder,
}

@doc("One resolved section, ready to render.")
model ResolvedSection {
  @doc("Versioned semantic component type, e.g. price-box.v1")
  type: string;

  @doc("Display mode, guaranteed to be within the component's supported modes.")
  mode?: string;

  @doc("Component props (empty for placeholders).")
  props?: Record<unknown>;

  presentation: SectionPresentation;
}

@doc("A fully resolved screen layout: base config merged with platform, tenant and kill layers.")
model ResolvedScreen {
  @doc("Screen id, e.g. ppt/dashboard")
  screen: string;

  @doc("Published config version this resolution was computed from.")
  version: int32;

  sections: ResolvedSection[];
}

@doc("Sparse per-organization layout override (validated against superadmin-authored rails).")
model TenantLayoutOverride {
  @doc("Full desired section order by type; omitted = keep base order.")
  order?: string[];

  @doc("Per-section patches: visible / mode / whitelisted props.")
  sections?: Record<unknown>;
}

model TenantOverrideEnvelope {
  override: Record<unknown> | null;
  rails: Record<unknown>;
  published: Record<unknown> | null;
}

model PutTenantOverrideRequest {
  screen: string;
  override_config: TenantLayoutOverride;
}

@route("/api/v1/layout")
@tag("Layout")
interface LayoutApi {
  @doc("Resolve a screen's layout for the caller's organization and platform.")
  @get
  @route("/resolved/{screen}")
  getResolved(
    @path screen: string,
    @query platform?: LayoutPlatform,
  ): ResolvedScreen | Error;

  @doc("Read the caller organization's override plus the rails and published base.")
  @get
  @route("/tenant-override")
  getTenantOverride(@query screen: string): TenantOverrideEnvelope | Error;

  @doc("Save the caller organization's override. Rejected with 422 when it exceeds the rails.")
  @put
  @route("/tenant-override")
  putTenantOverride(@body body: PutTenantOverrideRequest): TenantOverrideEnvelope | Error;
}
```

- [ ] **Step 2: Compile + regenerate**

Run: `cd docs/api/typespec && npm install && npx tsp compile .`
Expected: compiles; `docs/api/generated/openapi.yaml` updated (additive only — `oasdiff breaking` in CI must stay green).

- [ ] **Step 3: Commit**

```bash
git add docs/api/typespec docs/api/generated
git commit -m "docs(api): layout domain TypeSpec (resolved + tenant override)"
```

---

### Task 9: Workspace gates + repo-map

**Files:**
- Modify: `docs/repo-map.md` (the crates bullet for `layout-core` — extend it with the control-plane pointers)
- Test: full gates

**Interfaces:**
- Consumes: everything above.
- Produces: green workspace gates; repo-map current.

- [ ] **Step 1: repo-map**

In `docs/repo-map.md`, extend the existing `layout-core` bullet (added by the previous plan) to read:

```markdown
- `layout-core` — Layout & Content Manager contract: screen configs, merge resolver
  (base → platform → tenant → kill), publish/rails validation. Pure logic, no DB.
  Control plane: `db/src/repositories/layout.rs` + migration 00220; routes at
  `api-server/src/routes/layout/` (admin + tenant + resolved) and
  `reality-server/src/routes/layout.rs` (public resolved).
  Spec: `docs/superpowers/specs/2026-07-19-layout-content-manager-design.md`.
```

- [ ] **Step 2: Full gate**

Run:
```bash
cd backend && cargo fmt --all \
  && cargo clippy --workspace --all-targets -- -D warnings \
  && cargo test -p layout-core \
  && DATABASE_URL=postgres://postgres:postgres@localhost:5433/postgres cargo test -p db --test layout_repo_tests
./scripts/check-migration-versions.sh
```
Expected: all green (layout-core 15 tests; layout_repo_tests 4 tests). Include any `cargo fmt` fallout in the commit.

- [ ] **Step 3: Commit**

```bash
git add docs/repo-map.md backend
git commit -m "docs(repo-map): layout control-plane pointers"
```

---

## Deliberate scope decisions (do not "fix" these during implementation)

- **No HTTP-level integration tests in this plan.** Route handlers are thin adapters over `layout-core` (15 unit tests) and `LayoutRepository` (4 DB tests). The editor plan brings a request-path harness plus dev-stack smoke checks; adding an auth-mocking harness here would double the plan for marginal coverage.
- **No ISR webhook / cache invalidation yet** — that belongs to the defensive-rendering plan (reality-web consumes the endpoint there).
- **No audit-log table beyond version history + `updated_by`/`killed_by` columns** — full who/what/when-diff audit UI lands with the editor plan.
- **Platform-admin routes are utoipa-annotated but not in `ApiDoc`** — consistent with forms/announcements; a follow-up registers them all.
- **`resolve()` takes no app_version yet** — stale-client capability filtering is deferred exactly as documented in the layout-core crate.

## Out of scope (subsequent plans)

1. **Defensive rendering** — pilot screens in ppt-web/reality-web consuming `GET /resolved`, section registries, gap spacing, error boundaries, placeholder component, ISR revalidation.
2. **Superadmin editor MVP**, then tenant editor + rails authoring UI.
3. **Preview bridge**; mobile manifests + RN/KMP renderers.
