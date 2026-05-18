---
name: ppt-rust-backend
description: Navigate the backend Cargo workspace — crate layout, per-crate dev loop, sqlx workflow, seed data.
when_to_use: The plan touches anything under backend/ — a crate, a server binary, a migration, or sqlx offline data.
mode: both
capabilities: [C6]
tags: [backend]
---

# PPT Rust Backend

Cargo workspace at `backend/`. Axum-based servers + sqlx + utoipa.

## When to invoke

Any plan whose *Required capabilities* or *Suggested approach* references
`backend/crates/**` or `backend/servers/**`, or whose vector area is
`backend`.

## What it gives you

- Workspace layout — every crate's responsibility in one line
- Per-crate iteration loop (`cargo check -p <crate>`)
- sqlx offline-data workflow
- Where seed data lives (and why `just seed` does NOT exist)

## Inputs

- A target crate (one of the workspace members below) or a server binary

## Workspace layout

`backend/Cargo.toml` declares these workspace members:

| Member | Path | Responsibility |
|---|---|---|
| `common` | `backend/crates/common` | shared types, error, utils |
| `api-core` | `backend/crates/api-core` | public-API handlers / DTOs |
| `admin-core` | `backend/crates/admin-core` | admin / super-admin handlers |
| `db` | `backend/crates/db` | sqlx queries, migrations, seed helpers |
| `integrations` | `backend/crates/integrations` | external services (S3, email, …) |
| `tenant-ops` | `backend/crates/tenant-ops` | multi-tenant lifecycle ops |
| `api-server` | `backend/servers/api-server` | binary on `:8080` — Property Management API |
| `reality-server` | `backend/servers/reality-server` | binary on `:8081` — Reality Portal API |
| `deploy-server` | `backend/servers/deploy-server` | deploy automation server |

Migrations live in **two** places (intentional split — different DBs):
- `backend/crates/db/migrations/*.sql` — main PPT DB
- `backend/servers/deploy-server/migrations/*.sql` — deploy server DB

Seed code is **in-crate**: `backend/crates/db/src/seed/` (`data.rs`,
`factories.rs`, `runner.rs`, `mod.rs`). There is **no `just seed`** recipe
— see [`ppt-db-migrations`](../ppt-db-migrations/SKILL.md) for the gap and
how to invoke seeding today.

## Steps

1. **Iterate on one crate.** The whole-workspace check is slow; iterate per crate:
   ```bash
   cd backend
   cargo check -p <crate>          # fastest signal
   cargo clippy -p <crate> -- -D warnings
   cargo test   -p <crate> -- <filter>
   ```
2. **Run a server binary locally** (Docker not required — direct cargo):
   ```bash
   just api              # api-server on :8080
   just reality-server   # reality-server on :8081
   ```
   These expect `postgres` reachable on `localhost:5432`; bring it up via
   [`ppt-dev-stack`](../ppt-dev-stack/SKILL.md) if needed.
3. **sqlx — add or change a query.**
   - Edit the query.
   - With DB up, `cd backend && cargo check -p db` to validate against live schema.
   - Refresh offline data so CI passes:
     ```bash
     just db-prepare      # cargo sqlx prepare --workspace
     ```
   - Commit the regenerated `backend/.sqlx/*.json` files.
4. **Add a migration.**
   ```bash
   just db-migration <name>     # creates backend/crates/db/migrations/<ts>_<name>.sql
   # edit it, then:
   just db-migrate              # apply locally
   just db-prepare              # regenerate sqlx offline cache if queries changed
   ```
5. **Whole-workspace gate** before claiming done:
   ```bash
   just check-backend && just test-backend
   ```

## Deterministic verification

```bash
# 1. cargo toolchain on PATH
cargo --version | grep -q '^cargo' && echo OK
# expected: OK

# 2. workspace resolves
(cd backend && cargo metadata --no-deps --format-version=1 >/dev/null) && echo OK
# expected: OK
# (parens isolate the `cd` to a subshell — subsequent checks still run from
#  the repo root regardless of where the cargo metadata call landed.)

# 3. sqlx-cli available (for db-migrate / db-prepare)
cargo sqlx --version >/dev/null 2>&1 || sqlx --version >/dev/null 2>&1
# expected: zero exit (one of the two forms works)

# 4. all declared members exist on disk (paths from repo root)
for m in common api-core admin-core db integrations tenant-ops; do
  test -f "backend/crates/$m/Cargo.toml" || { echo "missing: $m"; exit 1; }
done
echo OK
# expected: OK
```

## Smoke check (single command)

```bash
cd backend && cargo check --workspace --message-format=short
```

> Note: first run downloads + compiles deps and can take several minutes;
> subsequent runs are fast. The verification harness times out individual
> smoke checks — if this one's never been run on the host, run it manually
> first to warm the cache.

## After-task verification

```bash
just check-backend && just test-backend
```

## Cross-references

- [`.research/implementer-prompt.md`](../../../.research/implementer-prompt.md)
- [`ppt-db-migrations`](../ppt-db-migrations/SKILL.md) — migrations + seed gap
- [`ppt-typespec`](../ppt-typespec/SKILL.md) — when changing API contracts
- [`ppt-tests`](../ppt-tests/SKILL.md) — narrower test filters
