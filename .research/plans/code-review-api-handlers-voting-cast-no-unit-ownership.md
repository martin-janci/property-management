# code-review-api-handlers-voting-cast-no-unit-ownership

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review (dispatcher Tier-1d 2026-09-03 api-handlers)
**Confidence:** high

## Hypothesis
`cast_vote` in `backend/servers/api-server/src/routes/voting.rs` accepts a caller-supplied `req.unit_id` and, on the self path (`delegation_id == None`), does not check that the caller owns or occupies that unit. Because the repository INSERT uses `ON CONFLICT (vote_id, unit_id) DO UPDATE`, any authenticated tenant member can cast a ballot for a unit they do not own and overwrite the real owner's already-cast ballot. This changes the weighted outcome of a legally-significant owner vote (Slovak building governance). The delegation path already gates ownership; the self path must mirror it — add the missing `get_owners_rls` + `is_owner` check before constructing `CastVote`.

## Evidence
- `backend/servers/api-server/src/routes/voting.rs:1067-1180` — `cast_vote` (route: `voting.rs:47` POST `/api/v1/voting/{id}/cast`); on `delegation_id == None` it builds `CastVote { vote_id: id, user_id: rls.user_id(), unit_id: req.unit_id, .. }` (`:1155-1161`) with no ownership check.
- `backend/crates/db/src/repositories/vote.rs:224-278` — `cast_vote_rls` INSERTs verbatim, weight = `(SELECT ownership_share FROM units WHERE id = $3)` (target unit, not caller's), `ON CONFLICT (vote_id, unit_id) DO UPDATE SET user_id = EXCLUDED.user_id, answers = EXCLUDED.answers`. RLS is tenant-scoped, not unit-owner-scoped.
- Reference gate that must be mirrored: `backend/servers/api-server/src/routes/delegations.rs:207-233` — `create_delegation` calls `state.<unit_repo>.get_owners_rls(&mut **rls.conn(), unit_id)` then `let is_owner = owners.iter().any(|o| o.user_id == auth.user_id)` and returns 403 "You must be an owner of the unit ...".
- Delegation path in `cast_vote` (`voting.rs:1107-1128`) already checks delegate ownership via `EXISTS (SELECT 1 FROM vote_delegations WHERE delegator_user_id ... AND delegate_user_id = $2 AND unit_id = $3)`; only the self path is missing the check.
- Severity: authorization bypass enabling silent tampering with weighted vote results; reachable by any authenticated tenant member; integrity/data-corruption on a governance surface.

## Files
- `backend/servers/api-server/src/routes/voting.rs`
- `backend/crates/db/src/repositories/vote.rs`
- `backend/servers/api-server/src/routes/delegations.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Seed two owners in the same tenant: user A owns unit U1; user B owns unit U2. Create an active vote V that includes both units.
2. As user A, POST `/api/v1/voting/{V}/cast` with body `{ "unit_id": U2, "answers": {...} }` and no `delegation_id`.
3. Expected: 403 "You must be an owner of the unit ..." and U2's ballot untouched.
4. Actual on `main`: 200; a row appears in `vote_responses` with `vote_id=V`, `unit_id=U2`, `user_id=A`, `vote_weight = U2's ownership_share`; if B had already voted, the `ON CONFLICT` clause silently rewrites B's ballot.

## Suggested approach
1. In `backend/servers/api-server/src/routes/voting.rs::cast_vote`, after `existing = <vote_repo>.get_active_by_id(id)` and before constructing `CastVote` on the `delegation_id.is_none()` branch, look up unit owners on the RLS connection: `let owners = state.unit_repo.get_owners_rls(&mut **rls.conn(), req.unit_id).await?;` and reject with `Err(AppError::Forbidden("You must be an owner of the unit to cast a ballot".into()))` unless `owners.iter().any(|o| o.user_id == rls.user_id())`.
2. Mirror the exact error surface used by `delegations.rs` so audit logs group these consistently.
3. Keep the delegation path unchanged — it already verifies the delegation row and unit_id together.
4. Add a targeted trace at `debug!` level naming `unit_id`, `user_id`, and the check outcome so the auth path is greppable.
5. Do NOT alter `cast_vote_rls` — the check belongs at the handler boundary; the repository layer stays a raw INSERT so idempotent retries still work for legitimate callers.
6. Extend the existing `voting` integration test module with the failing-then-passing case in *Test plan* below.
7. Verify no other handlers in `voting.rs` accept caller-supplied `unit_id` on write paths (`update_response`, etc.); if any do, either apply the same gate or file follow-up.

## Alternatives considered
- **Enforce ownership at the DB layer via a stricter RLS policy on `vote_responses`** — rejected because unit ownership lives in `units`/`unit_owners` and RLS on `vote_responses` is currently tenant-scoped; changing the policy risks breaking legitimate delegation writes and admin backfills. The handler-level gate is smaller and mirrors the delegation path exactly.
- **Reject the endpoint entirely on the self path and require every ballot to go through a delegation row** — rejected because self-voting is the primary path and the delegation table is only used for proxy voting; forcing everyone through delegations distorts the data model and audit trail.

## Root-cause trace
1. Symptom: any authenticated tenant member can POST `/api/v1/voting/{V}/cast` with an arbitrary `unit_id` and either create or silently overwrite another owner's ballot.
2. ← Handler `cast_vote` at `voting.rs:1155-1161` constructs `CastVote { unit_id: req.unit_id, .. }` with no ownership check on the `delegation_id.is_none()` branch.
3. ← Repository `cast_vote_rls` at `vote.rs:224-278` trusts the passed `unit_id`, weights the ballot by the target unit's `ownership_share`, and applies `ON CONFLICT (vote_id, unit_id) DO UPDATE`, so a spoofed write cleanly replaces the legitimate row.
4. ← The delegation branch already enforces ownership (`voting.rs:1107-1128`), and `delegations.rs:207-233` shows the intended gate (`get_owners_rls` + `is_owner`) — the self path was written without mirroring it.
5. Origin: the `cast_vote` self path in `voting.rs` — introduce a `git log -L :cast_vote:voting.rs` walk during implementation to name the exact commit.

## Test plan
- [ ] Integration test in `backend/servers/api-server/tests/suites/` (e.g. `voting_cast_ownership_tests.rs`): user A owns U1, user B owns U2; A POSTs cast for `unit_id=U2` → asserts 403 and `vote_responses` unchanged; then A POSTs for `unit_id=U1` → 200. Test must fail on `main`.
- [ ] Follow-up scenario: B first casts for U2; A then POSTs cast for `unit_id=U2` → 403 and B's row still present.
- [ ] Run: `cargo test -p api-server --test suites voting_cast_ownership_tests` (or the crate/tests layout in place).

## Out of scope
- Rewriting the RLS policy on `vote_responses`.
- Refactoring the delegation branch or altering `ON CONFLICT` semantics for legitimate retries.
- Adding admin/manager override endpoints for corrective ballot rewrites.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-voting-cast-no-unit-ownership.md`
- Mark the matching `backlog.json` row as `status: "done"`
