# ACC — Backend Architecture & Service Topology

> Decision record for delivering the `ACC` product as a **separate backend server that shares core crates with
> `api-server`**, mirroring how `reality-server` is structured today. Grounded in the actual `backend/` workspace.

## Context (as-is)

The Rust backend is a single Cargo workspace = **shared `crates/*`** composed by **thin `servers/*`** binaries:

```
backend/
├── crates/                      # shared core (libraries)
│   ├── common/                  # errors, types, utilities
│   ├── api-core/                # auth extractors, middleware, cache, OpenAPI plumbing
│   ├── db/                      # sqlx pool, migrations, RLS-aware queries
│   ├── tenant-ops/              # tenant/RLS operations
│   └── integrations/            # external integrations
└── servers/
    ├── api-server/      :8080   # Property Mgmt API   (OAuth/JWT issuer)
    ├── reality-server/ :8081   # Reality Portal API  (SSO consumer; users separate from PM)
    └── deploy-server/           # worktree deploy control
```

`reality-server` depends on exactly **`common` + `api-core` + `db`** (`backend/servers/reality-server/Cargo.toml`),
binds `:8081` (`main.rs`), and owns its `handlers/ routes/ extractors/ state.rs/ observability.rs`. **It shares the
same PostgreSQL database** with `api-server` (root `CLAUDE.md`: "Shared Database"), but keeps its own user accounts.

The accounting MVP currently lives **embedded in `api-server`** as `/api/v1/accounting/*` (on `origin/dev`, with
`accounting.tsp` + migrations `00183/00184`, RLS-secured). This decision **extracts it into its own server**.

## Decision

**Add `backend/servers/accounting-server` (port `:8082`), structured as a clone of `reality-server`, sharing the core
crates with `api-server`; put accounting domain logic in a new `backend/crates/accounting-core`.**

```
backend/
├── crates/
│   ├── common/        ← shared (unchanged)
│   ├── api-core/       ← shared (unchanged): JWT validation, extractors, middleware, OpenAPI, observability
│   ├── db/             ← shared (unchanged): pool + migrations (now also accounting tables)
│   └── accounting-core/   # NEW — invoices, documents, VAT engine, numbering, money/rounding math
└── servers/
    └── accounting-server/ :8082   # NEW — thin binary: routes/ handlers/ state.rs/ main.rs/ observability.rs
                                    #   deps = common + api-core + db + accounting-core
```

### Why "core" stays in `api-core` / `db` / `common`
"Share core with api-server" = depend on the **same plumbing crates**, not on `api-server` the binary:
- **Auth:** `accounting-server` validates the **same JWTs** via `api-core` extractors; `api-server` remains the
  **OAuth/token issuer**, `accounting-server` is a **resource server** (same pattern as `reality-server` = SSO consumer).
- **DB:** same `db` crate, same pool, same migrations dir → accounting tables live in the shared Postgres with RLS.
- **Money/rounding/VAT math** that must be identical everywhere → `accounting-core` (single source), consumed by the
  server and reusable by any future worker (recurring runs, OCR post, bank-match).

### Key sub-decisions

| Topic | Decision | Note |
|-------|----------|------|
| **Port** | `:8082` | After 8080 (api), 8081 (reality). |
| **Database** | **Shared Postgres**, dedicated accounting tables + RLS | Matches `reality-server`. Keep `db`-crate boundary clean so a future split to a **dedicated accounting DB** is possible if the product is spun out commercially. |
| **Identity** | Shared auth plumbing (`api-core`), **own account/user domain** | Like reality portal users are "separate from PM". Accounting customers ≠ PPT owners/managers. |
| **Domain logic** | New `crates/accounting-core` | Server crate stays thin (routes/handlers/state), exactly like `reality-server`. |
| **OpenAPI/SDK** | Own `/swagger-ui` + spec → `@ppt/accounting-api-client` (hey-api) | `accounting.tsp` already exists on dev. |
| **Frontend** | `@ppt/accounting-web` (Next.js + next-intl) → `:8082` | Mirrors `reality-web → reality-server`. |
| **Deploy** | New service in compose/manifests/caddy; CORS origins; per-worktree via `pmctl` | `accounting.dev.*` host like `wt-*` deploys. |
| **Observability** | Reuse `api-core` OTel/metrics/sentry stack | `reality-server` already does. |

### Migration path (from the dev MVP)

1. Scaffold `crates/accounting-core` + `servers/accounting-server` (copy `reality-server` skeleton).
2. Move `api-server`'s `handlers/accounting/*` + routes → `accounting-server`; lift pure logic into `accounting-core`.
3. Keep migrations `00183/00184` (+ new ones) in the shared `db` crate.
4. Generate `@ppt/accounting-api-client` from the new server's OpenAPI; point `@ppt/accounting-web` at `:8082`.
5. Remove `/api/v1/accounting/*` from `api-server` once the new server is green (avoid a dual-serve window in prod).
6. Wire deploy: compose service, caddy route, CORS, `pmctl` worktree target.

## Alternatives considered

- **A. Keep accounting embedded in `api-server`** (status quo on dev). *Rejected* for the standalone-product goal:
  couples release cadence, auth surface, and scaling of an external SaaS to the internal PM API.
- **B. Fully standalone service with its own DB + own auth from day one.** *Deferred*: maximal isolation but adds
  identity federation + cross-DB sync now, for a benefit only realized if/when the product is commercially separated.
  The chosen design **keeps that door open** (clean `db`/`accounting-core` boundaries) without paying the cost upfront.
- **C. Separate server, shared DB & shared core** ✅ *Chosen* — consistent with `reality-server`, lowest friction,
  honors "share core with api-server," and is incrementally splittable later.

## Impact on the epic set

Adds one delivery epic (platform/topology), and refines dependencies of `EPIC-ACC-14/16`:

- **`EPIC-ACC-17` — Backend Service (`accounting-server`) & Shared-Core Integration** (P1, foundational): scaffold the
  crate+server, share `common`/`api-core`/`db`, bind `:8082`, OpenAPI+SDK, deploy/CORS, extract dev MVP handlers.
- `EPIC-ACC-01` (auth) now explicitly = **consume** `api-core` JWT validation; `api-server` stays the issuer.
- `EPIC-ACC-14` (API) = the public REST surface is **served by `accounting-server`**, not `api-server`.
