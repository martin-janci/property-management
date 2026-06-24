# code-review-api-handlers-block-user-cross-tenant

**Vector:** security
**Score:** 3
**Source:** PR #1768 (recent messaging churn) + rotating-expert-review 2026-06-24 (api-handlers segment)
**Confidence:** high

## Hypothesis

The `POST /api/v1/messages/users/{id}/block` handler at `routes/messaging.rs:1272` accepts a `user_id_to_block` UUID from the path and writes a row keyed on `(blocker_id = caller, blocked_id = path UUID, organization_id = caller_tenant)` without first verifying the path UUID belongs to the caller's tenant. This is asymmetric with `start_thread` (lines 383–411), which explicitly cross-checks every recipient against `organization_members` for the caller's tenant and returns `403 CROSS_ORG_DENIED` on mismatch. The smallest fix is to lift the same `same_tenant` membership query into `block_user` (and `unblock_user`) before invoking the repository, so a resident in tenant A cannot create a block entry naming a user in tenant B.

## Evidence

- `backend/servers/api-server/src/routes/messaging.rs:1272-1298` — `block_user` reads `tenant_id = rls.tenant_id()` but never validates that `user_id_to_block` is a member of that tenant before calling `repo.block_user_rls(...)`.
- `backend/servers/api-server/src/routes/messaging.rs:336-411` — `start_thread` shows the canonical pattern: query `organization_members` for the caller's `tenant_id` over the candidate set, then reject with `CROSS_ORG_DENIED` if any recipient is not a member.
- `backend/servers/api-server/src/routes/messaging.rs:1334-1351` — `unblock_user` has the same shape and the same gap (it doesn't even read `tenant_id`).
- rotating-expert-review 2026-06-24, segment `api-handlers`, finding `code-review-api-handlers-block-user-cross-tenant` — security expert traced the call path from route registration (line 250) through the handler to the repository write.

## Files

- `backend/servers/api-server/src/routes/messaging.rs:1272`
- `backend/servers/api-server/src/routes/messaging.rs:1334`
- `backend/servers/api-server/tests/messaging_attachments_authz_tests.rs`

## Dependencies

(none)

## Required capabilities

- [x] C1 — Systematic debugging
- [x] C6 — Verification before completion
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. Seed two tenants A and B with one resident each (`userA` in A, `userB` in B).
2. As `userA`, call `POST /api/v1/messages/users/{userB.id}/block`.
3. Expected: `403 CROSS_ORG_DENIED` (consistent with `start_thread`). Actual: `200 OK` and a `user_blocks` row is created with `blocker_id=userA`, `blocked_id=userB`, `organization_id=tenantA`.

## Suggested approach

1. In `backend/servers/api-server/src/routes/messaging.rs:1272` (`block_user`), after the self-block guard and before `repo.block_user_rls`, run the same `SELECT user_id FROM organization_members WHERE organization_id = $1 AND user_id = ANY($2) AND status = 'active'` query used at line 383, scoped to `[user_id_to_block]` and the caller's `tenant_id`.
2. If the resulting row count is `0`, release `rls`, return `403 CROSS_ORG_DENIED` with the same error shape `start_thread` uses.
3. Apply the same change to `unblock_user` at line 1334 — it currently doesn't even read `tenant_id`; add `let tenant_id = rls.tenant_id();` and the same cross-tenant gate.
4. Optional but recommended: extract the `same_tenant` membership check into a helper near `require_thread_participant` (line 1762) to avoid drift the next time another handler needs it.
5. Add a behavioral regression test in `backend/servers/api-server/tests/messaging_attachments_authz_tests.rs` (or a sibling `messaging_block_authz_tests.rs`): two-tenant fixture, attempt cross-tenant block, assert `403 CROSS_ORG_DENIED` and that no row landed in `user_blocks`.
6. Run `cargo test -p api-server --test messaging_attachments_authz_tests` and `cargo clippy --workspace -- -D warnings`.

## Alternatives considered

- **Enforce via DB-level RLS on `user_blocks.blocked_id`** — rejected because there is no `users.organization_id` column (membership lives in `organization_members`), so a policy referencing `blocked_id` would need a correlated subquery on every write. Handler-level validation matches the rest of the messaging routes and is easier to test.
- **Silently ignore cross-tenant blocks (return 200, no row written)** — rejected because returning `200` on a request the user wasn't authorized to make is a behavioral-divergence trap: clients would assume the block landed. The asymmetry with `start_thread` should be removed by aligning on the explicit `403`, not papered over.

## Root-cause trace

1. Symptom: cross-tenant `POST /messages/users/{id}/block` succeeds (write to `user_blocks` with `organization_id = caller tenant`, `blocked_id = arbitrary UUID`).
2. ← `block_user` handler at `backend/servers/api-server/src/routes/messaging.rs:1289-1298` calls `repo.block_user_rls` with `organization_id: tenant_id` (caller's), but `blocked_id` is whatever the path supplied.
3. ← The repository write `block_user_rls` (in `backend/crates/db/src/repositories/messaging.rs`) accepts `CreateBlock { blocker_id, blocked_id, organization_id }` and inserts without re-checking that `blocked_id` is in `organization_id`.
4. ← The same pattern is correctly guarded in `start_thread` at line 383 (`organization_members` membership query) but that guard was never propagated to the block/unblock route added later (BIT-183 messaging hardening).
5. Origin: handler added with PR for messaging block/unblock (predates the recent N-party hardening in PRs #1768/#1802 — the recipient guard was added to `start_thread` but block/unblock kept the older shape).

## Test plan

- [ ] New behavioral test in `backend/servers/api-server/tests/` that seeds two tenants, attempts a cross-tenant block, and asserts `403 CROSS_ORG_DENIED` + no `user_blocks` row.
- [ ] Regression test for `unblock_user` parity: same fixture, attempt cross-tenant unblock, assert `403`.
- [ ] Run: `cargo test -p api-server --test messaging_attachments_authz_tests` (or the new sibling file). Confirm the new tests fail on `dev` before the fix lands.

## Out of scope

- Reworking `start_thread`'s recipient validation into a generic middleware. The block/unblock handlers cover the immediate gap; broader refactor can land separately if other routes show the same drift.
- Migrating `user_blocks` to RLS-by-`organization_id`. Handler validation is sufficient for the IDOR; a DB-level guard is a defense-in-depth follow-up.

## After-merge

- Move this file to `plans/_archive/code-review-api-handlers-block-user-cross-tenant.md`
- Mark `backlog.json` row `code-review-api-handlers-block-user-cross-tenant` as `status: "done"`
