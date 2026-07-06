# code-review-api-handlers-compliance-minutes-stubbed

**Vector:** bug
**Score:** 3
**Source:** api-handlers rotating review 2026-07-06 (hotspot in `backend/servers/api-server/src/routes/regional_compliance.rs`)
**Confidence:** high

## Hypothesis

`get_slovak_vote_minutes` and `get_czech_usneseni` return the legally-required
minutes (Slovak `zápisnica` / Czech `usnesení`) for a completed building vote,
but both handlers build the response from **hardcoded literals** instead of the
actual vote result. `meeting_date`, `meeting_location`, `total_ownership_shares`,
`participating_shares`, `votes_for`, `votes_against`, `abstentions`, and
`result_approved` all ship the same static values for every building and every
vote — only `vote_id`, `building_id`, and `title` are read from the DB. Any
manager who downloads the minutes gets a document that says "meeting held
2024-01-15 in Bratislava, 60/10/5 for/against/abstain, approved". These are the
routes at the top of Epic 72 (regional legal compliance), reachable from
authenticated production callers today.

## Evidence

- `backend/servers/api-server/src/routes/regional_compliance.rs:344` — `meeting_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()` in `get_slovak_vote_minutes` (Slovak minutes handler).
- `backend/servers/api-server/src/routes/regional_compliance.rs:345,349-357` — `meeting_location: "Spolocenska miestnost, Hlavna 1, Bratislava"`, `total_ownership_shares: Decimal::new(10000, 2)`, `participating_shares: 7500`, `votes_for: 6000`, `votes_against: 1000`, `abstentions: 500`, `result_approved: true`. Response shape produced only from these literals plus DB `vote_id/building_id/title`.
- `backend/servers/api-server/src/routes/regional_compliance.rs:902` — same-shape stub in `get_czech_usneseni` (Czech SVJ minutes handler).
- `backend/servers/api-server/src/routes/regional_compliance.rs:77,97` — both endpoints are mounted at `/api/v1/regional-compliance/slovak/voting/minutes/{vote_id}` and `/api/v1/regional-compliance/czech/svj/usneseni/{vote_id}`; reachable via `RlsConnection` (authenticated + tenant-scoped, but not manager-gated in read).
- `backend/crates/db/src/models/vote.rs` (contains `VoteResults`, `VoteQuestion`, `VoteOptionResult` — the real result shape already used by `validate_slovak_vote` at line 262).

## Files

- `backend/servers/api-server/src/routes/regional_compliance.rs:344`
- `backend/servers/api-server/src/routes/regional_compliance.rs:902`
- `backend/crates/db/src/models/regional_compliance.rs`
- `backend/crates/db/src/models/vote.rs`

_(New test file `backend/servers/api-server/tests/regional_compliance_minutes_tests.rs` is created by the implementer — not listed above so G7 file-existence check passes on plan-creation.)_

## Dependencies

_(none)_

## Required capabilities

- [x] C1 — Systematic debugging (bug vector — trace how the stubs were shipped past review)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (compliance surface — expect scrutiny)

Mode: cloud-ok

## Repro steps

1. Bring up `stack up pm-local` (or drive the remote bridge MCP).
2. Log in as an org manager and create a Slovak-jurisdiction vote; record the actual `participation_count`, `eligible_count`, and cast a distinct pattern of votes (e.g. 3 yes / 1 no / 1 abstain) so the results are recognizable.
3. `GET /api/v1/regional-compliance/slovak/voting/minutes/{vote_id}` with the manager's bearer token + `X-Tenant-ID`.
4. Expected: response body reflects the actual meeting date the vote closed at, the actual `participation_count`/`eligible_count`, and the actual per-question yes/no/abstain tallies.
5. Actual (today): `meeting_date == 2024-01-15`, `meeting_location == "Spolocenska miestnost, Hlavna 1, Bratislava"`, `participation_percentage == 75.00`, `votes_for == 60.00`, `votes_against == 10.00`, `abstentions == 5.00`, `result_approved == true` — **for every vote, every building, every org**. Repeat with a Czech-jurisdiction vote and `/api/v1/regional-compliance/czech/svj/usneseni/{vote_id}` — same class of stub at `regional_compliance.rs:902`.

## Suggested approach

1. Land a failing-on-main integration test first (`backend/servers/api-server/tests/regional_compliance_minutes_tests.rs`, `#[sqlx::test(migrator = "db::MIGRATOR")]`): seed org+building+vote with a distinctive `participation_count`/`eligible_count` and a known question result, POST it, then `GET .../minutes/{vote_id}` and assert `participation_percentage` and `votes_for` are computed from the seeded vote — **not** 7500/6000 constants. Assert on the value, not a source grep. Mirror it for `/czech/svj/usneseni/{vote_id}`. (IG3.)
2. Compute `meeting_date` from the source-of-truth already on the vote row — the closing timestamp of the poll (or the vote's `created_at`/`closed_at` — pick the field that means "when the meeting concluded"). Add a `meeting_location: Option<String>` to `Vote` / `Poll` if it's not already there (Epic 72 spec has a "meeting_location" concept — check the migration and add it if missing; if it is, plumb the DB field into the response).
3. Reuse the `VoteResults` parsing that already works in `validate_slovak_vote` (`regional_compliance.rs:262`): pull `serde_json::from_value::<VoteResults>(vote.results.clone())`, walk `q.results`, sum `yes`/`no`/`abstain` percentages into `votes_for`/`votes_against`/`abstentions`. Reject the request with `422 UNPROCESSABLE_ENTITY` (not silently return `80.0`) if the JSON is missing or unparseable — a minutes document with fabricated tallies is the failure mode.
4. Derive `total_ownership_shares` and `participating_shares` from `eligible_count` × the building's `ownership_share_total`. If the numbers aren't in the DB (they probably are — check `regional_compliance_repo`), extend the repo, don't invent 10000/7500.
5. Populate `participants` and `questions` from actual DB rows (`vote_participants`, `vote_questions` — Slovak jurisdiction requires per-owner minutes signoff; empty vectors are a legal defect, not a stub).
6. Add a manager gate to the read: minutes disclose per-owner voting positions, which is intra-org PII. Mirror `require_manager(...)` at `regional_compliance.rs:31` on the two GET endpoints; keep RLS on top.
7. Apply the same shape to `get_czech_usneseni` — different quorum rule (`SS 1206 zakona 89/2012 Sb.`) but the same "stop lying about the meeting" fix.

## Alternatives considered

- **Ship a `501 NOT_IMPLEMENTED` on both endpoints** — rejected because the API is already advertised in the OpenAPI spec and the frontend is likely wired up; a hard 501 is a worse UX than "compute from the vote we actually have". A returning-stubs endpoint is *worse* than a 501 (silent legal risk), but 501 is only correct if we're rolling the feature back entirely.
- **Only fix the tallies; keep the stubbed meeting date/location** — rejected because the meeting date on a Slovak minutes document is a legal fact (`SS 14 ods. 1 zakona 182/1993 Z.z.`); wrong date invalidates the whole document. Fixing only the numbers still ships a document dated 2024-01-15 with 2026 tallies.

## Root-cause trace

1. Symptom: `GET .../slovak/voting/minutes/{vote_id}` returns identical `meeting_date`/`meeting_location`/`participation_percentage`/`votes_for`/`votes_against`/`abstentions`/`result_approved` for every vote_id.
2. ← handler `get_slovak_vote_minutes` at `backend/servers/api-server/src/routes/regional_compliance.rs:304-364` — builds `SlovakVoteMinutes { … }` from static literals, wiring only `vote_id`/`building_id`/`title` from the loaded `vote`.
3. ← the sibling handler `validate_slovak_vote` (same file, lines 220-301) shows the correct pattern: it parses `vote.results` as `VoteResults` and derives the percentages — so the DB has the data, the handler just doesn't use it.
4. ← the same-shape stub in `get_czech_usneseni` at `regional_compliance.rs:902` confirms this is a copy-paste class defect, not a one-off.
5. Origin: Epic 72 initial rollout (regional_compliance.rs handlers shipped as skeletons with the intent to fill in later; the intent-marker got lost — no `TODO` in the current source, so grep-based catch nets missed it. Search for the introducing commit via `git log -L 304,364:backend/servers/api-server/src/routes/regional_compliance.rs -- backend/servers/api-server/src/routes/regional_compliance.rs`).

## Test plan

- [ ] `backend/servers/api-server/tests/regional_compliance_minutes_tests.rs::slovak_minutes_reflect_seeded_vote` — seed distinct participation/eligible/tallies, `GET .../slovak/voting/minutes/{vote_id}`, assert body reflects the seeded numbers (fails on today's tree).
- [ ] `backend/servers/api-server/tests/regional_compliance_minutes_tests.rs::czech_usneseni_reflect_seeded_vote` — mirror for Czech.
- [ ] Negative: seed a vote with `vote.results` as `Value::Null`; the endpoint must return `422 UNPROCESSABLE_ENTITY` (not `200 OK` with fabricated tallies).
- [ ] Authz: seed a non-manager org member; `GET .../minutes/{vote_id}` returns `403 FORBIDDEN` (regression pins the new manager gate).
- [ ] Command: `cd backend && cargo test -p api-server --test regional_compliance_minutes_tests` (locally against `DATABASE_URL`; CI in `backend.yml`).

## Out of scope

- Adding the `meeting_location` DB column if it doesn't exist yet — that's a migration + spec question; if the column is missing, this plan just carries `None` through and files a follow-up for the migration. **Do NOT** silently invent the location from other fields.
- The `configure_slovak_voting` / `configure_czech_svj` write paths (they already work and are manager-gated).
- Sidecar concerns from the same rotating review — `gdpr-fake-dpo` (separate plan `code-review-api-handlers-gdpr-fake-dpo.md`) and `vote-validator-fake-defaults` (separate backlog row, score 2, not promoted this run).

## After-merge

- Move this file to `plans/_archive/code-review-api-handlers-compliance-minutes-stubbed.md`.
- Mark the matching `backlog.json` row as `status: "done"` and add the merged PR # to `sources`.
