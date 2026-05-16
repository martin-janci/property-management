# Code Review #2 — RLS Policies & Database Migrations

**Branch:** `integration/multitenancy-phases-2-5p5`
**Migration range reviewed:** `00127_*` through `00140_*` (Phases 2–5.5)
**Reviewer:** Code Reviewer #2 (read-only audit)

---

## 1. Migration inventory

| #     | Title                                | Kind                | Idempotent? | RLS-enabled?    | Gotchas |
|-------|--------------------------------------|---------------------|-------------|-----------------|---------|
| 00127 | `users_principal_kind`               | DDL (column add)    | Yes (`IF NOT EXISTS`) | n/a (users is exempt) | DEFAULT `'staff'` is the right safe default; backfill scans `platform_admins` if present. |
| 00128 | `user_memberships`                   | DDL + RLS + DML backfill | Yes (`CREATE TABLE IF NOT EXISTS`, `ON CONFLICT … DO NOTHING`, `IF EXISTS` table guard) | YES | Backfill from `organization_members.role_type` falls back to `'member'` for empty strings — verify Phase 5 role catalog includes that. |
| 00129 | `principal_kind_guards`              | DDL (function + trigger) | Yes (`CREATE OR REPLACE`, `DROP TRIGGER IF EXISTS`) | n/a | Comment claims "RlsConnection clears all `app.*` GUCs" — actually `clear_request_context()` only clears 4 specific GUCs (see §3). The actual safety mechanism is the `set_config(_, _, TRUE)` transaction-local scope, which IS robust. |
| 00130 | `user_invites`                       | DDL + RLS           | Yes (`CREATE TABLE IF NOT EXISTS`, `DROP POLICY IF EXISTS`) | YES | No partial-unique on `(LOWER(email), organization_id) WHERE accepted_at IS NULL` — duplicate pending invites are possible (low impact, but worth noting). |
| 00131 | `user_merge_collisions`              | DDL + RLS           | Yes | YES (super-admin only) | Correct: queue-only, super-admin-gated. |
| 00132 | `merge_portal_users_into_users`      | DDL + DML (data)    | Yes (re-run-safe via `portal_origin_id` round-trip + collision-already-queued check) | n/a | `portal_origin_id` is **UNIQUE** (partial idx, NOT NULL pred). NO `ON CONFLICT DO UPDATE` — safe. SSO-only users get sentinel `'!sso-only-no-password'` (cannot validate as Argon2id — correct). |
| 00133 | `extend_agency_branding`             | DDL + RLS           | Yes (`ADD COLUMN IF NOT EXISTS`, `DROP POLICY IF EXISTS`) | YES (added here; was missing in 00108) | Pre-existing rows with NULL `organization_id` are super-admin-only — correct. New `agency_id` made nullable. |
| 00134 | `create_tenant_feature_flags`        | DDL + RLS + DML seed | Mostly (`CREATE TRIGGER` is NOT idempotent — re-run will fail; `CREATE POLICY` is also not idempotent here, no `DROP POLICY IF EXISTS`) | YES | **Idempotency gap:** re-running this migration on an existing schema will fail at the bare `CREATE TRIGGER` and `CREATE POLICY` statements. Only matters if a manual re-apply is attempted; sqlx tracks applied versions so production is fine. |
| 00135 | `listings_publish_state` + `reserved_platform_hosts` | DDL + DML backfill + DDL (lookup) | Mostly (`ADD COLUMN` lacks `IF NOT EXISTS`; `CREATE TABLE` lacks `IF NOT EXISTS`; `INSERT INTO reserved_platform_hosts` lacks `ON CONFLICT`) | **NO RLS on `reserved_platform_hosts`** (acceptable — global lookup, mutated only by super-admin paths) | `is_published` backfill is conservative (`status='active'`). Missing `IF NOT EXISTS` on `ALTER TABLE … ADD COLUMN is_published`. |
| 00136 | `listings_global_read_policy`        | RLS + helper fns    | Yes (`CREATE OR REPLACE`, `DROP POLICY IF EXISTS … listings_tenant_isolation … listings_four_context`) | YES (replaces existing) | `is_global_read_context()` is `STABLE` but **NOT** `SECURITY DEFINER` — fine because it only reads a session GUC. WITH CHECK correctly omits the global-read clause (read-only). `clear_request_context()` extended to clear `app.global_read`. |
| 00137 | `create_tenant_settings`             | DDL + RLS           | Yes (`IF NOT EXISTS` for table/indexes) — but `CREATE TRIGGER` and `CREATE POLICY` lack `IF NOT EXISTS` / `DROP IF EXISTS` (same gap as 00134) | YES | Two policies (`super_admin` for ALL, `tenant_read` for SELECT) — writes are explicitly app-layer-gated via `SettingsStore`. |
| 00138 | `create_capability_grants` + `two_factor_auth_verifications` + `impersonation_tokens` + audit_logs triggers | DDL + RLS + triggers | Mixed (`IF NOT EXISTS` on tables; `CREATE OR REPLACE` on functions; `DROP TRIGGER IF EXISTS` on the audit triggers; **but** bare `CREATE POLICY` on the new tables — same idempotency gap) | YES (all 3 new tables) | `granted_by != user_id` is **app-only** (no DB constraint). audit_logs append-only triggers added — good defense-in-depth. |
| 00139 | `organizations_soft_delete`          | DDL                 | Yes (`ADD COLUMN IF NOT EXISTS`, `CREATE INDEX IF NOT EXISTS`) | n/a | Clean. |
| 00140 | `rls_soft_delete_filter` (generated) | RLS bulk-update     | Yes (every `DROP POLICY IF EXISTS` + `CREATE POLICY`) | applies AND-clause only | **Stale-manifest bug — see §2 and §4.** |

**Sequencing & FK validity:** No gaps, no duplicates with 00001–00126. Every FK resolves at migration time (organizations, users, listings, agency_branding all pre-exist).

---

## 2. RLS policy correctness

### 2.1 Per-table verification of new tenant tables (00128–00138)

| Table | RLS on? | Policy shape | Soft-delete filter applied (00140)? |
|-------|---------|--------------|--------------------------------------|
| `user_memberships`            | YES (FORCE) | `is_super_admin() OR organization_id = get_current_org_id()` | **NO** ❌ |
| `user_invites`                | YES (FORCE) | same                                                          | **NO** ❌ |
| `user_merge_collisions`       | YES (FORCE) | `is_super_admin()`                                            | n/a (super-admin-only is fine) |
| `agency_branding`             | YES (FORCE) | super-admin OR (org_id IS NOT NULL AND match)                 | **NO** ❌ |
| `tenant_feature_flags`        | YES        | super-admin OR org_match                                      | **NO** ❌ |
| `reserved_platform_hosts`     | NO          | (global lookup, no policy intentional)                        | n/a |
| `listings`                    | YES (existing FORCE) | `super_admin OR org_match OR (is_published AND is_global_read_context())` (00136 four-context) | **PARTIAL — see §2.3** ❌ |
| `tenant_settings`             | YES        | super-admin (ALL) + tenant_read (SELECT)                      | **NO** ❌ |
| `capability_grants`           | YES        | super-admin only                                              | n/a (super-admin-only) |
| `two_factor_auth_verifications` | YES      | super-admin + own-row SELECT + open-INSERT                    | n/a (per-user, not per-org) |
| `impersonation_tokens`        | YES        | super-admin only                                              | n/a |

### 2.2 Phase 4 listings four-context policy (00136) — verified correct

```sql
USING (is_super_admin() OR organization_id = get_current_org_id()
       OR (is_published AND is_global_read_context()))
WITH CHECK (is_super_admin() OR organization_id = get_current_org_id())
```

WITH CHECK correctly omits the global-read clause — the platform host is read-only. ✅

`is_global_read_context()` is `STABLE`, not `SECURITY DEFINER` — acceptable (reads only a GUC). The fail-closed default (`COALESCE(..., FALSE)` + `EXCEPTION WHEN OTHERS THEN RETURN FALSE`) is correct. `clear_request_context()` was extended to also reset `app.global_read` — defense for leak #2 confirmed. ✅

### 2.3 Phase 5.5 soft-delete sweep (00140) — **CRITICAL DEFECT**

**Helper function:** `get_current_org_not_deleted()` is `STABLE` (good) but **NOT** `SECURITY DEFINER`. It reads `organizations` directly. Because `organizations` has **no RLS** (intentionally — see comment in 00006), this works today. But if RLS is ever enabled on `organizations`, this helper will silently start returning FALSE in tenant contexts, locking out every tenant. Documenting this as a latent risk; a defensive `SECURITY DEFINER SET search_path = public` would future-proof.

**Coverage gap (the big one):** 00140 was generated against `tenant-data-manifest.json` whose `git_sha` field is `2c56505f` (Phase 0+1 base, `migration_count: 127`). At that point, **none of the Phase 2–5 tenant tables existed yet**. The actual repo now has 140 migrations, but the manifest still shows 127 and the table list omits every table created in 00128–00138.

Tables created by Phases 2–5 that are tenant-scoped but **not** in 00140:

- `user_memberships` (org-scoped via `organization_id`)
- `user_invites` (org-scoped)
- `agency_branding` (org-scoped after 00133)
- `tenant_feature_flags` (org-scoped)
- `tenant_settings` (org-scoped)

Every one of these will continue serving rows from a soft-deleted organization in tenant context → **Defense #16 (leak #16, "soft-delete bypass") is incomplete**.

**Listings double-policy bug:** 00140 emits

```sql
DROP POLICY IF EXISTS listings_tenant_isolation ON listings;
CREATE POLICY listings_tenant_isolation ON listings FOR ALL
    USING ((is_super_admin() OR organization_id = get_current_org_id())
           AND get_current_org_not_deleted())
    WITH CHECK (...);
```

But 00136 already replaced `listings_tenant_isolation` with `listings_four_context`. Net effect after migrating from a fresh DB:

1. `listings_four_context` exists (from 00136) — **without** soft-delete filter.
2. `listings_tenant_isolation` is created **anew** by 00140 — **without** the four-context global-read OR-clause.

Two permissive `FOR ALL` policies on the same table OR together. Effective USING:

```
super_admin OR org_match OR (is_published AND is_global_read_context())   -- four_context, no soft-delete filter
   OR ((super_admin OR org_match) AND get_current_org_not_deleted())      -- new policy, with soft-delete
```

Since the four_context branch alone returns TRUE for any matching org (regardless of deleted_at), **a soft-deleted org's listings remain visible to tenant queries**. Defense #16 is bypassed for `listings`. The only saving grace is that 00136's WITH CHECK doesn't have the soft-delete clause either, so writes still work the same way (and the `listings_tenant_isolation` WITH CHECK is also OR'd, so writes also pass for soft-deleted orgs).

**Root cause:** the manifest was committed in 0f81eb5d (Phase 5.5, on top of 2c56505f which predates 00136), `00140` was generated (d068739b) before Phase 4's `00136` was merged in. The integration merge brought the two together but **did not regenerate either artifact**.

### 2.4 Idempotency notes

- 00134, 00137, 00138 all create policies with bare `CREATE POLICY`. If a developer ever drops the migration version from `_sqlx_migrations` and re-runs, it will fail with `policy already exists`. Low-risk but easy to fix (`DROP POLICY IF EXISTS` first).
- 00135 lacks `IF NOT EXISTS` on `ALTER TABLE listings ADD COLUMN is_published` and `CREATE TABLE reserved_platform_hosts`, and lacks `ON CONFLICT` on the seed inserts.

---

## 3. Principal-kind guard analysis (Phase 2 / migration 00129)

**Trigger mechanism:**

```sql
BEFORE UPDATE OF principal_kind ON users
FOR EACH ROW EXECUTE FUNCTION users_principal_kind_guard()
```

Function rejects unless `current_setting('app.principal_kind_change_authorized', TRUE) = 'true'`. The mutator function `set_principal_kind()` arms the GUC with `set_config(_, _, TRUE)` — `TRUE` here means **transaction-local scope**.

**Robustness assessment:**

1. ✅ Transaction-local GUC means a malicious caller cannot pre-arm the flag from a separate connection / pooled session — the flag dies at `COMMIT/ROLLBACK`.
2. ✅ `SECURITY DEFINER` on `set_principal_kind()` correctly elevates only that audited path.
3. ⚠️ The function's docstring claims "RlsConnection clears all `app.*` GUCs on release" — this is **false**. `clear_request_context()` (00006, extended in 00136) only clears a fixed allowlist of GUCs and does not include `app.principal_kind_change_authorized`. The transaction-local scope makes the docstring lie not exploitable, but it's misleading and should be corrected.
4. ⚠️ A direct psql connection with `BEGIN; SET LOCAL app.principal_kind_change_authorized = 'true'; UPDATE users SET principal_kind = 'platform' WHERE id = …; COMMIT;` bypasses the audit trail. Only mitigated by:
   - DB role hardening (no production role should have direct UPDATE on `users.principal_kind`).
   - The CHECK constraint on principal_kind values.
   - Audit logging from outside the DB layer.
   This is a **known-failure mode** of the GUC pattern and should be documented in the threat model.
5. ✅ Trigger fires on `BEFORE UPDATE OF principal_kind` (column-specific) — not bypassed by mass-assignment unless principal_kind is included.
6. ✅ `IS DISTINCT FROM` short-circuits no-op updates — UPDATE statements that touch other columns won't trip the guard.

**Confidence:** Medium-high for application-mediated traffic. Low for direct DB-role attack — the guard is bypassable by anyone with a privileged psql connection.

---

## 4. Manifest coverage (defense #17)

`backend/manifests/tenant-data-manifest.json`:

- **`manifest_version`:** 1
- **`git_sha`:** `2c56505f` (Phase 0+1 base — pre-Phase 2)
- **`migration_count`:** 127 (actual: **140**)
- **Tables listed:** 367 unique (369 entries — verify dupes)
- **`generated_at`:** `2026-05-15T07:41:21Z`

**Tables in the schema but missing from manifest:**

| Table | Created by | Tenant-scoped? | Visible to purge/export? |
|-------|-----------|----------------|---------------------------|
| `user_memberships`           | 00128 | YES (`organization_id`) | **NO** ❌ |
| `user_invites`               | 00130 | YES (`organization_id`) | **NO** ❌ |
| `tenant_feature_flags`       | 00134 | YES (`organization_id`) | **NO** ❌ |
| `tenant_settings`            | 00137 | YES (`organization_id`) | **NO** ❌ |
| `agency_branding` (post-00133) | 00133 (added `organization_id`) | YES (org-scoped after 00133) | **NO** ❌ |
| `user_merge_collisions`      | 00131 | super-admin only | n/a (correctly excluded) |
| `capability_grants`          | 00138 | super-admin only | n/a |
| `impersonation_tokens`       | 00138 | super-admin only | n/a |
| `two_factor_auth_verifications` | 00138 | per-user | n/a (could be considered for purge — no `organization_id` so check-rls-coverage's FK walk would skip it) |
| `reserved_platform_hosts`    | 00135 | global lookup | n/a |

**Direct consequence:** `tenant-ops` `purge` and `export` routines (per the runbook in `docs/multitenancy/operability.md`) read this manifest as the single source of truth. A tenant purge today **would silently leave behind**:

- the org's user_memberships rows
- pending user_invites for the org
- the org's tenant_feature_flags settings
- the org's tenant_settings rows
- the org's agency_branding row

This is exactly the leak #17 that the manifest gate was designed to prevent.

**CI gate dry-read of `check-rls-coverage.sh`:**

- Logic walks FK graph from `organization_id`/`org_id` columns. ✅ — would correctly identify all 5 missing tables IF re-run.
- `--strict` exits 1 on uncovered tables. ✅
- `--emit-manifest` writes JSON. ✅ — but the manifest in the repo was last regenerated at `migration_count = 127`. There is **no CI step pinning the manifest's `migration_count` to the actual file count** to detect staleness. Recommend: a tiny check in CI (`test "$(ls migrations | wc -l)" == "$(jq .migration_count manifest.json)"`) plus running `--emit-manifest` and `git diff --exit-code` to fail if the committed manifest is stale.

---

## 5. Verdict

**Counts:** ✅ 8 — ⚠️ 5 — ❌ 3

**Pass ✅ items:**

1. Migration sequencing 00127–00140 is correct: no gaps, no overlaps with 00126, FKs valid, every new tenant table has RLS enabled.
2. `00132` portal_users merge is collision-safe (no `ON CONFLICT DO UPDATE`, queue-on-collision via `00131`).
3. `users.portal_origin_id` is UNIQUE (partial index) and re-runs are no-ops.
4. `is_global_read_context()` is `STABLE` and fail-closed.
5. `clear_request_context()` was correctly extended to clear `app.global_read` (defense #2).
6. Phase 4 `listings_four_context` policy WITH CHECK correctly excludes the global-read OR-clause (read-only invariant).
7. Capability self-grant rejected at the application layer (`admin-core::capability::grant`).
8. `audit_logs` append-only triggers in 00138 close the super-admin/owner mutation gap.

**Concerns ⚠️ items:**

1. Migrations 00134, 00137, 00138 use bare `CREATE POLICY` / `CREATE TRIGGER` (not strictly idempotent). Low-impact (sqlx tracks versions) but easy to harden.
2. 00135 uses bare `ALTER TABLE … ADD COLUMN`, `CREATE TABLE`, and seed `INSERT` without `IF NOT EXISTS` / `ON CONFLICT`.
3. 00130 lacks a partial-unique index on pending invites (duplicate pending invites possible).
4. `set_principal_kind()` guard is bypassable by direct DB-role access via `SET LOCAL`. Comment in 00129 is misleading about `clear_request_context()` clearing the GUC.
5. Capability `granted_by != user_id` is enforced **only** at the application layer — no DB CHECK constraint. A bug or a future direct DB write path would bypass leak #21 defense.

**Failures ❌ items (top 3, in priority order):**

1. **`tenant-data-manifest.json` is stale (`migration_count=127` vs actual 140; `git_sha=2c56505f`).** It is missing every Phase 2–5 tenant-scoped table: `user_memberships`, `user_invites`, `agency_branding`, `tenant_feature_flags`, `tenant_settings`. The Phase 5.5 `purge` / `export` routines that read this manifest will silently leave behind tenant data. **Action:** regenerate via `bash backend/scripts/check-rls-coverage.sh --emit-manifest backend/manifests/tenant-data-manifest.json` after Phase 5.5 integration; add a CI staleness check.

2. **Migration 00140 does not include the soft-delete `AND get_current_org_not_deleted()` filter for the 5 new Phase 2–5 tenant tables** (because it was generated against the stale manifest). A soft-deleted organization's `user_memberships`, `user_invites`, `agency_branding`, `tenant_feature_flags`, and `tenant_settings` remain visible to tenant queries. **Action:** regenerate 00140 (or write 00141) after refreshing the manifest.

3. **Migration 00140 references the obsolete policy name `listings_tenant_isolation`** and creates it as a *new* policy alongside 00136's `listings_four_context`. Two `FOR ALL` permissive policies OR together; the four_context branch lacks the soft-delete filter, so **listings of a soft-deleted org remain visible** in tenant context. **Action:** regenerate 00140 — the generator's introspection of `pg_policies` will find the post-00136 name.

All three failures share the same root cause: the manifest and the generated migration were produced before Phase 4 was integrated, and were not refreshed at integration-merge time.
