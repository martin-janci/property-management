# security-equipment-idor

**Vector:** security
**Score:** 3
**Source:** code-review api-core 2026-05-25 — handlers `delete_equipment` / `update_equipment` / `update_maintenance` in `backend/servers/api-server/src/routes/ai.rs`
**Confidence:** high

## Hypothesis
The equipment CRUD endpoints under `/api/v1/ai/.../equipment` bind the authenticated principal as `_principal` and discard it, passing only the path UUID into the repository. `delete_equipment` calls `state.equipment_repo.delete(id)`, which runs `DELETE FROM equipment WHERE id = $1` with no org/tenant/owner predicate, so any authenticated user can delete any other tenant's equipment by supplying its UUID — a cross-tenant write (IDOR). `update_equipment` (`equipment_repo.update(id, data)`) and `update_maintenance` (`equipment_repo.update_maintenance(id, data)`) share the identical discard pattern. This is the same IDOR class the team just fixed for voice devices in PR #461; the sibling workflow handlers (`update_workflow`/`delete_workflow`) already scope correctly via `org_id`, proving the codebase's intended pattern. The smallest correct fix is to thread the caller's org/tenant scope into each equipment repository call and add the predicate to the SQL, returning `404` on a non-owned row.

## Evidence
- `backend/servers/api-server/src/routes/ai.rs:1133` — `delete_equipment(State, _principal: RequestPrincipal, Path(id))` ignores `_principal` and calls `state.equipment_repo.delete(id)`
- `backend/servers/api-server/src/routes/ai.rs:1042` — route wired: `.route("/{id}", delete(delete_equipment))` (reachable production route)
- `backend/crates/db/src/repositories/equipment.rs:143` — `pub async fn delete(&self, id: Uuid)` runs `DELETE FROM equipment WHERE id = $1` (equipment.rs:144) with no owner/org predicate
- `backend/servers/api-server/src/routes/ai.rs` — `update_equipment` calls `equipment_repo.update(id, req)` (`equipment.rs:104` — `UPDATE equipment SET ... WHERE id` unscoped) and `update_maintenance` calls `equipment_repo.update_maintenance(id, req)` (`equipment.rs:213`) — both discard `_principal`
- Contrast: `update_workflow`/`delete_workflow` in the same file call `workflow_repo.update(id, org_id, req)` / `delete(id, org_id)` — scoped; `acknowledge_alert` passes `principal.user_id` — scoped. The equipment cluster is the lone unscoped group.
- Found by the rotating expert (Rust) review of segment `api-core`; call path traced end-to-end from the wired Axum route to the unscoped `DELETE`/`UPDATE`. Same pattern fixed for voice devices in PR #461 (merged 2026-05-24).

## Files
- `backend/servers/api-server/src/routes/ai.rs`
- `backend/crates/db/src/repositories/equipment.rs`

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Authenticate as user A (org 1) and create an equipment row (`POST /api/v1/ai/.../equipment`); note its returned `id` (`EQ_A`).
2. Authenticate as a different user B in org 2, with no relationship to `EQ_A`.
3. As B, call `DELETE /api/v1/ai/.../equipment/{EQ_A}`.
4. Expected: `404 Not Found` (B cannot see or affect org 1's equipment). Actual today: `204 No Content` and the row is deleted — B silently destroyed org 1's equipment record. The same holds for `PUT /equipment/{EQ_A}` (update) and `PUT /equipment/maintenance/{id}`.

## Suggested approach
1. Determine the tenant/org scoping column on `equipment` (inspect the table migration and how `create_equipment` populates it — likely an `organization_id` / `building_id`). Confirm what `RequestPrincipal` exposes for org scope (mirror how `update_workflow` derives `org_id` in the same file).
2. Change `equipment_repo.delete`, `update`, and `update_maintenance` in `backend/crates/db/src/repositories/equipment.rs` to accept the caller's org scope and fold it into the `WHERE` clause, e.g. `DELETE FROM equipment WHERE id = $1 AND organization_id = $2`.
3. In the handlers (`delete_equipment`, `update_equipment`, `update_maintenance`), rename `_principal` to `principal`, derive the org scope the repository expects (same extraction the workflow handlers use), and pass it in.
4. Keep the existing `Ok(false) => 404` / not-found branches — with the scoped query, a non-owned or non-existent id returns `false`/zero-rows, so an attacker gets an indistinguishable `404`.
5. Audit the remaining equipment readers wired near `ai.rs:1038-1051` (`get_equipment`, `list_maintenance`, `create_maintenance`) for the same missing-scope pattern and fix any cross-tenant reads/writes within this PR's blast radius.

## Alternatives considered
- **Fetch-then-check in the handler (load equipment, compare org, then call unscoped delete)** — rejected: introduces a check-then-act race plus an extra round-trip; folding the predicate into the single statement is atomic and simpler.
- **Enforce via Postgres RLS only** — rejected as the sole fix: this repository method takes a plain `id` and the call site does not pass an RLS-scoped connection, so the explicit predicate is the reliable, testable change. RLS hardening can land separately under the existing RLS backlog.

## Root-cause trace
1. Symptom: user B's `DELETE …/equipment/{EQ_A}` returns `204` and deletes org 1's equipment.
2. ← immediate cause at `backend/servers/api-server/src/routes/ai.rs:1133` — `delete_equipment` ignores `_principal`, passing only `id` to the repository.
3. ← upstream cause at `backend/crates/db/src/repositories/equipment.rs:144` — `DELETE FROM equipment WHERE id = $1` filters on `id` alone, so the authorization decision was never expressed in the query.
4. Origin: the AI equipment/predictive-maintenance feature shipped the mutation paths without an ownership check; the `_principal` underscore binding shows the scope was scaffolded but never wired — identical to the voice-device path fixed in PR #461.

## Test plan
- [ ] Integration test in the api-server suite: user B (org 2) deleting user A's (org 1) equipment gets `404` and the row still exists (this fails on `dev` today — currently returns `204` and deletes the row).
- [ ] Same cross-tenant assertion for `PUT /equipment/{id}` (update) and `PUT /equipment/maintenance/{id}`.
- [ ] Positive regression: user A deleting/updating their own org's equipment still returns success.
- [ ] Run: `cargo test -p api-server --lib` (plus the new equipment integration test); confirm the cross-tenant test fails before the fix and passes after.

## Out of scope
- Broader RLS-connection migration for the `equipment` repository (tracked separately).
- Soft-delete vs hard-delete semantics for equipment.
- Audit logging / rate limiting of equipment-delete actions.
- The workflow and alert handlers (already org-scoped).

## After-merge
- Move this file to `plans/_archive/security-equipment-idor.md`
- Mark the matching `backlog.json` row as `status: "done"`
