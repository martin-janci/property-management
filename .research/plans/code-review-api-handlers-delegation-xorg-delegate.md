# code-review-api-handlers-delegation-xorg-delegate

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review (Tier-1d api-handlers 2026-11-08) · verified in-repo 2026-11-09 · signal `code-review-api-handlers-delegation-xorg-delegate`
**Confidence:** high

## Hypothesis
`create_delegation` (POST `/api/v1/delegations`) resolves the incoming `delegate_user_id` with the non-RLS `state.user_repo.find_by_id` — a global existence check only. There is no verification that the delegate shares the delegator's organization. An authenticated owner can therefore delegate scopes `voting`, `financial`, `documents`, `faults`, or `all` to a user from any other organization, granting the outside party proxy authority over governance and financial actions inside the owners' association. The smallest safe fix is to mirror the `authorize_unit_inner` org gate already used by `unit_residents.rs:79-92` — call `state.org_member_repo.is_member(building.organization_id, delegate_user_id)` (or the equivalent owner-tenant lookup) and return `400 OUT_OF_ORG` / `403` when the delegate is not a member.

## Evidence
- `backend/servers/api-server/src/routes/delegations.rs:183-205` — delegate existence is `state.user_repo.find_by_id(req.delegate_user_id)` only; no org/tenant gate.
- `backend/servers/api-server/src/routes/delegations.rs:158` — scopes `voting | financial | documents | faults | all` are allowed to be granted to that unverified `delegate_user_id`.
- `backend/servers/api-server/src/routes/delegations.rs:~430` — the `accept_delegation` path only checks `delegate_user_id == auth.user_id`; there is no downstream org gate on acceptance either.
- `backend/servers/api-server/src/routes/unit_residents.rs:79-84` — canonical org gate pattern: `state.org_member_repo.is_member(building.organization_id, user_id).await`.
- `backend/servers/api-server/src/state.rs:278,509,680` — `state.org_member_repo: OrganizationMemberRepository` is already wired on `AppState`.

## Files
- `backend/servers/api-server/src/routes/delegations.rs:183`
- `backend/servers/api-server/src/routes/unit_residents.rs:79`
- `backend/servers/api-server/tests/suite_2.rs`
- `backend/servers/api-server/tests/suites`

## Required capabilities
- [x] C1 — Systematic debugging
- [x] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Seed two organizations `org_a`, `org_b`, each with one owner user (`owner_a` in `org_a`, `other_b` in `org_b`) and one unit owned by `owner_a` in `org_a`.
2. `POST /api/v1/auth/login` as `owner_a` → bearer + refresh.
3. `POST /api/v1/delegations` as `owner_a` with body `{"delegate_user_id":"<other_b.id>","unit_id":"<unit_a.id>","scopes":["financial","voting"],"expires_at":"<+30d ISO>"}`.
4. Expected: `400` / `403 OUT_OF_ORG` (delegate is not an `org_a` member). Actual on pre-fix: `201` with a persisted `delegations` row granting `other_b` cross-org financial + voting proxy over the `org_a` unit.

## Suggested approach
1. Add an org-gate after the existence check in `routes/delegations.rs:create_delegation` at ~L204: resolve the delegator's organization — when `unit_id` is provided, look it up via `unit_repo.find_by_id` / `building_repo` to get `organization_id`; when `unit_id` is `None`, use `auth.organization_id` from the request's tenant context.
2. Call `state.org_member_repo.is_member(organization_id, req.delegate_user_id).await`; on `Err` → `500 DB_ERROR` (same shape as the existing failure path at L193-198); on `Ok(false)` → `400 OUT_OF_ORG` with a body like `ErrorResponse::new("OUT_OF_ORG","Delegate user is not a member of this organization")`.
3. On `Ok(true)` fall through to the existing ownership check at L207-236.
4. Mirror the same gate on `accept_delegation` for defence-in-depth — verify the delegate still belongs to the delegation's organization at accept time (membership can change between `create` and `accept`).
5. Add a regression suite under `backend/servers/api-server/tests/suites/delegation_xorg_tests.rs`, register it in `backend/servers/api-server/tests/suite_2.rs` using the same `#[path = …]` pattern already used by `change_password_session_revocation_tests` (see commit `d5e36f0` / PR #3353 for the register-and-use shape).
6. Keep the error code stable across `create` and `accept` so downstream tooling can detect the class.
7. `cargo clippy -p api-server --all-targets -- -D warnings` and `cargo fmt --all` before pushing.

## Alternatives considered
- **Gate in a middleware** — rejected because delegation resolves the target organization from either `req.unit_id` or `auth.organization_id`; a generic middleware does not have that context and would double-query. The handler is the correct layer.
- **Rely on `authorize_unit_inner`** — rejected because that helper gates the *caller* (delegator) only; the delegate is a separate principal that the helper does not touch, so reusing it does not close the gap.

## Root-cause trace
1. Symptom: `POST /api/v1/delegations` succeeds with a cross-org `delegate_user_id`; the delegation row grants out-of-org proxy authority for `voting`/`financial`/`documents`/`faults`/`all`.
2. ← `routes/delegations.rs:183-205` — existence check is `user_repo.find_by_id` (global, non-RLS); no `is_member` call follows.
3. ← `routes/delegations.rs:~430 accept_delegation` — only `delegate_user_id == auth.user_id`; no org re-check, so even if the delegate later leaves the org the acceptance still lands.
4. Origin: the delegations module was added without reusing the `authorize_unit_inner` / `is_member` pattern already established by `unit_residents.rs:79-84`.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/delegation_xorg_tests.rs` — `create_delegation_rejects_out_of_org_delegate` (fails on pre-fix: 201 OK; passes after: 400/403 OUT_OF_ORG).
- [ ] Same suite — `accept_delegation_rejects_delegate_no_longer_in_org` (membership revoked between create+accept → accept returns 403).
- [ ] Positive regression — `create_delegation_allows_same_org_delegate` (same-org delegate must still succeed so we don't over-tighten).
- [ ] Command: `cargo test -p api-server --test suite_2 delegation_xorg_tests -- --nocapture`

## Out of scope
- Rewriting the delegation scope model or expiry semantics.
- Admin / god-mode delegation (a separate flow).
- Non-`POST /api/v1/delegations` entry points (the `/api/v1/portfolio/...` delegation helpers were not read for this plan).

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-delegation-xorg-delegate.md`
- Mark the matching `backlog.json` row as `status: "done"`
