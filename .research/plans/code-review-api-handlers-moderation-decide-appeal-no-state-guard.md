# code-review-api-handlers-moderation-decide-appeal-no-state-guard

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review [api-handlers] 2026-11-03
**Confidence:** high

## Hypothesis
`decide_appeal` in `api-server` validates the decision text and the rationale length but never loads the case to verify that an appeal was actually filed. A moderator can POST `/moderation/cases/{id}/appeal/decide` against any case in their org — even one where `appeal_filed = FALSE` or whose status is `Pending` / `Removed` / already `AppealApproved` — and the repo `UPDATE` succeeds unconditionally. That lets a moderator fabricate an appeal outcome (Art. 17 statement-of-reasons corruption) or overwrite the prior `appeal_decided_by` / `appeal_decided_at` on an already-decided case (audit forensics loss). Smallest fix mirrors the PR #3192 pattern on `file_appeal`: fetch the case, 409 unless `appeal_filed = TRUE AND status = Appealed`, and tighten the repo `WHERE` as defense in depth.

## Evidence
- `backend/servers/api-server/src/routes/aml_dsa/moderation.rs:584-630` — `decide_appeal` validates `req.decision` and `req.rationale` but calls `compliance_repo.decide_appeal(id, org_id, …)` with no prior load of the case and no state check.
- `backend/servers/api-server/src/routes/aml_dsa/moderation.rs:497-540` — `file_appeal` precedent: it fetches the case via `get_moderation_case` and refuses with 409 on `AppealApproved`/`AppealRejected`. `decide_appeal` has no analogue.
- `backend/crates/db/src/repositories/compliance.rs:864-895` — repo `UPDATE moderation_cases SET appeal_decision = $2, appeal_decided_by = $3, appeal_decided_at = NOW(), status = $4 WHERE id = $1 AND organization_id = $5` carries neither `appeal_filed = TRUE` nor `status = 'appealed'` in its `WHERE`.
- Reachability confirmed: `aml_dsa/mod.rs:84-87` wires `decide_appeal` at `POST /moderation/cases/:id/appeal/decide`; `routes/mod.rs:104` makes `aml_dsa` live.

## Files
- `backend/servers/api-server/src/routes/aml_dsa/moderation.rs:584`
- `backend/crates/db/src/repositories/compliance.rs:864`
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
1. Seed a moderation case for `org_a` in status `Pending` (no appeal filed). `appeal_filed` is `FALSE`.
2. As a moderator in `org_a`, POST `/moderation/cases/{id}/appeal/decide` with `{"decision":"upheld","rationale":"fabricated"}`. Expected: 409 CONFLICT (no appeal was filed). Observed: 200 OK — `appeal_decision = 'upheld'`, `appeal_decided_by = moderator.id`, `appeal_decided_at = now()`, `status = AppealApproved`. A fabricated Art. 17 statement-of-reasons payload is now persisted.
3. Repeat Repro Scenario B: seed a case in `AppealApproved` with `appeal_decided_by = Alice`, `appeal_decided_at = T0`. POST the decide-appeal endpoint again as moderator Bob. Observed: `appeal_decided_by` is overwritten to Bob and `appeal_decided_at` to now — the prior forensics trail is destroyed.

## Suggested approach
1. In `handlers_moderation.rs::decide_appeal` (`:584`), after `require_moderator_role` + `tenant_id` extraction + the existing `validate_appeal_decision` / `validate_text_field` calls, fetch the case: `let existing = state.compliance_repo.get_moderation_case(id, org_id).await?.ok_or((StatusCode::NOT_FOUND, "Moderation case not found".into()))?;`.
2. Add a state guard: `if !existing.appeal_filed || !matches!(existing.status, ModerationStatus::Appealed) { return Err((StatusCode::CONFLICT, "No pending appeal to decide".into())); }`. Mirror the shape of `file_appeal:530-539`.
3. (Optional defense in depth) Tighten the repo `UPDATE` in `compliance.rs:877-895` by appending `AND appeal_filed = TRUE AND status = 'appealed'` to the `WHERE`. On zero rows affected, return a repo-level `Error::NotFound` and 409 at the handler.
4. Leave the existing `write_compliance_audit` call (`:609-621`) in place — it already captures the decision payload for the DSA Art. 17 trail; the fix is to guard the entry point, not change the audit.
5. Add `tracing::warn!(case_id = %id, prior_status = ?existing.status, appeal_filed = existing.appeal_filed, user_id = %user.user_id, "decide_appeal rejected: no pending appeal")` on the 409 branch.
6. Run `cargo test -p api-server --test aml_dsa_authz_pap60_tests` and the new regression tests added in Step 7.
7. Add regression tests: `decide_appeal_without_filing_returns_409` (seed `Pending`, assert 409) and `decide_appeal_rejects_overwrite_of_decided_case` (seed `AppealApproved`, re-POST as different moderator, assert 409 + row's `appeal_decided_by`/`appeal_decided_at` unchanged).

## Alternatives considered
- **Only tighten the repo `UPDATE` `WHERE`** — rejected because the zero-rows-affected path cannot distinguish "wrong org" (should be 404) from "case has no filed appeal" (should be 409). The handler layer owns the HTTP status; the repo predicate is a defense-in-depth belt.
- **Flip `decide_appeal` to idempotent (silently no-op on terminal state, 200 OK with unchanged body)** — rejected because the EU DSA Art. 17 flow treats each appeal decision as a one-shot event; silent idempotence would let a moderator discover whether a given case had been previously decided without an explicit "already decided" signal, and would obscure the forensics-loss class in logs.

## Root-cause trace
1. Symptom: moderator POSTs `/moderation/cases/{id}/appeal/decide` on a case with `appeal_filed = FALSE`; the row's `appeal_decision`/`appeal_decided_by`/`appeal_decided_at` are populated and `status` flips to `AppealApproved`/`AppealRejected`, fabricating a complaint outcome.
2. ← `backend/servers/api-server/src/routes/aml_dsa/moderation.rs:584-612` — handler calls `compliance_repo.decide_appeal` without first fetching the case or checking `appeal_filed` / `status`.
3. ← `backend/crates/db/src/repositories/compliance.rs:864-895` — repo `UPDATE` carries no state predicate (`appeal_filed = TRUE AND status = 'appealed'`), so any row in the tenant can be flipped.
4. Origin: the sibling-handler guard added in PR #3192 (`code-review-api-handlers-appeal-no-status-guard`) only landed on `file_appeal`. `take_moderation_action` and `decide_appeal` were not updated and inherit the pre-guard permissiveness.

## Test plan
- [ ] Regression test in `backend/servers/api-server/tests/suites/aml_dsa_authz_pap60_tests.rs` — `decide_appeal_without_filing_returns_409` (seed `Pending`, POST decide-appeal, assert 409 + unchanged row).
- [ ] Regression test — `decide_appeal_rejects_overwrite_of_decided_case` (seed `AppealApproved` with prior `decided_by`/`decided_at`, re-POST, assert 409 + row unchanged).
- [ ] Regression test — `decide_appeal_happy_path_still_works` (seed `Appealed`, POST, assert 200 + status flip + audit entry).
- [ ] Exact command: `cargo test -p api-server --test aml_dsa_authz_pap60_tests decide_appeal` + `cargo test --workspace`.

## Out of scope
- The companion `take_moderation_action` gap is covered by a sibling plan (`code-review-api-handlers-moderation-take-action-no-state-guard`) — do not fold them into one PR.
- The `file_appeal` missing-audit gap (`code-review-api-handlers-moderation-file-appeal-no-audit`) is a separate backlog row.
- Any revision of the DSA Art. 17 statement-of-reasons payload shape or the appeal timing fields on `moderation_cases`.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-moderation-decide-appeal-no-state-guard.md`
- Mark the matching `backlog.json` row as `status: "done"`
