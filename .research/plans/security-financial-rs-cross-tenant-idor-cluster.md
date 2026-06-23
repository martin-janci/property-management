# security-financial-rs-cross-tenant-idor-cluster

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review api-handlers 2026-06-23 · financial.rs:669 + financial.rs:693
**Confidence:** medium

## Hypothesis
`routes/financial.rs` has a two-handler IDOR cluster: `assign_unit_fee` (line 693) and `get_unit_fees` (line 669) both call `verify_org_access(state, auth.user_id, payload/query.organization_id)` — verifying only that the caller is a member of the org passed in the request payload/query — and then call the repo on the path `unit_id` (and, for `assign_unit_fee`, on `fee_schedule_id`) **without checking those ids belong to that org**. Any authenticated org-member can therefore (1) READ another tenant's unit fees by passing their own org_id in the query and the target's unit_id in the path, and (2) WRITE another tenant's unit fees the same way. The state-mutating write (`assign_unit_fee`) is the higher-severity defect because it creates rows for the attacker's `fee_schedule_id` against another tenant's `unit_id`. Fix shape: scope the repo queries by org_id (mirror `list_unit_invoices:1048`, which already JOINs to org). One combined PR addresses both.

## Evidence
- `backend/servers/api-server/src/routes/financial.rs:693` — `assign_unit_fee` verifies caller's membership in `payload.organization_id` (line 699) but then calls `financial_repo.assign_unit_fee(unit_id, payload.fee_schedule_id, …)` (line 700-708) — no org-membership check on the path `unit_id` or on `fee_schedule_id`.
- `backend/servers/api-server/src/routes/financial.rs:669` — `get_unit_fees` verifies caller's membership in `query.organization_id` (line 675) but then calls `financial_repo.get_unit_fees(unit_id, as_of)` (line 679-681) — no org join, unlike the canonical sibling `list_unit_invoices` at financial.rs:~1048.
- `backend/crates/db/src/repositories/financial.rs` (repository functions referenced by both handlers) — `get_unit_fees` issues a `SELECT … FROM unit_fees WHERE unit_id = $1 AND $2 BETWEEN effective_from AND effective_to`; `assign_unit_fee` issues an unscoped `INSERT INTO unit_fees (unit_id, fee_schedule_id, …)`. Neither query has an `organization_id` predicate or a JOIN to `units` / `fee_schedules` to verify org ownership.
- Same auth shape (`AuthUser` + `verify_org_access(…, payload/query.organization_id)`) is used by sibling handlers in the same file (`get_fee_schedules`, `create_fee_schedule`) that DO scope their repo queries by org — so the pattern is established; these two handlers regressed from it.

## Files
- `backend/servers/api-server/src/routes/financial.rs`
- `backend/crates/db/src/repositories/financial.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** no C4/C5 → cloud-ok.

Mode: cloud-ok

## Repro steps
1. Seed two organizations A and B, each with one unit (units `U_a` for A, `U_b` for B). Seed one `fee_schedule` `FS_a` in A.
2. As tenant A (a manager of A, JWT cookie set), call `POST /api/v1/financial/units/{U_b}/fees` with body `{"organization_id": "<A_id>", "fee_schedule_id": "<FS_a>", "override_amount": 999, "effective_from": "2026-01-01", "effective_to": "2027-01-01"}`.
3. Expected: `404 NOT_FOUND` or `403 FORBIDDEN` (A may not assign a fee to B's unit). Actual (on `dev`): `201 CREATED` — A wrote a row to `unit_fees` keyed on B's `U_b` and A's `FS_a`. Repeat with `GET /api/v1/financial/units/{U_b}/fees?organization_id=<A_id>` to confirm the same path leaks reads.

## Suggested approach
1. **Repo layer first** (`backend/crates/db/src/repositories/financial.rs`):
   - Change `get_unit_fees(unit_id, as_of)` → `get_unit_fees_for_org(unit_id, org_id, as_of)` and add `JOIN units u ON u.id = unit_fees.unit_id WHERE u.organization_id = $org_id` (or equivalent if the schema scopes via `fee_schedules` → org).
   - Change `assign_unit_fee(unit_id, fee_schedule_id, …)` → `assign_unit_fee_for_org(unit_id, fee_schedule_id, org_id, …)` and either (a) verify both `unit_id` and `fee_schedule_id` belong to `org_id` in one query before insert, or (b) use a `WITH … WHERE EXISTS` guard in the INSERT itself so a foreign `unit_id` yields 0 rows inserted → handler returns 404.
2. **Handler layer** (`routes/financial.rs:669, 693`):
   - In `get_unit_fees`, after `verify_org_access`, call `financial_repo.get_unit_fees_for_org(unit_id, query.organization_id, as_of)`. If the result is empty AND the caller's check passed, the handler should still return `[]` (no info leak about whether unit exists in other orgs).
   - In `assign_unit_fee`, after `verify_org_access`, call `financial_repo.assign_unit_fee_for_org(unit_id, payload.fee_schedule_id, payload.organization_id, …)`. Map the "0 rows inserted" path to `404 NOT_FOUND` (same shape as sibling secure handlers in the file).
3. Keep `verify_org_access` (it gates that the caller is a member of the claimed org) — but make org-scoping the **single source of truth** for resource ownership.
4. Run `backend/scripts/lints/check-discarded-principal.sh` (if present) over the touched file to confirm no other lurking handlers in this file have the same `_principal`/unscoped-id pattern; if any, add a follow-up issue (do NOT widen scope here).
5. Add the regression tests in *Test plan* below; verify they fail on `dev` before the fix and pass after.

## Alternatives considered
- **RLS only (rely on row-level security)** — rejected because both handlers run on `self.pool` (not an `RlsConnection`), so RLS is not in force on this path. Either we route through RLS (large refactor; touches every repo call site) or we add the explicit predicate. The latter is the smallest correct fix and matches the pattern already shipped for `ai/llm.rs` IDOR cluster (PR #766/#816) and the equipment/voice-device fixes.
- **Handler-only guard (check unit→org membership before calling repo)** — rejected because it doubles the DB roundtrips and races with concurrent modifications; the predicate belongs in the same query that reads/writes the row.

## Root-cause trace
1. Symptom: `POST /api/v1/financial/units/{foreign_unit_id}/fees` with attacker's `organization_id` returns 201 and writes a row.
2. ← `routes/financial.rs:700` calls `financial_repo.assign_unit_fee(unit_id, payload.fee_schedule_id, …)` with no org context.
3. ← `crates/db/src/repositories/financial.rs` `assign_unit_fee` issues `INSERT INTO unit_fees (unit_id, fee_schedule_id, …) VALUES (…)` with no `WHERE EXISTS …` ownership guard.
4. Origin: `routes/financial.rs` shipped with `verify_org_access(payload.organization_id)` as the SOLE auth check; the implementor assumed the org-membership check + a path id was sufficient (a recurring fallacy in IDOR-cluster handlers — see Issue #766, #519, and BIT-73 history).

## Test plan
- [ ] `backend/servers/api-server/tests/financial_unit_fee_idor_tests.rs` — new integration test: seed orgs A, B; as A, attempt (1) `GET /units/{U_b}/fees?organization_id=<A_id>` → expect 404 or empty result; (2) `POST /units/{U_b}/fees` with `organization_id=<A_id>` → expect 404; assert no row was inserted in `unit_fees` for `U_b`.
- [ ] `backend/crates/db/tests/financial_unit_fee_repo_tests.rs` — repo-layer contract test: seed A and B; call `get_unit_fees_for_org(U_b, <A_id>, …)` → expect empty; call `assign_unit_fee_for_org(U_b, FS_a, <A_id>, …)` → expect `Ok(None)` / `Err(NotFound)`.
- [ ] Command: `cd backend && cargo test -p api-server financial_unit_fee_idor && cargo test -p db financial_unit_fee_repo` (and confirm fmt + clippy still green).

## Out of scope
- Migrating all of `routes/financial.rs` to `RlsConnection` (separate refactor; see open hotspot `churn-hotspot-backend-servers-api-server-src-routes-financial-rs`).
- Sibling handlers in `routes/financial.rs` not flagged in this review (e.g. invoice/payment-matching flows already gated via the canonical pattern at `list_unit_invoices:1048` were not re-audited).
- The `code-review-api-handlers-messaging-nparty-block-bypass` finding (different file, different fix shape; carried in backlog for next promotion cycle).

## After-merge
- Move this file to `plans/_archive/security-financial-rs-cross-tenant-idor-cluster.md`
- Mark backlog rows `code-review-api-handlers-financial-assign-fee-cross-tenant` and `code-review-api-handlers-financial-get-unit-fees-leak` as `status: "done"`
