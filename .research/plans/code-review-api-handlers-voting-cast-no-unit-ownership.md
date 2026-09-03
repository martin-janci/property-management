# code-review-api-handlers-voting-cast-no-unit-ownership

**Vector:** security
**Score:** 3
**Source:** dispatcher Tier-1d rotating-expert-review 2026-09-03 (api-handlers)
**Confidence:** high

## Hypothesis
The self-vote path in `cast_vote` accepts a caller-supplied `unit_id` without verifying that the caller owns the target unit; the backing repository INSERT uses `ON CONFLICT (vote_id, unit_id) DO UPDATE` which overwrites any pre-existing ballot for that unit. Combined, any authenticated member of a tenant can silently overwrite another owner's weighted ballot on a legally-significant Slovak building-governance vote — ballot stuffing / vote fraud. Fix: gate the self path with the same `unit_repo.get_owners_rls` + `is_owner` check that `delegations.rs` already uses.

## Evidence
- `backend/servers/api-server/src/routes/voting.rs:1067` — `cast_vote` handler entry; only re-validates ownership on the `req.delegation_id.is_some()` branch. Route mounted at `voting.rs:47` as `POST /{id}/cast`.
- `backend/servers/api-server/src/routes/voting.rs:1155` — self path constructs `CastVote { unit_id: req.unit_id, .. }` and calls `cast_vote_rls` with no ownership check.
- `backend/crates/db/src/repositories/vote.rs:224` — `cast_vote_rls` INSERTs using caller-supplied `unit_id`, sets `vote_weight = COALESCE((SELECT ownership_share FROM units WHERE id = $3), 1.0)`, and `ON CONFLICT (vote_id, unit_id) DO UPDATE SET user_id = EXCLUDED.user_id, ...`.
- `backend/servers/api-server/src/routes/delegations.rs:207` — `create_delegation` DOES enforce the missing check (`get_owners_rls` → `is_owner` → `403 NOT_OWNER`). Same helper, same executor, same request field; the pattern to mirror already exists in-repo.

## Files
- `backend/servers/api-server/src/routes/voting.rs`
- `backend/servers/api-server/src/routes/delegations.rs`
- `backend/crates/db/src/repositories/vote.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [x] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Seed one active `votes` row (status=active, allow_delegation=false) in an organization O; create two owner users A and B; unit U1 owned by A, U2 owned by B.
2. Authenticated as A, POST `/api/v1/voting/{vote_id}/cast` with `{ "unit_id": "<U2>", "answers": {...}, "delegation_id": null }`.
3. Expected: `403 NOT_OWNER` and no row in `vote_responses` for `(vote_id, U2)` (mirroring the delegations 403).
4. Actual on `main`: `200 OK` with a `VoteReceipt`; a `vote_responses` row is inserted for `(vote_id, U2)` with `user_id = A` and `vote_weight = U2.ownership_share`. If B had already voted, the `ON CONFLICT` overwrites B's row silently.

## Suggested approach
1. In `backend/servers/api-server/src/routes/voting.rs::cast_vote`, right after the delegation branch (around `voting.rs:1150`, before constructing `CastVote`), load unit owners on the self path: `let owners = state.unit_repo.get_owners_rls(&mut **rls.conn(), req.unit_id).await` with the same error-mapping shape used in `delegations.rs:210`.
2. Reject with `StatusCode::FORBIDDEN` + `ErrorResponse::new("NOT_OWNER", "You must be an owner of the unit to cast a vote for it")` when `!owners.iter().any(|o| o.user_id == rls.user_id())`. Match the exact code + status used by `delegations.rs:230` so clients get one contract.
3. Keep the delegation branch untouched — its existing `is_active_delegation` EXISTS query already scopes to `(delegator_user_id, delegate_user_id, unit_id)` and covers the owner-of-delegator dimension.
4. Add an integration test in the api-server `tests/` tree that fails on `main` and passes after the fix (see Test plan).
5. No repository change is strictly required (handler-level fix); leave `cast_vote_rls` as-is to keep the fix small. Note in the PR body that a defense-in-depth follow-up (owner check inside `cast_vote_rls` itself) is captured under `code-review-api-handlers-vote-update-no-tenant-rls`.

## Alternatives considered
- **Enforce ownership inside `cast_vote_rls` via a `SELECT ... FROM units WHERE id = $3 AND EXISTS (SELECT 1 FROM unit_owners ...)` sub-check inline in the INSERT** — rejected because it splits the authorization contract across two layers, complicates ergonomic error mapping (repo returns a `sqlx::Error`, not a typed authz error), and diverges from the in-repo pattern established by `delegations.rs`. Handler-level check keeps the read-then-write in one place and is trivially unit-testable.
- **Drop the `ON CONFLICT DO UPDATE` and switch to `ON CONFLICT DO NOTHING`** — rejected because a legitimate re-cast (owner changing their mind before poll close) needs to update the row; removing the update would introduce a new "cannot correct my ballot" bug while only partially closing the exploit (the first-write case still lets A cast for U2).

## Root-cause trace
1. Symptom: authenticated tenant member A casts a ballot for `unit_id = U2` (owned by B); server responds 200 and stores/overwrites the row.
2. ← `voting.rs:1155` builds `CastVote { unit_id: req.unit_id, .. }` without an owner check on the self path.
3. ← `voting.rs:1067` (`cast_vote` handler) only wires the owner check into the `req.delegation_id.is_some()` branch (the delegations EXISTS query at :1107-1128).
4. ← `vote.rs:224` (`cast_vote_rls`) trusts its `data.unit_id` argument and uses `ON CONFLICT (vote_id, unit_id) DO UPDATE SET user_id = EXCLUDED.user_id, ...`, so an unauthorized ballot silently supersedes a prior legitimate one.
5. Origin: the delegation path was added later (see `create_delegation` in `delegations.rs`) with a dedicated owner check; the self path pre-dates that hardening and was never back-ported.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/voting_cast_owner_guard_tests.rs` (new) — authenticated `A` POSTs `/voting/{id}/cast` with `unit_id = U2` (owned by B); asserts 403 `NOT_OWNER` and that no `vote_responses` row exists for `(vote_id, U2)`.
- [ ] Regression scenario: seed a legitimate ballot from B on `(vote_id, U2)`, then attempt the same cross-owner cast from A; assert the pre-existing row is unchanged (`user_id == B.id`).
- [ ] Delegation branch unchanged: `A` casts for `U2` with a valid `delegation_id` where `B` delegated `U2` to `A` — still succeeds (existing test must keep passing).
- [ ] Command: `cargo test -p api-server --test suites voting_cast_owner_guard_tests`

## Out of scope
- Adding a repository-level ownership check inside `cast_vote_rls` (defense in depth) — covered by `code-review-api-handlers-vote-update-no-tenant-rls` follow-up.
- Sanitizing raw sqlx error messages returned by `voting.rs` handlers — covered by the wider `db_error` sweep already in-flight.
- Reworking `ON CONFLICT` semantics for ballot corrections.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-voting-cast-no-unit-ownership.md`
- Mark the matching `backlog.json` row as `status: "done"`
