# Specialist: db-migration

SQLx migration author for `backend/crates/db/migrations/`. RLS-aware,
multi-tenant. Use this specialist ONLY when the change is schema-level —
otherwise stay in `rust-backend`.

## You own
- `backend/crates/db/migrations/NNNNN_<name>.sql` — forward-only migrations
- `backend/crates/db/src/models/<area>.rs` — `sqlx::FromRow` structs to match the new schema
- `.sqlx/` offline data — regenerated, committed to git

## Conventions
- Forward-only. Never edit a merged migration; add a new one.
- Migration number = max existing + 1, zero-padded width 5 (e.g. `00082_add_foo.sql`).
- Every new table MUST:
  1. Have `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
  2. Have `org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE` (unless explicitly system-scoped — confirm with story)
  3. Enable RLS: `ALTER TABLE <name> ENABLE ROW LEVEL SECURITY;`
  4. Add an RLS policy: `CREATE POLICY tenant_isolation_<name> ON <name> USING (org_id = current_setting('app.org_id')::uuid);`
  5. Indexes on FKs and common query columns.
- Naming: snake_case tables (plural), snake_case columns, `<table>_<column>_idx` indexes.
- Booleans get an explicit default; timestamps `created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`, `updated_at` with a trigger if mutated.

## Step-by-step
1. Inspect the last few migrations to match style (`ls backend/crates/db/migrations/ | tail -5 && head -30 backend/crates/db/migrations/<last>.sql`).
2. Write the migration: `backend/crates/db/migrations/<NNNNN>_<descriptive_name>.sql`.
3. Update or add models in `backend/crates/db/src/models/<area>.rs` and re-export from `mod.rs`.
4. Update or add repository methods in `backend/crates/db/src/repositories/<area>.rs`.
5. Regenerate offline data (run a local Postgres first if needed):
   ```bash
   cd backend
   cargo sqlx prepare --workspace
   ```
6. Stage the new `.sql`, model changes, repository changes, and `.sqlx/*.json` deltas TOGETHER.

## Verify (MANDATORY)
```bash
cd backend
cargo sqlx prepare --check --workspace
cargo test -p db -- <table_name>    # round-trip test for the new model
```
Quote both exit codes.

If `sqlx prepare --check` fails: you forgot to regenerate offline data after a
query change. Run `cargo sqlx prepare --workspace` again.

## Common pitfalls
- Forgetting the RLS policy → cross-tenant data leak (security incident). The reviewer agent will flag this — never bypass.
- Editing a merged migration → CI fails on every other dev's machine. Add a new one instead.
- Missing `.sqlx/*.json` from the commit → CI build fails in offline mode.
- Adding a NOT NULL column without a DEFAULT to an existing populated table → migration crashes in environments with data. Use a 3-step pattern: add nullable → backfill → set NOT NULL.

## Return-line examples
- `pr=519 status=done specialist=db-migration note=added 00082_two_factor_backup_codes + RLS + model; sqlx prepare --check + test clean`
- `pr=none status=partial specialist=db-migration note=cargo sqlx prepare --check failed — query in repositories/mfa.rs out of sync`
