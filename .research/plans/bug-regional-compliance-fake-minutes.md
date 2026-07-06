# bug-regional-compliance-fake-minutes

**Vector:** bug
**Score:** 3
**Source:** hotspot in `backend/servers/api-server/src/routes/regional_compliance.rs`
**Confidence:** medium

## Hypothesis
`get_slovak_vote_minutes` and `get_czech_usneseni` in `routes/regional_compliance.rs` return legally-styled compliance documents (vote-minutes, usneseni) whose vote counts, participants, share totals, meeting date, meeting location, SVJ name, and IČO are **all hardcoded literal constants** — only `vote_id`, `building_id`, and `title` come from the database. The endpoints are exposed under the `Regional Compliance` tag and served alongside real compliance calls, indistinguishable on the wire from a genuine minutes document. This is the exact class of quiet dishonesty PR #2030 / #2086 / #2117 killed for `SlovakAccountingExport`; the smallest fix is to (a) source every non-metadata field from actual voting data or refuse the request with 404/422, and (b) split the response type so a partial document is honestly labeled `partial: true, missing_fields: [...]`.

## Evidence
- `backend/servers/api-server/src/routes/regional_compliance.rs:341-361` — `get_slovak_vote_minutes` returns `SlovakVoteMinutes` with hardcoded `meeting_date = 2024-01-15`, `meeting_location = "Spoločenská miestnosť, Hlavná 1, Bratislava"`, `total_ownership_shares = 100`, `participation = 75%`, `votes_for = 60`, `votes_against = 10`, `abstentions = 5`, `result_approved = true`, empty `participants` / `questions` vectors.
- `backend/servers/api-server/src/routes/regional_compliance.rs:897-920` — same fabrication in `get_czech_usneseni` (hardcoded `svj_name = "SVJ Hlavní 1"`, `ico = "12345678"`, hardcoded meeting date + share counts + vote counts, `result_approved = true`).
- Class parallel: `SlovakAccountingExport` had the identical "looks complete but isn't" leak (missing revenue/receivables fields defaulted to zero, `partial` flag never set). Fixed in PR #2030 (test), #2069 / #2099 (`compute_partial`), #2117 (named-field constructor). This handler is where the class relocated.
- Reviewer note (`.research/signals/2026-07-06.json` → `code-review-api-handlers-fake-minutes`): rotating expert review flagged it while inspecting the churn cluster around PRs #2099 / #2117.

## Files
- `backend/servers/api-server/src/routes/regional_compliance.rs:341`
- `backend/servers/api-server/src/routes/regional_compliance.rs:897`

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
1. `POST /api/v1/regional-compliance/votes/{id}/minutes-slovak` for any existing `voting_sessions` row.
2. Inspect the JSON response: fields `meeting_date`, `meeting_location`, `total_ownership_shares`, `participation`, `votes_for`, `votes_against`, `abstentions`, `result_approved` all identical to the constants in `routes/regional_compliance.rs:341-361` regardless of the underlying data.
3. Expected (after fix): the response either sources those fields from real tables (`voting_sessions.results`, `unit_ownership_shares`, `buildings`, `board_meetings`) OR returns `{ partial: true, missing_fields: [...] }` with the un-sourceable fields absent from the JSON.

## Suggested approach
1. Add a `partial: bool` and `unsupported_fields: Vec<String>` pair to `SlovakVoteMinutes` / `CzechUsneseni` (mirror the `SlovakAccountingExport` shape already merged in `models/regional_compliance.rs`).
2. Replace each hardcoded literal in the two handlers with a real fetch:
   - `meeting_date`, `meeting_location` → `board_meetings` (if the vote was tied to a meeting) or the `voting_sessions.metadata` JSON; otherwise mark `partial + unsupported_fields.push("meeting_date")`.
   - `total_ownership_shares` → sum of `unit_ownership_shares.share_fraction` for the building (already used by the accounting export).
   - `votes_for` / `votes_against` / `abstentions` / `participation` → aggregate from `voting_sessions.results` (already parsed by `validate_slovak_vote`).
   - `svj_name` / `ico` (Czech) → `organizations.legal_name` / `organizations.registration_number` for the tenant.
3. Extract a small `derive_slovak_minutes(&vote, &building, &org) -> (SlovakVoteMinutes, Vec<&'static str>)` helper (or an `impl SlovakVoteMinutes::from_sources(...)` constructor, same shape as `SlovakAccountingExport::new` after PR #2117) so the handler never assembles the struct field-by-field.
4. Mirror the same helper for Czech.
5. If any of `voting_sessions` / `board_meetings` / `unit_ownership_shares` isn't seeded, return `partial: true` — do NOT invent a value. Never emit `result_approved: true` off a manufactured tally.
6. Update the `#[utoipa::path(...)]` schema so the `partial` / `unsupported_fields` fields are documented; regenerate the OpenAPI + `@ppt/api-client` (matches the pattern used by PR #2098 for the enum-sync guard).
7. `cargo clippy -p api-server --all-targets -- -D warnings` + the tests below.

## Alternatives considered
- **Delete the endpoints outright** — rejected because the routes are advertised in the `Regional Compliance` tag and the mobile / web clients may be relying on them; a sudden 404 breaks compatibility. The partial-honesty approach preserves the shape and lets clients adopt the truth gradually.
- **Return 501 Not Implemented until the DB schema catches up** — rejected because parts of the response *are* derivable today (SVJ name, share sum). A 501 hides that progress and blocks the mobile clients from consuming what's real. Partial-honesty is the smallest step that stops the lie without regressing what already works.

## Root-cause trace
1. Symptom: `GET /votes/{id}/minutes-slovak` returns identical `meeting_date=2024-01-15` / vote counts across every vote id.
2. ← Handler at `routes/regional_compliance.rs:341-361` builds the struct with literal constants inline.
3. ← The endpoints were introduced as stubs to keep the client shape live before the aggregation SQL was written; the "TODO: source real data" comment was never followed up on.
4. Origin: introduced when the Regional Compliance tag first shipped; tracked here as the first time it surfaces as a `code-review-finding`.

## Test plan
- [ ] `backend/servers/api-server/tests/regional_compliance_minutes_tests.rs` — new `#[sqlx::test]` file:
  - `slovak_minutes_reports_partial_when_no_meeting_data` — seed only `voting_sessions` (no `board_meetings`, no unit shares); assert response `partial: true`, `unsupported_fields` contains `meeting_date`, `total_ownership_shares`.
  - `slovak_minutes_sources_share_total_from_units` — seed `unit_ownership_shares` with fractions summing to `100`; assert `total_ownership_shares == 100`.
  - `slovak_minutes_never_manufactures_result_approved` — seed a vote with `results = null`; assert `result_approved == false` (or field absent) — pins the negation of the current lie.
  - Mirror 3 cases for `get_czech_usneseni`.
- [ ] Command: `cargo test -p api-server regional_compliance_minutes` (CI provides Postgres).
- [ ] OpenAPI regen: `docs/api/generated/openapi.yaml` gains the `partial` / `unsupported_fields` fields on both response schemas.

## Out of scope
- The vote-validation fake-fallback fix (`code-review-api-handlers-fake-vote-fallback`) — separate plan, different lines.
- The metric-tuple-swap refactor (`code-review-api-handlers-metric-tuple-swap`) — refactor vector, separate item.
- Populating historical `board_meetings` rows so old votes get non-partial responses — data-quality task, not a code fix.

## After-merge
- Move this file to `plans/_archive/bug-regional-compliance-fake-minutes.md`
- Mark the matching `backlog.json` row (`code-review-api-handlers-fake-minutes`) as `status: "done"`
