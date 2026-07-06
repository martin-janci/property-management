# bug-regional-compliance-fake-vote-validation

**Vector:** bug
**Score:** 3
**Source:** hotspot in `backend/servers/api-server/src/routes/regional_compliance.rs`
**Confidence:** medium

## Hypothesis
`validate_slovak_vote` and `validate_czech_vote` in `routes/regional_compliance.rs` silently substitute a `75%` participation and `80%` approval default when the underlying vote data is missing or malformed (deserialize failure, empty `questions`, no matching yes-option, or f64→Decimal conversion failure). Both defaults exceed the seeded simple-majority quorum (`50.01%`), so the endpoints return `is_valid: true` with a `legal_reference` — a legally-styled compliance signal — for votes where the system has *no evidence* of any voter data. The right fix is to fail closed: return an error (or `is_valid: false` with a `reason: insufficient_data`) whenever the input is not concrete, and pin that behavior with a regression test.

## Evidence
- `backend/servers/api-server/src/routes/regional_compliance.rs:253-259` — `actual_participation = 75.00%` fallback when `eligible_count == 0` in `validate_slovak_vote`.
- `backend/servers/api-server/src/routes/regional_compliance.rs:261-279` — `approval_percentage` defaults to `80.00%` when the vote's `results` JSON fails to deserialize, has no `questions`, has no matching yes-option, or the first-option f64 percentage cannot convert to `Decimal`.
- `backend/servers/api-server/src/routes/regional_compliance.rs:803-829` — identical shape in `validate_czech_vote` (same 75%/80% defaults), returning `CzechVoteValidation { is_valid: true }` with a `legal_reference` on malformed input.
- Reviewer note (`.research/signals/2026-07-06.json` → `code-review-api-handlers-fake-vote-fallback`): rotating expert review flagged this while inspecting the churn cluster around PRs #2099 / #2117.
- Class parallel: PR #2030 / #2086 / #2117 killed the same "looks complete but isn't" class in `SlovakAccountingExport`; this handler is where the class relocated.

## Files
- `backend/servers/api-server/src/routes/regional_compliance.rs:253`
- `backend/servers/api-server/src/routes/regional_compliance.rs:803`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (derived from ticks above — C4 / C5 both unticked):**

Mode: cloud-ok

## Repro steps
1. Seed a `voting_sessions` row where `results` JSON is either `null`, `{}`, or a string that fails serde deserialization; leave `eligible_count = 0` and `questions = []`.
2. Call `POST /api/v1/regional-compliance/votes/{id}/validate-slovak` with a `jurisdiction_rules` payload whose `quorum_required = 50.01` and `approval_required = 50.01`.
3. Expected (after fix): HTTP 422 with `{ is_valid: false, reason: "insufficient_data" }` (or equivalent error variant). Actual (today): HTTP 200, `SlovakVoteValidation { is_valid: true, actual_participation: 75.00, approval_percentage: 80.00, legal_reference: "Zákon o vlastníctve bytov …" }`.

## Suggested approach
1. Add an error variant to the existing route error type (e.g. `RegionalComplianceError::InsufficientVoteData { reason }`), returning `StatusCode::UNPROCESSABLE_ENTITY` (422).
2. In `validate_slovak_vote` (`routes/regional_compliance.rs:230-290` region): replace the `eligible_count == 0` branch and every `.unwrap_or(80.0)` / silent-default arm with an explicit `Err(InsufficientVoteData { reason: "…" })`. Enumerate the failure cases: (a) `eligible_count == 0`, (b) `serde_json::from_value(vote.results)` errors, (c) `results.questions` empty, (d) no option text matches the yes-marker set, (e) f64→Decimal conversion fails.
3. Apply the same treatment to `validate_czech_vote` (`routes/regional_compliance.rs:803-829` region) — identical shape, symmetrical fix.
4. Keep the "happy path" arithmetic unchanged; only the fallback paths flip from silent-default to fail-closed.
5. Add a `#[utoipa::path(...)]` response variant for the 422 so the OpenAPI + generated clients pick it up. Regenerate via `pnpm --filter @ppt/api-client generate` (or the standard TypeSpec pipeline).
6. Add a doc comment on both handlers linking back to this class ("compliance evidence must be verifiable — see `SlovakAccountingExport::new` for the type-level pattern that killed the sibling class").
7. Verify with `cargo clippy -p api-server --all-targets -- -D warnings` and the tests below.

## Alternatives considered
- **Return `is_valid: false` with a soft `insufficient_data` reason on the 200 shape** — rejected because compliance clients are known to log-and-continue on 200s. A 422 forces the caller to notice the missing precondition; a 200 with `is_valid: false` risks silent skipping in downstream automation.
- **Fill missing eligible_count from the underlying `unit_ownership_shares` table** — rejected because it merely relocates the fabrication (aggregating unit shares to "participation" without any voter attendance data is the same class of guess). The endpoints are compliance signals; if the input is missing, we must not manufacture it.

## Root-cause trace
1. Symptom: `POST /votes/{id}/validate-slovak` returns `is_valid: true` on a vote whose `results` blob is malformed/empty.
2. ← `validate_slovak_vote` computes `approval_percentage = 80.0` via `.unwrap_or(80.0)` on the option-lookup at `regional_compliance.rs:277`.
3. ← That fallback path was introduced to keep the handler compiling before the data-completeness contract was locked. The `SlovakAccountingExport` class (PR #2030 / #2086 / #2117) picked out the equivalent lie in the export path; the vote-validation counterpart was missed.
4. Origin: unclear commit — the shape has been present since the endpoints were first added; tracked here as the first time it surfaces as a `code-review-finding`.

## Test plan
- [ ] `backend/servers/api-server/tests/regional_compliance_vote_validation_tests.rs` — new `#[sqlx::test]` file:
  - `validate_slovak_vote_rejects_empty_results` — seed a vote with `results = json!({})`; assert HTTP 422 + `insufficient_data`.
  - `validate_slovak_vote_rejects_zero_eligible_count` — same but `eligible_count = 0`; assert 422.
  - `validate_slovak_vote_rejects_missing_yes_option` — `results.questions[0].options` contains only "No"; assert 422.
  - `validate_slovak_vote_happy_path` — well-formed data crossing quorum; assert 200 + `is_valid: true` (regression against over-eager 422).
  - Mirror 4 cases for `validate_czech_vote`.
- [ ] Command: `cargo test -p api-server regional_compliance_vote_validation` (Postgres provided by CI's `backend.yml` sqlx-test lane).
- [ ] Snapshot the OpenAPI response schema after regen: `docs/api/generated/openapi.yaml` gains a 422 case for both endpoints.

## Out of scope
- Fixing the sibling fake-minutes fabrication (`get_slovak_vote_minutes` / `get_czech_usneseni`, lines 341-361 & 897-920). Different code paths, different data, tracked separately as `code-review-api-handlers-fake-minutes`.
- Reworking `get_accounting_metrics` tuple-transposition risk (`code-review-api-handlers-metric-tuple-swap`) — refactor-vector, separate item.
- Any UI / mobile change reflecting the new 422; the frontend already fails-closed on non-2xx.

## After-merge
- Move this file to `plans/_archive/bug-regional-compliance-fake-vote-validation.md`
- Mark the matching `backlog.json` row (`code-review-api-handlers-fake-vote-fallback`) as `status: "done"`
