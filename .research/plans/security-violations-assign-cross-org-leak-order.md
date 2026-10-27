# security-violations-assign-cross-org-leak-order

**Vector:** security
**Score:** 3
**Source:** Issue #3187 | PR #3183 (introduced the failing test)
**Confidence:** high

## Hypothesis
`assign_violation` in `routes/violations.rs:337` runs `require_manager` and then checks `is_member(org_id, req.assigned_to)` on the acting user's own org BEFORE it ever queries the target violation. A manager in Org B calling `POST /violations/{org-A-violation}/assign` with any non-Org-B assignee uuid trips the member check and returns `403 Forbidden` instead of the tenant-isolating `404 Not Found` the rest of the mutation surface returns. That leaks resource existence across tenants and reds the `cross_org_assign_violation_is_not_found` test on `dev`, which blocks every backend PR's `test` check.

## Evidence
- Issue #3187 (2026-10-27, labels: bug, security, follow-up) — states the test asserts 404 but gets 403, pinpointing `assign_violation`.
- `backend/servers/api-server/src/routes/violations.rs:337-368` — the handler runs `require_manager` + `is_member` *before* `assign_violation`, so cross-org callers short-circuit to 403.
- `backend/servers/api-server/tests/suites/violations_mutation_cross_org_idor_tests.rs:250-262` — the failing test sends `{"assigned_to": Uuid::new_v4()}` (not in Org B) so the member check fires before any violation lookup.
- PR #3183 added the regression suite + partial 404 fix for other handlers; this one slipped through because the handler has an additional in-org guard the others don't.
- `backend.yml` shard-3 red on `dev` HEAD (noted on PR #3184).

## Files
- `backend/servers/api-server/src/routes/violations.rs:337`
- `backend/crates/db/src/repositories/violations.rs`
- `backend/servers/api-server/tests/suites/violations_mutation_cross_org_idor_tests.rs:250`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. From `backend/`, run `cargo test -p api-server --test suite_8 -- cross_org_assign_violation_is_not_found`.
2. Expected: 404 (per `assert_cross_org_404`). Actual: 403 — the status assertion fires `assertion failed: left: 403, right: 404` on the response status line.

## Suggested approach
1. In `routes/violations.rs:337 assign_violation`, reorder: look up the violation in the acting org *first* (via the repo's org-scoped fetch), returning 404 if it is not visible. Only after the violation is proven in-tenant run the `is_member(org_id, req.assigned_to)` check and return 403 for a bad assignee.
2. Prefer collapsing the two-call pattern into a single DB round-trip: extend `violation_repo.assign_violation(violation_id, org_id, assignee)` (or add a thin wrapper) so the UPDATE is gated on `(violation.org_id = $org AND assignee.org_id = $org)` and returns a typed error distinguishing "violation not found in org" vs "assignee not in org". Map the first to 404 via `map_mutation_err`, the second to 403.
3. Keep `require_manager` as the outer gate — role check stays first, tenancy check moves ahead of member check.
4. Re-run `cargo test -p api-server --test suite_8 -- cross_org_assign_violation_is_not_found` to confirm 404. Then re-run the full `violations_mutation_cross_org_idor_tests` module and `violations_role_authz_tests` to confirm no in-org regression (manager assigning a legitimate in-org member must still succeed → 200).
5. Grep the other mutation handlers in `violations.rs` for the same pre-existence membership-check pattern (there is no evidence it exists elsewhere, but confirm) — specifically `record_payment` and `add_evidence` which have assignee-like inputs.

## Alternatives considered
- **Keep the current order but change the member-check error to a 404** — rejected because it would leak that an Org-A violation *does* exist (403 vs 404 vs the real 200 path all become indistinguishable only for the cross-org read case; in-org invalid-assignee requests would then also return 404, which breaks the legitimate error signal to the UI for "you picked a non-member of your org").
- **Delete the test** — rejected: the test encodes the correct multi-tenant invariant. The test is right; the handler is wrong.

## Root-cause trace
1. Symptom: `cross_org_assign_violation_is_not_found` asserts 404, response is 403.
2. ← `routes/violations.rs:357` returns `forbidden_error("Cannot assign a violation to a user outside your organization")` on the `is_member` short-circuit before any org-scoped lookup of `violation_id`.
3. ← PR #3171 (`gate violations add_evidence on manager role`, 2026-10) and the subsequent assign-member check added the in-org assignee guard without reordering the existence check — the handler was always 403 for cross-org callers because the member check pre-dated the IDOR regression suite.
4. Origin: handler shape dates to the first `assign_violation` implementation; the test only landed in PR #3183, which is why it is now visible.

## Test plan
- [ ] `cargo test -p api-server --test suite_8 -- cross_org_assign_violation_is_not_found` passes (currently fails on `dev`).
- [ ] `cargo test -p api-server --test suite_8 -- violations_mutation_cross_org_idor_tests` passes (full module — no regression in sibling cases).
- [ ] `cargo test -p api-server --test suite_8 -- violations_role_authz_tests` passes (role-authz still intact — in-org manager assign still returns 200 to a valid in-org assignee).
- [ ] Exact command: `cd backend && cargo test -p api-server --test suite_8 -- violations_mutation_cross_org_idor_tests violations_role_authz_tests`.

## Out of scope
- Reworking the other `require_manager` + `is_member` orderings elsewhere in the codebase (reservations, maintenance). This plan is scoped to the violations surface and the one failing test.
- Changing the response schema or error envelope for the 403 path on in-org invalid assignees.

## After-merge
- Move this file to `plans/_archive/<slug>.md`
- Mark the matching `backlog.json` row as `status: "done"`
- Close Issue #3187 with a reference to the merging PR.
