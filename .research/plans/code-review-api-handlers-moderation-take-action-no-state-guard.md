# code-review-api-handlers-moderation-take-action-no-state-guard

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review [api-handlers] 2026-11-03
**Confidence:** high

## Hypothesis
`take_moderation_action` in `api-server` has no case-status guard: a moderator can POST a `Remove` (or any other) action against a moderation case whose appeal has already been finally decided (`AppealApproved` / `AppealRejected`), and the repo `UPDATE` succeeds — silently erasing the final appeal decision and re-closing the case in the moderator-favoring direction. PR #3192 just added this exact guard to `file_appeal`, but the sibling action handler was not covered. The smallest fix is to mirror the PR #3192 pattern: load the case up front, 409-reject if `status` is already a terminal `appeal_*` state, and tighten the repo `UPDATE` with a `status NOT IN (…)` predicate as defense in depth.

## Evidence
- `backend/servers/api-server/src/routes/aml_dsa/moderation.rs:409-467` — `take_moderation_action` validates rationale/role but never fetches the current `ModerationCase` to check `status`.
- `backend/servers/api-server/src/routes/aml_dsa/moderation.rs:520-540` — `file_appeal` already runs a `matches!(existing.status, AppealApproved | AppealRejected) → 409` guard (merged in PR #3192) that the sibling handler is missing.
- `backend/crates/db/src/repositories/compliance.rs:786-826` — `take_moderation_action` repo function runs `UPDATE moderation_cases SET status = $2, decision = $3, … WHERE id = $1 AND organization_id = $7` with no `status NOT IN ('appeal_approved','appeal_rejected')` predicate.
- Reachability confirmed: `aml_dsa/mod.rs:84-87` wires `take_moderation_action` into the router; `routes/mod.rs:104` makes `aml_dsa` live.

## Files
- `backend/servers/api-server/src/routes/aml_dsa/moderation.rs:409`
- `backend/crates/db/src/repositories/compliance.rs:786`
- `backend/servers/api-server/tests/suites/aml_dsa_authz_pap60_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [x] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Seed a moderation case for `org_a` with a moderated content item.
2. As a moderator in `org_a`, POST `/moderation/cases/{id}/action` with `{"action":"Remove","rationale":"…"}` — case transitions to `Removed`.
3. As the content owner, POST `/moderation/cases/{id}/appeal` with a reason — case transitions to `Appealed`.
4. As a moderator, POST `/moderation/cases/{id}/appeal/decide` with `{"decision":"upheld","rationale":"…"}` — case transitions to `AppealApproved`.
5. As a moderator, POST `/moderation/cases/{id}/action` AGAIN with `{"action":"Remove","rationale":"re-close"}` — expected: 409 CONFLICT. Observed: 200 OK, `moderation_cases.status` flips from `AppealApproved` back to `Removed`, the appeal decision is erased.

## Suggested approach
1. In `handlers_moderation.rs::take_moderation_action` (`:409`), after `require_moderator_role` + `tenant_id` extraction, call `state.compliance_repo.get_moderation_case(id, org_id).await?` (same helper `file_appeal:497` already uses).
2. Add a guard immediately after: `if matches!(existing.status, ModerationStatus::AppealApproved | ModerationStatus::AppealRejected) { return Err((StatusCode::CONFLICT, "This case has a final appeal decision and cannot be re-moderated".into())); }`. Mirror the wording / 409 shape used in `file_appeal:530-539` for consistency.
3. (Optional defense in depth) Narrow the repo `UPDATE` in `compliance.rs:803-826` by adding `AND status NOT IN ('appeal_approved','appeal_rejected')` to the `WHERE`; map a zero-row-affected result to the same 409 at the handler layer. Keep the handler guard — the repo predicate alone is silent on no-op.
4. If the case is already in a non-terminal non-appealed state (e.g. `Pending`, `UnderReview`), keep the existing happy path.
5. Add tracing on the 409 branch: `tracing::warn!(case_id = %id, prior_status = ?existing.status, user_id = %user.user_id, "take_moderation_action rejected: case has final appeal decision")`.
6. Run `cargo test -p api-server --test aml_dsa_authz_pap60_tests` and the new regression test added in Step 7.
7. Add a regression test to `aml_dsa_authz_pap60_tests.rs` (or `compliance_wave1b_tests.rs` if the authz file is reviewer-closed): seed `AppealApproved`, POST `/moderation/cases/{id}/action`, assert 409 + unchanged row.

## Alternatives considered
- **Only tighten the repo `UPDATE` predicate** — rejected because a silent `rowcount == 0` is harder to distinguish from "case not found in this org" and the handler can't return the correct 409 vs 404 without an extra round-trip. The handler-side guard gives a single clear 409 and the exact `prior_status` for the audit trail.
- **Lift the guard into a shared `assert_case_mutable()` helper used by all three handlers** — rejected for scope: the companion `decide_appeal` gap (sibling plan) and this one touch overlapping but distinct state transitions, so premature consolidation risks a wrong-shaped helper. The follow-up refactor is cheap once both land.

## Root-cause trace
1. Symptom: moderator re-POSTs `/moderation/cases/{id}/action` on a case in `AppealApproved`; the row's `status` flips back to the moderator's choice and the appeal decision is erased.
2. ← `backend/servers/api-server/src/routes/aml_dsa/moderation.rs:409-467` — handler calls `compliance_repo.take_moderation_action` without first checking the case's current status.
3. ← `backend/crates/db/src/repositories/compliance.rs:803-826` — repo `UPDATE` carries no `status NOT IN ('appeal_approved','appeal_rejected')` predicate, so it unconditionally overwrites any state.
4. Origin: the `file_appeal` sibling guard was introduced in PR #3192 (`code-review-api-handlers-appeal-no-status-guard`) but did not propagate to `take_moderation_action` or `decide_appeal`. Those two handlers were untouched by #3192 and inherit the pre-guard permissiveness.

## Test plan
- [ ] Regression test in `backend/servers/api-server/tests/suites/aml_dsa_authz_pap60_tests.rs` — `take_moderation_action_on_appeal_approved_returns_409` (seed `AppealApproved`, call handler, assert 409 + DB row unchanged).
- [ ] Regression test — same shape against `AppealRejected`.
- [ ] Repo-level test in `compliance.rs`'s unit tests (if present) asserting the `UPDATE` is a no-op on terminal appeal states (defensive).
- [ ] Exact command: `cargo test -p api-server --test aml_dsa_authz_pap60_tests take_moderation_action` + `cargo test --workspace`.

## Out of scope
- The companion `decide_appeal` gap is covered by a sibling plan (`code-review-api-handlers-moderation-decide-appeal-no-state-guard`) — do not fold them into one PR.
- The `file_appeal` missing-audit gap (`code-review-api-handlers-moderation-file-appeal-no-audit`) is a separate backlog row; leave the audit emission unchanged in this PR.
- Any wider refactor of `require_moderator_role` or audit-emission wrappers.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-moderation-take-action-no-state-guard.md`
- Mark the matching `backlog.json` row as `status: "done"`
