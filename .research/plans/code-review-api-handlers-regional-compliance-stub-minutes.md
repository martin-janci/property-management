# code-review-api-handlers-regional-compliance-stub-minutes

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 code review of `api-handlers` segment (2026-06-28)
**Confidence:** high

## Hypothesis
`get_slovak_vote_minutes` and `get_czech_usneseni` in `regional_compliance.rs` look up a real `vote_id` from the DB and then return a response struct that mixes the real `vote.title` and `vote.building_id` with hardcoded fixtures (`meeting_date: 2024-01-15`, `meeting_location: "Spolocenska miestnost, Hlavna 1, Bratislava"`, `svj_name: "SVJ Hlavni 1"`, `ico: "12345678"`, `total_ownership_shares 10000`, `participating_shares 7500`, `votes_for 6000`, `votes_against 1000`, `abstentions 500`, `participants: vec![]`, `questions: vec![]`). The handlers are mounted on the live router and the OpenAPI tag advertises them as legal vote minutes / SVJ usneseni. Property managers using the Slovak/Czech tenancy will receive an authoritative-looking compliance document whose meeting date, location, vote tallies, and participant list are fabricated. The fix is to either compute the response from real vote data (participants, tallies, building meeting metadata) or gate the endpoint behind a `cfg(feature = "stub-compliance-minutes")` flag and return `501 Not Implemented` on the live router until the data pipeline is wired.

## Evidence
- `backend/servers/api-server/src/routes/regional_compliance.rs:290-309` — `SlovakVoteMinutes` returned with `meeting_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()`, `meeting_location: "Spolocenska miestnost, Hlavna 1, Bratislava"`, fixed `participating_shares Decimal::new(7500, 2)`, `votes_for Decimal::new(6000, 2)`, `votes_against Decimal::new(1000, 2)`, `abstentions Decimal::new(500, 2)`, `participants: vec![]`, `questions: vec![]`.
- `backend/servers/api-server/src/routes/regional_compliance.rs:830-852` — same shape for `CzechSvjUsneseni`, additionally with `svj_name: "SVJ Hlavni 1"`, `ico: "12345678"`, `requires_notary: false` hardcoded.
- Router is wired live — Phase 1.5 confirmed via static analysis that `/api/v1/regional-compliance/slovak/voting/minutes/{vote_id}` and `/.../czech/svj/usneseni/{vote_id}` are mounted.
- Real DB data is fetched: `vote.building_id` and `vote.title` are read from the live vote and mixed with the fixtures, which strengthens the impression of authenticity.
- Commit 684d7f68c (PR #1861) added 19 regional-compliance endpoints; the stubs landed as part of the same Epic 72 push, so this is a recent introduction rather than legacy dead code.

## Files
- `backend/servers/api-server/src/routes/regional_compliance.rs:290`
- `backend/servers/api-server/src/routes/regional_compliance.rs:830`
- `backend/servers/api-server/tests/regional_compliance_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode:** `Mode: cloud-ok` (no UI, no device — backend unit + integration tests only).

## Repro steps
1. Seed a vote with `vote_id = X`, building `B`, title `"Annual fund-allocation vote"`, **5 participants**, **70%** participating shares, **65 / 30 / 5** for/against/abstain split.
2. Call `GET /api/v1/regional-compliance/slovak/voting/minutes/{X}` as the building's manager.
3. Expected: response carries the real meeting date, location (drawn from building metadata), `participating_shares` ≈ 70%, `votes_for` ≈ 65%, `participants` = list of 5 owner refs.
4. Actual: response carries `meeting_date: 2024-01-15`, `meeting_location: "Spolocenska miestnost, Hlavna 1, Bratislava"`, `participating_shares 75.00`, `votes_for 60.00`, `participants: []`.

## Suggested approach
1. **Decide the contract.** Two safe options — (a) wire the response to real data; (b) return `501 Not Implemented` on the live router with a feature flag. Recommend (a) for both endpoints — Epic 72 advertises these as legal documents and 501 would surprise consumers.
2. **Identify the missing data source.** Vote participants, tallies, and meeting metadata. Cross-reference `votes` + `vote_participants` + `buildings.meeting_*` columns. If those tables don't carry meeting date/location, the migration is part of this plan's scope (add `meeting_date`, `meeting_location` to `building_meetings` or similar).
3. **Replace the fixtures** in `regional_compliance.rs:290-309` with computed values from the participant/tally repos. Same for the Czech handler at `:830-852`.
4. **Carry forward jurisdiction-specific fields** — `svj_name` and `ico` for Czech come from the building's parent SVJ entity; if there is no `svj_name`/`ico` column today, add it (Story 72.x scope) or refuse to mint a usneseni without one.
5. **Add an integration test** in `backend/servers/api-server/tests/regional_compliance_tests.rs` that runs the repro above and asserts on the real values.
6. **Tag the OpenAPI spec** — confirm the description for these endpoints does NOT promise legal authority. If it does, tone it down or wire (a).

## Alternatives considered
- **501 Not Implemented + feature flag** — rejected because Epic 72 already advertises these endpoints as shipped and ppt-web/reality-web may have begun consuming them; flipping to 501 silently is a regression risk.
- **Compute only the tallies, leave meeting_location hardcoded** — rejected because the hardcoded location is locale-specific (Bratislava vs Praha) and would mislead managers in any other city.

## Root-cause trace
1. Symptom: live API returns fabricated values for `meeting_date`, `meeting_location`, vote tallies, and participants.
2. ← `regional_compliance.rs:290-309` — `SlovakVoteMinutes` struct literal hardcodes those fields instead of reading from DB.
3. ← The handler does call `state.regional_compliance_repo.get_jurisdiction(...)` and an `unwrap_or(...)` rule lookup, but the struct literal then ignores everything except `vote.building_id` + `vote.title`.
4. Origin: PR #1861 (`fix(api-server): implement 19 regional-compliance endpoints (Epic 72)`, 2026-06-27) — the endpoints landed as stubs and were never wired to real participant/tally data.

## Test plan
- [ ] `backend/servers/api-server/tests/regional_compliance_tests.rs::slovak_voting_minutes_renders_real_tallies` — seed vote with 5 participants and the 70/65/30/5 split above; assert returned `participating_shares`, `votes_for`, `votes_against`, `abstentions`, and `participants.len()` match.
- [ ] `…::czech_svj_usneseni_renders_real_svj_metadata` — seed an SVJ building with `svj_name = "SVJ Vinohrady"`, `ico = "98765432"`; assert response carries those, NOT the hardcoded `"SVJ Hlavni 1"` / `"12345678"`.
- [ ] `…::vote_minutes_requires_manager_role` — sanity check that authz is unchanged.
- [ ] Local: `cargo test -p api-server regional_compliance_tests`.

## Out of scope
- Refactor of `regional_compliance.rs` into sub-modules (per existing `repeated-churn-...regional-compliance` consideration). Land the bugfix first, defer the split.
- AML/DSA endpoints in the sibling `aml_dsa/` directory — those use `require_compliance_role` and return real data per Phase 1.5 confirmation.
- Other Epic 72 endpoints not on the `/minutes/` + `/usneseni/` paths.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-regional-compliance-stub-minutes.md`
- Mark the matching `backlog.json` row as `status: "done"`
