# code-review-api-handlers-voting-cast-no-unit-ownership

**Vector:** security
**Score:** 3
**Source:** dispatcher Tier-1d rotating-expert-review 2026-09-03 (api-handlers segment)
**Confidence:** high

## Hypothesis
`POST /api/v1/voting/{id}/cast` accepts a caller-supplied `unit_id` and on the self-cast path (delegation_id == None) never checks that the caller owns/occupies that unit. Combined with the repository's `ON CONFLICT (vote_id, unit_id) DO UPDATE` on `vote_responses`, any authenticated tenant member can silently overwrite another owner's ballot for a unit they do not own — an authorization bypass on a legally-significant weighted-vote surface (Slovak building governance). The delegation path already enforces the correct check via `get_owners_rls`; the self-cast path must mirror it before constructing `CastVote`.

## Evidence
- `backend/servers/api-server/src/routes/voting.rs:1067-1180` — `cast_vote` gates only on `existing.is_active()` and re-validates ownership solely inside the `delegation_id.is_some()` EXISTS query (lines 1107-1128). The `else` branch (self-cast) builds `CastVote { vote_id: id, user_id: rls.user_id(), unit_id: req.unit_id, .. }` (lines 1155-1161) with **no owner check** on `req.unit_id`.
- `backend/crates/db/src/repositories/vote.rs:224-278` — `cast_vote_rls` inserts using the caller-supplied `unit_id`/`user_id` verbatim, sets `vote_weight = COALESCE((SELECT ownership_share FROM units WHERE id = $3), 1.0)` (target unit's weight!), and uses `ON CONFLICT (vote_id, unit_id) DO UPDATE SET user_id = EXCLUDED.user_id, answers = EXCLUDED.answers, ...`. RLS on this insert is org-scoped, not unit-owner-scoped.
- `backend/servers/api-server/src/routes/delegations.rs:207-233` — `create_delegation` already implements exactly the missing gate: `state.<unit_repo>.get_owners_rls(&mut **rls.conn(), unit_id)` → `let is_owner = owners.iter().any(|o| o.user_id == auth.user_id)` → `403 "You must be an owner of the unit ..."` when not owner. The same shape belongs in `cast_vote`'s self-cast branch.

## Files
- `backend/servers/api-server/src/routes/voting.rs:1067`
- `backend/crates/db/src/repositories/vote.rs:224`
- `backend/servers/api-server/src/routes/delegations.rs:207`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [x] C2 — Seed data (needs an org with two owner-users on distinct units + an active vote)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Seed: org O with active vote V; unit U1 owned by user A; unit U2 owned by user B; both A and B are org members.
2. Authenticate as user A and `POST /api/v1/voting/{V}/cast` with body `{ "unit_id": "<U2>", "answers": [...], "delegation_id": null }`.
3. Expected (post-fix): 403 with an "owner-of-unit required" error; `vote_responses` has no row for (V, U2) from A, and any existing B ballot is untouched.
4. Actual on `main`: 200/OK; a `vote_responses` row is upserted with `unit_id = U2`, `user_id = A`, `vote_weight = ownership_share of U2` — silently replacing B's ballot if one existed.

## Suggested approach
1. In `voting.rs::cast_vote`, immediately after loading `existing` (the vote) and confirming `existing.is_active()`, but **before** constructing `CastVote` (lines 1155-1161), add a self-cast owner gate scoped to `delegation_id.is_none()`.
2. Fetch the unit's owners via the same helper the delegation path uses: `state.<unit_repo>.get_owners_rls(&mut **rls.conn(), req.unit_id).await`. Use the exact repository accessor that `delegations.rs:210-215` calls (confirm the field name on `state` — likely `unit_repo` or embedded in another service handle).
3. Reject with `403` (mirror the delegation-path status/error shape) if `!owners.iter().any(|o| o.user_id == rls.user_id())`. Keep the message class-neutral: e.g. `"You must be an owner of the unit to cast a ballot for it"`; keep the error code aligned with `delegations.rs`'s existing "not-owner" code so clients handle both paths the same way.
4. Keep the delegation path untouched — the existing EXISTS check at lines 1107-1128 already enforces the delegate ↔ unit binding.
5. Do NOT change `vote.rs::cast_vote_rls` — the `ON CONFLICT` is legitimate for the same-owner idempotent re-cast case; the gate lives at the route layer where the caller's identity is bound.
6. Add a regression integration test: covers (a) owner casts for their own unit → 200 and row persisted; (b) non-owner casts for another unit → 403 and no row appears / no existing row is overwritten; (c) delegation-path unchanged. Test file: `backend/servers/api-server/tests/suites/vote_cast_owner_gate_tests.rs` (mirrors the existing `delegation_scopes_enum_decode_tests.rs` style).
7. Run the failing test on `main` first (IG3) to confirm it reproduces the bypass; run again after the fix to confirm 403.

## Alternatives considered
- **Enforce the check in the repository (`cast_vote_rls`) instead of the route** — rejected because `vote.rs` is DB-facing and does not know the caller identity abstraction (`auth.user_id` / `rls.user_id()`) in the same shape as the route; duplicating that plumbing into the repo layer creates two owner-check sources that can drift. Route-layer enforcement mirrors `delegations.rs` and keeps the gate near the caller-identity boundary.
- **Tighten RLS on `vote_responses` to unit-owner scope** — rejected because unit ownership is not a tenant partition; expressing it as an RLS policy would require an owner subquery on every write and would leak the same enforcement across paths that legitimately do not need it (e.g. audit trails). The application-layer gate is cheaper and matches the existing delegation-path pattern.

## Root-cause trace
1. Symptom: authenticated user A can cast a ballot on unit U2 (owned by B) and overwrite B's existing vote via `ON CONFLICT DO UPDATE`.
2. ← Immediate cause: `voting.rs:1155-1161` builds `CastVote { unit_id: req.unit_id, .. }` in the self-cast branch with no `is_owner` check.
3. ← Upstream cause: the ownership check exists only inside the `delegation_id.is_some()` EXISTS query at `voting.rs:1107-1128`, so removing/omitting `delegation_id` from the request drops the caller into an ungated branch.
4. ← Deeper cause: `vote.rs::cast_vote_rls` at `backend/crates/db/src/repositories/vote.rs:224-278` treats `(vote_id, unit_id)` as the natural idempotency key via `ON CONFLICT`, so an unchecked `unit_id` from the route becomes a ballot-overwrite primitive.
5. Origin: introduced when the delegation branch was added to `cast_vote` and the owner check migrated *into* that branch instead of being kept as a precondition covering both branches. Confirm via `git log -L :cast_vote:backend/servers/api-server/src/routes/voting.rs` — the pre-delegation revision either enforced or lacked the check globally; this plan's fix restores the invariant on the self-cast path.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/vote_cast_owner_gate_tests.rs` — new integration test: seed org + vote + two owners; assert 403 on cross-unit self-cast; assert 200 on own-unit self-cast; assert existing B ballot survives an A cross-cast attempt.
- [ ] Extend the failing case first (IG3): the "non-owner casts for U2" scenario must fail on `main` (expect 200 today, 403 after fix).
- [ ] Delegation path regression: an existing valid delegation self-cast still succeeds after the fix (test in the same file).
- [ ] Run: `cd backend && cargo test -p api-server --test main -- suites::vote_cast_owner_gate` (adjust `--test` name to match repo convention if the tests suite entrypoint is different — confirm via `ls backend/servers/api-server/tests/`).

## Out of scope
- Changing the `ON CONFLICT` semantics of `cast_vote_rls`.
- Adding RLS policies for unit-ownership scoping.
- Auditing older ballots to detect prior exploitation — mention in a follow-up issue if remediation of historical data is required.
- Any UI-side changes (the exploit is server-authoritative; UI never surfaces the bypass path).

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-voting-cast-no-unit-ownership.md`
- Mark the matching `backlog.json` row as `status: "done"`
