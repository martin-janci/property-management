# code-review-api-handlers-delegation-xorg-delegate

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review (dispatcher Tier-1d 2026-11-08 api-handlers)
**Confidence:** high

## Hypothesis
`create_delegation` in `backend/servers/api-server/src/routes/delegations.rs` validates
only the delegate user's *global* existence (`state.user_repo.find_by_id`, the non-RLS repo)
before persisting the delegation; there is no check that the delegate shares the delegator's
organization. An owner can therefore grant scopes `voting` / `financial` / `documents` /
`faults` / `all` to a user from any other organization — a cross-org privilege grant that
later allows that outside delegate to act as a proxy for governance and financial actions
inside an owners' association they do not belong to. The smallest safe fix is an
`org_member_repo.is_member(building.organization_id, req.delegate_user_id)` gate added
immediately after the existence check (mirroring the pattern already used in
`unit_residents.rs::authorize_unit_inner` L79-92).

## Evidence
- `backend/servers/api-server/src/routes/delegations.rs:183-205` — `find_by_id` on the
  non-RLS user repo is the only validation gate for `delegate_user_id`; confirmed in
  this routine run by reading lines 150-240 directly.
- `backend/servers/api-server/src/routes/delegations.rs:207-236` — the owner check gates
  only the *delegator* (via `get_owners_rls`); the delegate is never org-gated on the
  unit-scoped branch either.
- `backend/servers/api-server/src/routes/delegations.rs:432` — the accept path
  (`accept_delegation`) only asserts `delegate_user_id == auth.user_id`, so there is no
  downstream org gate to fail-closed on.
- Contrast pattern that already enforces this: `backend/servers/api-server/src/routes/unit_residents.rs:79-92`
  (`authorize_unit_inner` -> `org_member_repo.is_member`).

## Files
- `backend/servers/api-server/src/routes/delegations.rs:183`
- `backend/servers/api-server/src/routes/unit_residents.rs:79`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Seed two organizations A and B with distinct users (`user_a` in A owns `unit_a`; `user_b` in B).
2. As `user_a`, POST `/api/v1/delegations` with body `{ "delegate_user_id": "<user_b>", "unit_id": "<unit_a>", "scopes": ["voting","financial"] }`.
3. Expected: `400 Bad Request` with code `DELEGATE_NOT_MEMBER` (or `404` consistent with other cross-org gates). Actual: `201 Created` — the delegation persists and `user_b` can later accept it and act for `user_a` inside org A.

## Suggested approach
1. Resolve the delegator's organization id: on the unit-scoped branch (L208-236) take `building.organization_id` from the unit->building lookup already performed; on the unit-less branch (no `req.unit_id`), reject entirely — a plain-delegation across the whole platform has no org anchor and the current code already required a unit for owner gating, so an explicit 400 here closes the latent hole.
2. After the existence check at L184-194 (and inside the `if let Some(unit_id)` branch where the org id is known), add `state.org_member_repo.is_member(org_id, req.delegate_user_id).await?` and return `StatusCode::BAD_REQUEST` with code `DELEGATE_NOT_MEMBER` on `Ok(false)`.
3. Keep the RLS release idiom from the surrounding file: call `rls.release().await` before every early return (match the existing pattern at L161-168).
4. Log the rejection via `tracing::warn!(?org_id, delegate = %req.delegate_user_id, "cross-org delegation rejected")` so SOC can monitor abuse attempts.
5. Mirror the gate inside `accept_delegation` (defence in depth): if the delegation row's org drift from the accepter's membership, 403; prevents a stale pre-migration row from being accepted by a now-foreign user.

## Alternatives considered
- **Gate at the repository layer (push into `delegation_repo.create`)** — rejected because the org id is only resolved from the unit lookup in the handler; moving the check to the repo would require re-fetching the unit->building->org chain and the RLS connection is already open in the handler.
- **Allow cross-org delegations but require an explicit `cross_org: true` flag plus manager approval** — rejected as scope creep: no product requirement for cross-org proxies exists, and loosening the invariant now bakes the current bug into the API shape.

## Root-cause trace
1. Symptom: an owner in org A grants `voting` scope to a user in org B; the API returns 201 and the accepted delegation then lets the org-B user vote on org-A's assembly motions.
2. ← `routes/delegations.rs:183-205` — only a global existence check on `req.delegate_user_id`; no org gate.
3. ← `routes/delegations.rs:432` — acceptance path validates only the accepter identity, not org membership.
4. Origin: initial `create_delegation` implementation (predates PR #2789 / client-ip hardening era). The pattern copied the unit-owner check but did not copy the org-membership gate used in `unit_residents.rs`.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/delegations_tests.rs` — new case `create_delegation_rejects_cross_org_delegate`: seeds two orgs with a user each, posts the delegation, asserts `400 Bad Request` + `DELEGATE_NOT_MEMBER` code and that `delegation_repo.find_by_id` returns `None`.
- [ ] Same file — regression case `create_delegation_allows_same_org_delegate` asserting the happy path still returns 201 + persists.
- [ ] Exact command: `cargo test -p api-server --test suites delegations_tests::create_delegation_rejects_cross_org_delegate`.

## Out of scope
- Rewriting `accept_delegation` to a general invariant checker.
- Refactoring the shared `authorize_unit_inner` helper into a generic org gate.
- Touching `revoke_delegation` (no cross-org exposure on the revoke path).

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-delegation-xorg-delegate.md`
- Mark the matching `backlog.json` row as `status: "done"`
