# unchecked-todo-pr-1861

**Vector:** security
**Score:** 3
**Source:** PR #1861, Issue #1906, file `backend/servers/api-server/src/routes/regional_compliance.rs`
**Confidence:** high

## Hypothesis
PR #1861 shipped 19 regional-compliance endpoints with the PR-body CI/test checklist unchecked, and the post-merge review (issue #1906) confirmed two material defects: the DB quorum-rule lookup in `validate_slovak_vote`/`validate_czech_vote` is unreachable (wrong lookup key), and four sensitive write handlers carry no role gate so any authenticated org member can rewrite jurisdiction, GDPR DPO, accounting IBAN/ICO, and Czech SVJ config. Both are tenant-isolated but under-privileged, and finding #1 silently demotes the validator to its in-code fallback — the validation note that claims "computed against seeded database jurisdiction rules" is a lie at runtime. Smallest correct fix: add a snake-case `decision_type_key()` to the decision-type enums, switch the validator lookups to it, and gate the four config writes with `RequireCapability` (or the project's equivalent admin guard).

## Evidence
- Issue #1906 finding #1 — `validate_slovak_vote`/`validate_czech_vote` pass `legal_reference()` (`"SS 14 ods. 1 zakona 182/1993 Z.z."`) as the `decision_type` lookup key, while `jurisdiction_rules.decision_type` is seeded with `'simple_majority'`/`'two_thirds_majority'` (migration 00197). `WHERE decision_type = $2` never matches → silent fallback.
- `backend/servers/api-server/src/routes/regional_compliance.rs:175` and `:709` — confirmed both call sites use `payload.decision_type.legal_reference()` against `get_quorum_rule(...)`.
- Issue #1906 finding #5 — `lib.rs ~L309` mounts the nest under `RlsConnection` only; per `.claude/skills/ppt-implement/agents/rust-backend.md` org-admin-level writes need `RequireCapability` (forgetting it = route is public).
- Issue #1906 finding #6 — `regional_compliance_tests.rs` has no coverage for `validate_slovak_vote`/`validate_czech_vote`, `export_slovak_accounting`, the minutes/usneseni endpoints, or Czech SVJ config; a known-decision-type validator test (e.g. `two_thirds_majority → 66.67`) would have caught finding #1.

## Files
- `backend/servers/api-server/src/routes/regional_compliance.rs:175`
- `backend/servers/api-server/src/routes/regional_compliance.rs:709`
- `backend/crates/db/src/models/regional_compliance.rs`
- `backend/servers/api-server/tests/regional_compliance_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
- C4/C5 untouched → `cloud-ok`

Mode: cloud-ok

## Repro steps
1. Boot the dev stack and seed regional-compliance migrations (`stack up pm-local …`).
2. As an authenticated user POST `/api/v1/regional-compliance/slovak/validate-vote` with `{"decision_type": "two_thirds_majority", ...}` payload that matches the DB-seeded 66.67 quorum.
3. Inspect the response and the `jurisdiction_rules` query log: the validator returns the in-code fallback quorum (whatever `required_quorum_percentage()` hard-codes), never the seeded DB value. Expected: DB-driven quorum. Actual: silent fallback because `$2` is the Slovak statute string, not `"two_thirds_majority"`.
4. As a regular tenant member (not org-admin) POST `/api/v1/regional-compliance/slovak/gdpr` with attacker-chosen DPO. Expected: 403. Actual: 200 — config overwritten.

## Suggested approach
1. In `backend/crates/db/src/models/regional_compliance.rs`, add `fn decision_type_key(&self) -> &'static str` to `SlovakDecisionType` and `CzechDecisionType` returning the snake-case keys that match migration 00197 (`simple_majority`, `two_thirds_majority`, `three_quarters_majority`, etc.).
2. In `backend/servers/api-server/src/routes/regional_compliance.rs`, swap both `get_quorum_rule(..., payload.decision_type.legal_reference())` call sites (line 175, line 709) to `payload.decision_type.decision_type_key()`.
3. Add `RequireCapability` (or the existing `OrgAdmin`/manager guard used by sibling admin writes — match what `platform_admin.rs` uses) to `set_jurisdiction`, `configure_slovak_gdpr`, `configure_slovak_accounting`, and `configure_czech_svj` handler signatures.
4. In `backend/servers/api-server/tests/regional_compliance_tests.rs`, add: (a) a validator integration test that asserts a seeded `two_thirds_majority` → 66.67 quorum is returned from DB (red on `main`, green after step 2); (b) a 403 test that posts to each of the four config writes as a non-admin member; (c) an export-accounting test that the response `organization_id` echoes the authenticated tenant (covers the implicit-context tightening in #1906 finding #4 — leave that drop-`organization_id`-from-body change as a follow-up if it widens scope).
5. Run `cargo test -p api-server regional_compliance` then `cargo clippy -p api-server -- -D warnings`.

## Alternatives considered
- **Edit migration 00197 in place to seed the legal-reference strings as the lookup keys** — rejected because the seeded `'simple_majority'`/`'two_thirds_majority'` rows are already exercised correctly by `get_slovak_vote_minutes`/`get_czech_usneseni`, and the legal-reference strings are localised statute citations that don't belong as join keys. Aligning the code-side key is the smaller, safer fix.
- **Skip the role-gate work and ship only the quorum fix** — rejected because the post-merge review explicitly tied #1 and #5 together as the two highest-severity items, and shipping the quorum fix alone leaves the `set_jurisdiction`/GDPR/accounting writes public. The four-handler guard is one decorator each.

## Root-cause trace
1. Symptom: `validate_slovak_vote` response always quotes the in-code fallback quorum even when the seeded DB row would have returned a different value.
2. ← `get_quorum_rule(&mut **rls.conn(), Jurisdiction::Slovakia, payload.decision_type.legal_reference())` at `regional_compliance.rs:175` (and `:709` for the Czech variant) — the third argument is the *statute citation* string, not the lookup key the DB seeded.
3. ← `SlovakDecisionType::legal_reference()` and `CzechDecisionType::legal_reference()` in `models/regional_compliance.rs` were re-used as the lookup key during the PR #1861 implementation; no `decision_type_key()` equivalent exists for the validator code path. (The minutes/usneseni handlers correctly pass `"simple_majority"` literal, so the DB rows themselves are right.)
4. Origin: PR #1861 (merged 2026-06-27) — Epic 72 implementation that replaced the stubbed handlers. The unchecked CI/test checklist in the PR body is the surfaced symptom; the missing key abstraction is the underlying cause.

## Test plan
- [ ] `backend/servers/api-server/tests/regional_compliance_tests.rs::test_validate_slovak_vote_uses_seeded_quorum` — seed `two_thirds_majority` row, POST validate, assert response `required_quorum_percentage == 66.67` (red on `main`).
- [ ] `backend/servers/api-server/tests/regional_compliance_tests.rs::test_validate_czech_vote_uses_seeded_quorum` — same shape for the Czech variant.
- [ ] `backend/servers/api-server/tests/regional_compliance_tests.rs::test_compliance_writes_require_admin_role` — 4 sub-cases, one per protected handler, asserts 403 from a non-admin tenant member.
- [ ] `cargo test -p api-server regional_compliance` (full module).
- [ ] `cargo clippy -p api-server -- -D warnings`.

## Out of scope
- Issue #1906 finding #2 (Czech `three_quarters_majority` legal-reference string mismatch between code and migration 00197) — a forward-fix migration; coordinate with db-migration owner separately.
- Issue #1906 finding #3 (`total_expenses`/`total_payables` hardcoded zero in export) — separate accounting-correctness task.
- Issue #1906 finding #4 (drop `organization_id` from the `ExportSlovakAccounting` body) — API contract change; gather call sites first.
- Issue #1906 finding #7 (`ch_north` data_region naming) — cosmetic; backlog row, not this plan.

## After-merge
- Move this file to `plans/_archive/unchecked-todo-pr-1861.md`
- Mark the matching `backlog.json` row as `status: "done"`
