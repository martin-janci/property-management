# code-review-api-handlers-voting-cast-no-unit-ownership

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review (dispatcher Tier-1d 2026-09-03 api-handlers)
**Confidence:** high

## Hypothesis
`cast_vote` (POST `/api/v1/voting/{id}/cast`) accepts a caller-supplied `req.unit_id` and only gates it on `existing.is_active()`. Ownership is re-validated only when `delegation_id` is `Some`. On the self path (`delegation_id: None`) the handler builds `CastVote { user_id: rls.user_id(), unit_id: req.unit_id, .. }` with no check that the caller owns/occupies `req.unit_id`. The repository INSERT relies solely on tenant-level RLS on `vote_responses`, so any authenticated member of the tenant can cast a ballot for a unit they do not own — and, via `ON CONFLICT (vote_id, unit_id) DO UPDATE`, silently overwrite another owner's already-cast ballot. Because `vote_weight` is derived from the target unit's `ownership_share`, the attacker's write also inherits that unit's weight, changing the weighted outcome of a legally-significant owner vote. Fix: add the same `get_owners_rls` + `is_owner` gate `delegations.rs::create_delegation` already applies (returns 403 "You must be an owner of the unit …") to the self path of `cast_vote` before constructing `CastVote`.

## Evidence
- `backend/servers/api-server/src/routes/voting.rs:1067-1180` — `cast_vote` self path builds `CastVote { unit_id: req.unit_id, .. }` with no owner check (`voting.rs:1155-1161`); delegation path already re-validates via the delegations `EXISTS` query at `:1107-1128`.
- `backend/crates/db/src/repositories/vote.rs:224-278` — `cast_vote_rls` INSERTs verbatim, sets `vote_weight = COALESCE((SELECT ownership_share FROM units WHERE id = $3), 1.0)` (attacker inherits target unit's weight), and `ON CONFLICT (vote_id, unit_id) DO UPDATE SET user_id = EXCLUDED.user_id, answers = EXCLUDED.answers, ...` (silent overwrite).
- `backend/servers/api-server/src/routes/delegations.rs:207-233` — `create_delegation` already performs the exact missing gate: `state.<unit_repo>.get_owners_rls(&mut **rls.conn(), unit_id)` → `owners.iter().any(|o| o.user_id == auth.user_id)` → 403 "You must be an owner of the unit …". Same pattern belongs in `cast_vote`.
- Full call path was traced statically from the mounted route (`voting.rs:47`) through the handler (`voting.rs:1067`) into the repository INSERT (`vote.rs:224`).
- Reachability: any authenticated tenant member; blast radius: silent tampering with weighted vote results on a Slovak-building-governance surface.

## Files
- `backend/servers/api-server/src/routes/voting.rs`
- `backend/crates/db/src/repositories/vote.rs`
- `backend/servers/api-server/src/routes/delegations.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug/security fix — trace the missing gate)
- [x] C2 — Seed data (two-user + two-unit RLS smoke fixture)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security-critical path — expect reviewer scrutiny)

**Execution mode (auto-derived from the ticks):**
- No C4 / C5 → `cloud-ok`

Mode: cloud-ok

## Repro steps
1. Seed two owners A and B and two units U1, U2 such that A owns U1 only and B owns U2 only, both under the same tenant/vote (`vote_responses` uses the RLS pool).
2. Authenticate as A and POST `/api/v1/voting/{vote_id}/cast` with `unit_id = U2` and any answer body — no `delegation_id`.
3. Expected: HTTP 403 with error text ≈ "You must be an owner of the unit to cast for it"; no row inserted into `vote_responses` for `(vote_id, U2)`; B's existing ballot (if any) is unchanged.
4. Actual on main: HTTP 200; `vote_responses` row `(vote_id, U2, user_id=A)` is created (or overwrites B's) with `vote_weight = ownership_share(U2)`.

## Suggested approach
1. In `voting.rs::cast_vote`, after the `existing.is_active()` check and before constructing `CastVote`, when `req.delegation_id.is_none()` load the owners of `req.unit_id` using the same repository call the delegations route already uses on the RLS connection: `state.<unit_repo>.get_owners_rls(&mut **rls.conn(), req.unit_id)`.
2. If `owners.iter().any(|o| o.user_id == rls.user_id())` is false, return `(StatusCode::FORBIDDEN, "You must be an owner of the unit to cast a vote for it".to_string())`.
3. Keep the delegation-path gate at `:1107-1128` untouched — it already checks the delegate is an owner via the `EXISTS` query.
4. Do not weaken the `ON CONFLICT` clause — the intent of allowing a re-cast on the same `(vote_id, unit_id)` remains, but the owner gate above now prevents cross-owner overwrite.
5. Consider a matching audit-log write (existing `voting.rs` audit sites) noting the ownership-rejected attempt if that pattern is already in the file; skip if the file has no audit hook.
6. Add integration test `backend/servers/api-server/tests/suites/voting_cross_owner_cast_tests.rs` covering the self-path 403 and a delegation-path negative for parity.
7. Run `cargo test -p api-server voting_cross_owner_cast` and `cargo test -p api-server voting` (existing suite) to prove no regression on the delegation path.

## Alternatives considered
- **Database CHECK / trigger on `vote_responses`** — rejected because the current schema stores `user_id` on the vote row rather than a foreign key derivable to unit-ownership; enforcing the invariant in Postgres would need a per-insert lookup against `units`/`memberships` and would still allow the app to serve a misleading 200 before rollback. The route-level gate mirrors the established delegation pattern and keeps the error UX consistent.
- **Move the check into `cast_vote_rls`** — rejected because the repository layer already runs under RLS-scoped connections and the ownership check is an application-layer authorization decision (mirrors `delegations.rs`). Placing it in the repository would double-execute for the delegation path (which already validated ownership) and would obscure the 403.

## Root-cause trace
1. Symptom: `POST /api/v1/voting/{id}/cast` returns 200 for user A with a body naming `unit_id = U2` owned by B; `vote_responses` gains/overwrites a row for `(vote_id, U2)` with `user_id = A` and `vote_weight = ownership_share(U2)`.
2. ← `voting.rs:1155-1161` (`cast_vote` self path) constructs `CastVote { user_id: rls.user_id(), unit_id: req.unit_id, .. }` with no ownership predicate.
3. ← `voting.rs:1107-1128` gate applies only when `req.delegation_id.is_some()`, so the self path skips owner validation entirely.
4. ← `vote.rs:224-278` (`cast_vote_rls`) issues the INSERT under tenant-scoped RLS (`vote_responses` policy); tenant RLS is not unit-owner-scoped, so the write succeeds.
5. Origin: introduced when the delegation branch's owner check was added to `cast_vote` without a symmetric branch for `delegation_id.is_none()`. `delegations.rs:207-233` shows the intended gate; `voting.rs` diverged.

## Test plan
- [ ] Integration test `voting_cross_owner_cast_self_path_returns_403` in `backend/servers/api-server/tests/suites/voting_cross_owner_cast_tests.rs`: user A owning U1 posts `cast` with `unit_id = U2` (owned by B, no delegation) → asserts 403 and zero rows inserted/updated for `(vote_id, U2)`.
- [ ] Integration test `voting_delegation_path_still_403s_when_delegate_not_owner` in the same file — proves the delegation-path gate is unaffected and does not regress.
- [ ] Regression test `voting_owner_can_still_cast_and_re_cast_for_own_unit` — user A owning U1 posts `cast` with `unit_id = U1`, then again with a different answer; asserts 200 and one row with the latest answer (existing `ON CONFLICT` behavior preserved).
- [ ] Local command: `cargo test -p api-server voting_cross_owner_cast` (new suite) and `cargo test -p api-server voting` (existing suite for regression coverage).

## Out of scope
- Refactoring `cast_vote_rls` beyond the call-site change (no INSERT semantics or `vote_weight` computation changes in this plan).
- Adding a unit-ownership cache — the `get_owners_rls` call is on the hot path only for cast, which is low-volume relative to reads; premature.
- Broader owner-vote governance rework (delegation chains, weighted-quorum edge cases) — separate track.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-voting-cast-no-unit-ownership.md`
- Mark the matching `backlog.json` row as `status: "done"`
