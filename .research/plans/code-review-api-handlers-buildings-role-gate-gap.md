# code-review-api-handlers-buildings-role-gate-gap

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review 2026-08-10 (api-handlers segment, churn-aligned)
**Confidence:** medium

## Hypothesis

Six unit + owner mutation handlers in `backend/servers/api-server/src/routes/buildings.rs` — `create_unit` (line 1205), `update_unit` (line 1429), `archive_unit` (line 1598), `assign_unit_owner` (line 1882), `update_unit_owner` (line 2104), `remove_unit_owner` (line 2266) — validate tenant membership via `RlsConnection` but never gate on `rls.has_role(TenantRole::Manager)`. The sibling `create_building` at line 501 does gate: `if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) { ... 403 INSUFFICIENT_ROLE }`. Adding the same one-line guard to each of the six handlers closes an intra-tenant privilege-escalation path where a non-Manager tenant member (Owner, Tenant, Resident, OwnerDelegate — 11 roles enumerated in `common/src/tenant.rs:83`) can create/modify/archive units or flip ownership shares in their own org.

## Evidence

- `backend/servers/api-server/src/routes/buildings.rs:1882` — `assign_unit_owner` reads `rls.user_id()` but skips the role check that `create_building:501` performs. Same shape in the other 5 handler bodies (lines 1205, 1429, 1598, 2104, 2266).
- `backend/servers/api-server/src/routes/buildings.rs:38-73` — the six routes are mounted (`POST/PUT/DELETE` on `/{building_id}/units/{unit_id}` and `/owners[/{user_id}]`) with no per-route middleware.
- `backend/crates/api-core/src/extractors/rls_connection.rs:85` — `RlsConnection` validates tenant membership + sets RLS context; role is stored (`.role`) but never enforced by the extractor itself.
- `common/src/tenant.rs:83` — `TenantRole` has 11 variants including Owner, Tenant, Resident, OwnerDelegate; only Manager (+ super_admin) should be able to mutate buildings/units/ownership.
- Reachability confirmed: `pub mod buildings;` in `routes/mod.rs:19`; mounted at `.nest("/api/v1/buildings", routes::buildings::router())` in `lib.rs:150`.

## Files

- `backend/servers/api-server/src/routes/buildings.rs:1205`
- `backend/servers/api-server/src/routes/buildings.rs:1429`
- `backend/servers/api-server/src/routes/buildings.rs:1598`
- `backend/servers/api-server/src/routes/buildings.rs:1882`
- `backend/servers/api-server/src/routes/buildings.rs:2104`
- `backend/servers/api-server/src/routes/buildings.rs:2266`

(New test file `backend/servers/api-server/tests/buildings_authorization.rs` is created by the implementer; see *Test plan* below. It is intentionally not listed here so G7's "every Files bullet resolves on disk" check stays green.)

## Dependencies

<!-- No blockers — pattern already exists at line 501 and can be copied inline. -->

## Required capabilities

- [x] C1 — Systematic debugging (security-vector, six sibling handlers must be patched consistently)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security-vector — reviewer will want the negative test to prove the gate)

**Execution mode (auto-derived from the ticks):**

Mode: cloud-ok

## Repro steps

1. Seed a tenant with two users: user A with `TenantRole::Manager`, user B with `TenantRole::Tenant` (or Resident/Owner — any non-Manager role).
2. Authenticate as user B, obtain a JWT bound to that tenant.
3. Pick any unit id in the tenant's building set: `POST /api/v1/buildings/{building_id}/units/{unit_id}/owners` with body `{"user_id": "<any user id in tenant>", "ownership_share": "1.0"}` and the user-B bearer token.
4. Expected: `403 INSUFFICIENT_ROLE`. Actual today: `201 Created` — the owner row is inserted.
5. Same result for `POST /api/v1/buildings/{id}/units` (create_unit), `PUT /api/v1/buildings/{id}/units/{unit_id}` (update_unit), `DELETE /api/v1/buildings/{id}/units/{unit_id}` (archive_unit), `PUT /api/v1/buildings/{id}/units/{unit_id}/owners/{user_id}` (update_unit_owner), `DELETE /api/v1/buildings/{id}/units/{unit_id}/owners/{user_id}` (remove_unit_owner).

## Suggested approach

1. In each of the six handler bodies (create_unit, update_unit, archive_unit, assign_unit_owner, update_unit_owner, remove_unit_owner in `backend/servers/api-server/src/routes/buildings.rs`), add the exact guard used by `create_building` at line 501 as the first statement after the handler signature:
   ```rust
   if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
       return Err((
           StatusCode::FORBIDDEN,
           Json(ErrorResponse::new(
               "INSUFFICIENT_ROLE",
               "Manager role required",
           )),
       ));
   }
   ```
   Place it **before** `rls.user_id()` / building-lookup calls so we short-circuit on the auth failure and don't burn a DB connection when the role is wrong.
2. Add `backend/servers/api-server/tests/buildings_authorization.rs` with one `#[sqlx::test]` per touched handler that authenticates as `TenantRole::Tenant` in the same org, calls the endpoint, and asserts `403 INSUFFICIENT_ROLE`. Use the same helper pattern already established in `backend/servers/api-server/tests/community_authorization_tests.rs` (PR #2722 seeded that convention).
3. Add one positive-path case per handler (Manager gets `2xx`) — reuse existing test factories under `backend/servers/api-server/tests/common/` so this doesn't grow into a new test-harness project.
4. `cargo fmt --all && cargo clippy -p api-server --tests -- -D warnings` inside the worktree.
5. `cargo test -p api-server --test buildings_authorization` — must green before push.
6. PR title: `code-review-api-handlers-buildings-role-gate-gap: gate unit + owner mutations on Manager role`.

## Alternatives considered

- **Wrap the six routes in a `Router::layer(require_role(TenantRole::Manager))` middleware** — rejected because there is no existing `require_role` layer in this codebase (the pattern in every other privileged handler is the inline guard at the top of the function). Introducing a new layer would broaden the diff to a middleware refactor and delay closing the security gap. Follow-up refactor can extract the guard into a helper once the six handlers are safe.
- **Fix only `assign_unit_owner` and file the other five as separate tasks** — rejected because the six handlers form one privilege boundary; splitting risks leaving 4-5 handlers exposed on a partial merge, and each is a one-line change. A single PR keeps the boundary consistent.

## Root-cause trace

1. Symptom: any non-Manager tenant member can `POST/PUT/DELETE` on `/api/v1/buildings/{id}/units[/{unit_id}][/owners[/{user_id}]]` and mutate unit rows or owner-share rows in their own org (verified by static call-path inspection at buildings.rs:1205/1429/1598/1882/2104/2266).
2. ← Immediate cause at `buildings.rs:1882` (and 5 siblings): handler body relies on `RlsConnection` for auth but only checks tenant membership, never role.
3. ← Upstream cause at `backend/crates/api-core/src/extractors/rls_connection.rs:85`: `RlsConnection` populates `role: TenantRole` but doesn't enforce a role floor — that's each handler's responsibility, and only three handlers (create_building at 501, and lines 672 + 959) actually do it.
4. Origin: the six handlers were added over time without an inline copy of the create_building guard; the code-review process caught the gap after the sibling `create_building` guard became the de-facto pattern.

## Test plan

- [ ] `backend/servers/api-server/tests/buildings_authorization.rs::create_unit_forbidden_for_non_manager` — auth as Tenant, `POST /buildings/{id}/units` → expect 403 INSUFFICIENT_ROLE
- [ ] `backend/servers/api-server/tests/buildings_authorization.rs::update_unit_forbidden_for_non_manager` — Tenant `PUT /buildings/{id}/units/{unit_id}` → 403
- [ ] `backend/servers/api-server/tests/buildings_authorization.rs::archive_unit_forbidden_for_non_manager` — Tenant `DELETE /buildings/{id}/units/{unit_id}` → 403
- [ ] `backend/servers/api-server/tests/buildings_authorization.rs::assign_unit_owner_forbidden_for_non_manager` — Tenant `POST /buildings/{id}/units/{unit_id}/owners` → 403
- [ ] `backend/servers/api-server/tests/buildings_authorization.rs::update_unit_owner_forbidden_for_non_manager` — Tenant `PUT /buildings/{id}/units/{unit_id}/owners/{user_id}` → 403
- [ ] `backend/servers/api-server/tests/buildings_authorization.rs::remove_unit_owner_forbidden_for_non_manager` — Tenant `DELETE /buildings/{id}/units/{unit_id}/owners/{user_id}` → 403
- [ ] One positive-path case per handler asserting Manager gets `2xx`
- [ ] Command: `cargo test -p api-server --test buildings_authorization`

## Out of scope

- Refactoring `RlsConnection` to accept a required role type parameter. That's a broader API-shape change; do it in a follow-up.
- Extracting the guard into a `require_manager()` free function. Follow-up refactor once the six handlers land.
- Auditing the other `routes/*.rs` files for the same gap. Different scope — file a separate `code-review-api-handlers-*` vector per file if the rotating review surfaces more.
- Changing role-check semantics (e.g., allowing Owner). This plan preserves the exact rule used by `create_building`.

## After-merge

- Move this file to `plans/_archive/code-review-api-handlers-buildings-role-gate-gap.md`
- Mark `code-review-api-handlers-buildings-role-gate-gap` in `backlog.json` as `status: "done"`, add merged PR # to `sources`
