# code-review-regional-compliance-hardcoded-vote-minutes

**Vector:** bug
**Score:** 3
**Source:** dispatcher tier1d signal `code-review-regional-compliance-hardcoded-vote-minutes` (2026-08-01 api-handlers review) + code confirmation at `backend/servers/api-server/src/routes/regional_compliance.rs:341-361` and `:893-916`
**Confidence:** high

## Hypothesis
Two production handlers that generate legally-material condominium voting-minutes documents — the Slovak `zapisnica` (§14 z. 182/1993 Z.z.) and the Czech SVJ `usneseni` (§1206 z. 89/2012 Sb.) — return the vote's `title` and `building_id` from the actual row but populate every other legally-required field (meeting date/location, ownership shares, participation, votes for/against/abstentions, `quorum_met`, `result_approved`) with hardcoded module-level placeholder constants. Any external audit or regulator that pulls these minutes gets a fabricated record that always shows 75% turnout, 60/10/5 vote split, quorum met, and result approved — regardless of what the vote actually recorded. The smallest correct change is to plumb the real vote's aggregated result and the tenant's SVJ/building metadata through to the response builders instead of the `Decimal::new(…)` / `NaiveDate::from_ymd_opt(…)` literals.

## Evidence
- `backend/servers/api-server/src/routes/regional_compliance.rs:341-361` — `get_slovak_vote_minutes` returns `SlovakVoteMinutes { meeting_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(), meeting_location: "Spolocenska miestnost, Hlavna 1, Bratislava".to_string(), total_ownership_shares: Decimal::new(10000, 2), participating_shares: Decimal::new(7500, 2), participation_percentage: Decimal::new(7500, 2), quorum_met: true, votes_for: Decimal::new(6000, 2), votes_against: Decimal::new(1000, 2), abstentions: Decimal::new(500, 2), result_approved: true, participants: vec![], questions: vec![] }` — every numeric field is a hardcoded literal, `participants`/`questions` are empty.
- `backend/servers/api-server/src/routes/regional_compliance.rs:893-916` — `get_czech_usneseni` mirrors the same pattern: `svj_name: "SVJ Hlavni 1"`, `ico: "12345678"`, `meeting_date: NaiveDate::from_ymd_opt(2024, 1, 15)`, and the identical `Decimal::new(...)` participation/vote/quorum literals.
- Both handlers already resolve the real `Vote` row via the repository above the offending block, so the shape of the "correct" version is: pull `votes_for`/`votes_against`/`abstentions`/`participating_shares` from the vote's tally, `meeting_date`/`meeting_location` from the meeting record, `total_ownership_shares` from the building's ownership snapshot, and derive `quorum_met` / `result_approved` from those + the `rule` already looked up on the line above (`SS 14 ods. 1 zakona 182/1993 Z.z.` for SK, `SS 1206 zakona 89/2012 Sb.` for CZ).
- Wired as `GET /api/v1/regional-compliance/slovak/voting/{vote_id}/minutes` and `GET /api/v1/regional-compliance/czech/svj/vote/{vote_id}/usneseni` (see the `#[utoipa::path(get, path = …)]` macros immediately above each handler), so any tenant with a `require_manager`-passing role can render the fabricated document today.
- Signal file: `.research/signals/2026-08-01-api-handlers-tier1d.json` (dispatcher rotating-expert review, confidence=high, `score_delta=3`).

## Files
- `backend/servers/api-server/src/routes/regional_compliance.rs:341`
- `backend/servers/api-server/src/routes/regional_compliance.rs:893`
- `backend/servers/api-server/tests/suites/regional_compliance_tests.rs`

## Dependencies
- (none — `regional_compliance.rs` handlers are self-contained; the repo `vote` and building lookups they need already exist above the offending blocks)

## Required capabilities
- [x] C1 — Systematic debugging (bug, high confidence — trace vote data flow from repo → handler)
- [x] C2 — Seed data (need a real vote row with non-default tallies + building/meeting metadata to assert non-fabricated output)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode:** `Mode: cloud-ok`

## Repro steps
1. Seed a vote with distinctive tallies: `votes_for=42, votes_against=17, abstentions=3`, `participating_shares=62.00`, `total_ownership_shares=100.00`, `meeting_date=2026-03-14`, `meeting_location="Test Room A"`, building `SVJ TestName / ICO 99999999`.
2. `GET /api/v1/regional-compliance/slovak/voting/{seeded_vote_id}/minutes` as a manager of the seeded tenant.
3. Expected: the response echoes the seeded values.
4. Actual (today): response contains `votes_for: 60.00, votes_against: 10.00, abstentions: 5.00, participating_shares: 75.00, meeting_date: 2024-01-15, meeting_location: "Spolocenska miestnost, Hlavna 1, Bratislava"` — every field but `vote_id`, `building_id`, `title` is fabricated. Same asymmetry for the Czech `usneseni` endpoint.

## Suggested approach
1. In `regional_compliance.rs`, extend the `Vote` (or equivalent) repository read above line 341 / 893 to also fetch the vote's tally and meeting metadata (a single joined SELECT is preferable to N+1 lookups). If the schema doesn't yet carry `meeting_date` / `meeting_location`, add a follow-up column-add migration and gate this plan on it — but check the schema first: `docs/db-schema.md` + `backend/crates/db/migrations/*` likely already have `vote_meetings` / `vote_tallies` tables from Epic 4 (Voting).
2. Replace each `Decimal::new(...)` / `NaiveDate::from_ymd_opt(...)` / `"..."` literal in the `SlovakVoteMinutes { ... }` / `CzechSvjUsneseni { ... }` builders (lines 341-361, 893-916) with the corresponding fetched value.
3. Derive `quorum_met` from `participation_percentage >= rule.0` (the quorum threshold already looked up on the line immediately above the builder) instead of hardcoding `true`.
4. Derive `result_approved` from the same decision rule (`SimpleMajority` = `votes_for > votes_against`; other `decision_type`s already exist in the enum — cover them all rather than defaulting to `true`).
5. For `participants` / `questions` (currently `vec![]`): plumb the vote-participant list and the vote's questions/agenda items through the same repo read. If those tables don't exist yet, leave `vec![]` and add an evidence line "participants/questions plumbing is out-of-scope; tracked as follow-up".
6. Czech-side: add `svj_name` and `ico` to the building's compliance profile read (a `czech_svj_profile` table likely lives next to `slovak_ownership_snapshot`); replace the `"SVJ Hlavni 1"` / `"12345678"` literals.
7. Do NOT re-implement rule lookup — the `rule` tuple returned by `regional_compliance_repo.get_slovak_quorum_rule(...)` / `get_czech_quorum_rule(...)` above the builders is already correct; only the response population is broken.

## Alternatives considered
- **Delete the two handlers entirely and 501 the routes** — rejected because they're wired into the frontend's compliance dashboard (per `.research/management/coverage.json` epic 15 status = `in-progress`), and removing them regresses shipped UX for a build-status=shipped feature; better to fix the data population than pull the plug.
- **Return `501 Not Implemented` with a clear "endpoint under construction" body until the schema plumbing is done** — rejected because the endpoints are already advertised as producing a legally-material document, and returning a 501 silently swaps a fabricated-but-valid PDF-in-JSON for a broken client contract; a caller-visible break is arguably worse than a fixed data population that the frontend can render immediately.

## Root-cause trace
1. Symptom: `/api/v1/regional-compliance/slovak/voting/{vote_id}/minutes` returns identical fabricated participation/vote numbers for every seeded vote (see Repro step 4).
2. ← Immediate cause at `backend/servers/api-server/src/routes/regional_compliance.rs:344-361` — the `SlovakVoteMinutes { ... }` builder uses only `vote.title` and `vote.building_id` from the fetched row; every numeric field is a literal.
3. ← Upstream cause at `backend/servers/api-server/src/routes/regional_compliance.rs:315-330` — the surrounding handler only reads `vote_id → Vote` (title + building_id) plus the quorum rule (`rule.0`, `rule.1`); it never joins the tally / meeting-metadata rows that would populate the rest of the struct.
4. Origin: the handler shipped as scaffold-only when the endpoint was first wired for Epic 15 (regional compliance) — a `// TODO: replace placeholders with real vote data` comment likely accompanied it, and the tier1d review just caught that the follow-up never landed. `git log --diff-filter=A -- backend/servers/api-server/src/routes/regional_compliance.rs` names the introducing commit; the reviewer/QA process should have caught the literals in the first PR body but the endpoint was categorised as "adds regional-compliance shell endpoint" and merged.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/regional_compliance_tests.rs` — new integration test `slovak_vote_minutes_reflects_seeded_tally` seeds a vote with distinctive tallies + meeting metadata, calls the endpoint, and asserts each non-fabricated numeric field matches the seed. Second test `czech_usneseni_reflects_seeded_tally` mirrors it for the Czech endpoint.
- [ ] Both tests must fail on `dev` today (fabricated literals) and pass after the fix.
- [ ] Local run: `cd backend && cargo test -p api-server --test suite_regional_compliance_tests slovak_vote_minutes_reflects_seeded_tally czech_usneseni_reflects_seeded_tally` (or the appropriate `suite_N` binary once the file is placed).
- [ ] Add a smaller unit test for `derive_quorum_met(participation_percentage, quorum_threshold)` and `derive_result_approved(votes_for, votes_against, decision_type)` as pure functions extracted from the handler — they're the small pieces that most benefit from unit coverage and are trivially independent of the DB.

## Out of scope
- Participants / questions plumbing (record it as a follow-up finding — the schema needs verification).
- New CZ `czech_svj_profile` schema fields if none exist today. Add a plumbing evidence bullet instead of introducing schema changes in this plan.
- PDF/DOCX rendering of the minutes (that's a downstream concern; the JSON shape returned here is what the frontend renders).
- Cross-tenant IDOR audit for the two endpoints — the `require_manager(&state, org_id, ...)` gate already exists and is unrelated to the fabricated-data bug.

## After-merge
- Move this file to `plans/_archive/code-review-regional-compliance-hardcoded-vote-minutes.md`
- Mark the matching `backlog.json` row as `status: "done"`
