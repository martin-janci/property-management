---
name: ppt-db-migrations
description: Apply / author SQLx migrations for the PPT databases — and document the missing-seed-recipe gap.
when_to_use: The plan adds a migration, changes the schema, or needs seeded data to repro.
mode: both
capabilities: [C2, C6]
tags: [backend, infra]
---

# PPT DB Migrations

This repo is **all-SQLx** (Rust). There is no Drizzle / JS migration tool —
the JS side consumes a generated client over HTTP, it doesn't talk to the
DB directly. Two separate migration trees exist for two separate
databases.

## When to invoke

- Plan adds a column / table / index → author a migration
- Plan needs seed data for integration repro → use in-crate seed helpers
- Plan changes a query → likely needs `just db-prepare` after migrating
  (see [`ppt-rust-backend`](../ppt-rust-backend/SKILL.md))

## What it gives you

- Migration locations (yes, plural — two DBs)
- The standard apply / author loop
- **Callout: there is NO `just seed` recipe** — what that means for you

## Inputs

- `postgres` reachable on `localhost:5432` (bring up via
  [`ppt-dev-stack`](../ppt-dev-stack/SKILL.md))
- `DATABASE_URL` exported in env (commonly `postgres://ppt:ppt@localhost:5432/ppt`
  — check `docker-compose.dev.yml` for the actual credentials)

## Migration locations

| DB | Migrations dir | Server | Notes |
|---|---|---|---|
| Main PPT DB | `backend/crates/db/migrations/` | `api-server`, `reality-server` | Numbered `NNNNN_<name>.sql` |
| Deploy server DB | `backend/servers/deploy-server/migrations/` | `deploy-server` | Independent — separate DB |

## Steps

1. **Apply pending migrations** to your local DB:
   ```bash
   just db-migrate
   # under the hood: cd backend/crates/db && sqlx migrate run
   ```
   For the deploy-server DB, migrations apply automatically on
   `deploy-server` start (or `cd backend/servers/deploy-server && sqlx
   migrate run`).
2. **Author a new migration:**
   ```bash
   just db-migration <name>
   # creates backend/crates/db/migrations/<ts>_<name>.sql
   # edit the file, then:
   just db-migrate
   ```
3. **Refresh SQLx offline data** (required so CI builds without a DB):
   ```bash
   just db-prepare
   # cd backend && cargo sqlx prepare --workspace
   ```
   Commit the regenerated `backend/.sqlx/*.json`.
4. **Seed for integration tests:** see the **gap callout** below.

## Seed gap — there is no `just seed`

`justfile` has **no `seed`, `db-seed`, or `seed-db` recipe**
(verified by `grep '^seed\|db-seed\|seed-db' justfile` → empty).
The seed code exists as a Rust module at `backend/crates/db/src/seed/`
(`data.rs`, `factories.rs`, `runner.rs`, `mod.rs`) and is consumed
in-process by tests and one-off binaries.

**Implications for the implementer:**

- Don't write `just seed` in PR bodies or test plans. It doesn't exist.
- For integration tests that need data, use the `seed` module's factories
  directly inside the test, or invoke `runner` from a small binary if the
  plan demands a CLI-style seed.
- If a plan claims a `seed` recipe exists, treat it as **stale evidence**
  (per implementer prompt § *Stale evidence aborts*) — file an issue,
  don't fabricate the recipe.
- Bridge users: `ppt_seed` only works if the host's `seed_command` is
  configured at `https://p.rlt.sk/accounts`. On hosts without it, the
  call errors cleanly.

If you decide to *add* a seed recipe, the minimum kit is:
1. A `seed.sql` (or `seed-bin` binary wrapping `backend/crates/db/src/seed/runner.rs`)
2. A `seed` recipe in `justfile`:
   ```
   seed:
       cd backend && cargo run --bin seed-bin
   ```
3. Optionally `just db-prepare` after, if the seed introduces new queries.

That change is **out of scope** for any plan that doesn't explicitly call it
out — file as a separate vector.

## Deterministic verification

```bash
# 1. sqlx-cli available
cargo sqlx --version >/dev/null 2>&1 || sqlx --version >/dev/null 2>&1
# expected: zero exit (one form works)

# 2. both migration dirs exist
test -d backend/crates/db/migrations && \
test -d backend/servers/deploy-server/migrations && echo OK
# expected: OK

# 3. seed module present (for in-test seeding)
test -f backend/crates/db/src/seed/runner.rs && echo OK
# expected: OK

# 4. confirm `just seed` truly does not exist (regression guard)
just --list 2>/dev/null | grep -E '^\s+(seed|db-seed|seed-db)\b' | wc -l
# expected: 0
```

## Smoke check (single command)

```bash
test -d backend/crates/db/migrations && test -d backend/servers/deploy-server/migrations && test -f backend/crates/db/src/seed/runner.rs
```

## After-task verification

```bash
just db-migrate && just db-prepare
```

## Cross-references

- [`ppt-rust-backend`](../ppt-rust-backend/SKILL.md) — workspace context
- [`ppt-dev-stack`](../ppt-dev-stack/SKILL.md) — bring `postgres` up first
- [`ppt-bridge-mcp`](../ppt-bridge-mcp/SKILL.md) — `ppt_seed` / `ppt_db_query`
  in cloud mode
