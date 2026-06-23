# EPIC-ACC-17 — Backend Service (`accounting-server`) & Shared-Core Integration · Stories

> Covers the topology decided in [`../architecture.md`](../architecture.md): a separate `accounting-server` (`:8082`)
> sharing `common`/`api-core`/`db` core with `api-server`, mirroring `reality-server`. Implementation/infra epic.
> **Shared DoD:** `cargo build --workspace` + `cargo clippy --workspace -- -D warnings` clean · tests green ·
> no new core-crate API just for this server unless reused · OpenAPI compiles.

---

## STORY-ACC-17-001 — Scaffold `accounting-core` crate + `accounting-server` binary
*Foundational*

**User Story:** As the **platform**, I want a thin `accounting-server` binary backed by an `accounting-core` crate, so that accounting has its own deployable surface like `reality-server`.

**Acceptance Criteria**
- **Given** the workspace, **when** I add `crates/accounting-core` and `servers/accounting-server` (copying the `reality-server` skeleton: `routes/ handlers/ extractors/ state.rs/ main.rs/ observability.rs`), **then** both are workspace members and build.
- **Given** the binary, **when** I run `cargo run -p accounting-server`, **then** it binds `:8082` and serves `/health` + `/swagger-ui`.
- **Given** domain logic, **when** added, **then** it lives in `accounting-core` and the server crate stays thin (routing/wiring only).

**Technical Notes:** mirror `backend/servers/reality-server` exactly; reuse `api-core` observability (OTel/metrics/sentry).
**Test Cases:** `:8082/health` 200; swagger served; clippy clean; server crate has no business logic.

## STORY-ACC-17-002 — Share core & validate api-server JWTs (resource server)
*Foundational*

**User Story:** As the **platform**, I want `accounting-server` to share core crates and trust `api-server`-issued tokens, so that we don't re-implement auth/DB/tenancy.

**Acceptance Criteria**
- **Given** `accounting-server/Cargo.toml`, **when** declared, **then** its deps include `common`, `api-core`, `db`, `accounting-core` (same pattern as `reality-server` = common+api-core+db).
- **Given** a valid `api-server`-issued JWT, **when** a request hits a protected `accounting-server` route, **then** the shared `api-core` extractor validates it and resolves tenant/company context.
- **Given** an invalid/expired token, **when** a request arrives, **then** it's rejected 401; **and** `api-server` remains the sole token **issuer** (accounting-server issues none).

**Technical Notes:** resource-server pattern (like reality-server = SSO consumer); same `JWT_SECRET`/keys; tenant context via `tenant-ops`.
**Test Cases:** valid token accepted; tampered/expired rejected; tenant context populated; no token-issuing endpoints exist.

## STORY-ACC-17-003 — Shared-DB accounting tables + RLS
*Foundational*

**User Story:** As the **platform**, I want accounting tables in the shared Postgres with RLS, so that data is isolated per company like the rest of the system.

**Acceptance Criteria**
- **Given** the `db` crate migrations, **when** accounting migrations are added/kept (`00183/00184` + new), **then** `accounting-server` uses the same pool/migrations as `api-server`.
- **Given** RLS policies, **when** any accounting query runs, **then** it is scoped to the company; cross-company access returns nothing.
- **Given** the `db`-crate boundary, **when** reviewed, **then** it stays clean enough that a future **dedicated accounting DB** split is possible without app rewrites.

**Technical Notes:** migrations live in `db` crate (shared); RLS verified by test as defense-in-depth.
**Test Cases:** migration applies; RLS cross-company isolation; pool shared/configurable via `DATABASE_URL`.

## STORY-ACC-17-004 — OpenAPI + `@ppt/accounting-api-client`
*Depends on 001–003*

**User Story:** As a **frontend dev**, I want a generated typed client from the server's OpenAPI, so that `@ppt/accounting-web` consumes a contract, not hand-written calls.

**Acceptance Criteria**
- **Given** `accounting-server` routes, **when** built, **then** they emit an OpenAPI spec (utoipa) served at `/swagger-ui`.
- **Given** the spec, **when** SDK generation runs (hey-api), **then** `@ppt/accounting-api-client` is produced and type-checks.
- **Given** a contract change, **when** CI runs, **then** breaking-change detection flags it (consistent with repo API strategy).

**Technical Notes:** `accounting.tsp` already exists on dev — reconcile design-first TypeSpec with utoipa output per repo API strategy.
**Test Cases:** spec compiles; SDK generates + type-checks; breaking-change check wired.

## STORY-ACC-17-005 — Deploy wiring (`:8082`)
*Depends on 001*

**User Story:** As **DevOps**, I want `accounting-server` deployable like the other servers, so that worktree/staging/prod targets exist.

**Acceptance Criteria**
- **Given** compose/manifests, **when** updated, **then** an `accounting-server` service builds and runs on `:8082` with required env (`DATABASE_URL`, `JWT_SECRET`, `CORS_ALLOWED_ORIGINS`).
- **Given** the `pmctl` worktree system, **when** a worktree is opened, **then** `accounting.dev.*` (and `@ppt/accounting-web` host) resolve and serve.
- **Given** caddy routing + CORS, **when** the frontend calls `:8082`, **then** requests succeed cross-origin per config.

**Technical Notes:** mirror reality-server deploy; add host to caddy + CORS defaults; `pmctl` target.
**Test Cases:** service healthy in compose; worktree host reachable; CORS preflight ok.

## STORY-ACC-17-006 — Migrate `/api/v1/accounting/*` out of `api-server`
*Depends on 001–005*

**User Story:** As the **platform**, I want the embedded accounting MVP moved into the new server, so that there's one owner and no dual-serve.

**Acceptance Criteria**
- **Given** `api-server`'s `handlers/accounting/*` + routes (on dev), **when** migrated, **then** equivalent endpoints serve from `accounting-server` with pure logic lifted into `accounting-core`.
- **Given** parity is verified green, **when** the cutover lands, **then** `/api/v1/accounting/*` is **removed from `api-server`** (returns 404) — no dual-serve window in prod.
- **Given** existing clients, **when** repointed, **then** `@ppt/accounting-web` targets `:8082` and works end-to-end.

**Technical Notes:** dev/main are disjoint — land on the correct base; verify parity before removing the old route.
**Test Cases:** endpoint parity suite; old route 404 after cutover; web e2e against `:8082`.

---

## Coverage
| Story | Scope |
|-------|-------|
| 001 Scaffold crate+server | accounting-core + accounting-server `:8082` |
| 002 Share core & JWT | common/api-core/db + resource-server auth |
| 003 Shared-DB + RLS | migrations in `db`, isolation |
| 004 OpenAPI + SDK | `@ppt/accounting-api-client` |
| 005 Deploy wiring | compose/caddy/CORS/`pmctl` |
| 006 Migrate from api-server | extract `/api/v1/accounting/*`, drop old route |
