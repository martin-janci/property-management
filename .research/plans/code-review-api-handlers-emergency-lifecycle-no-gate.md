# code-review-api-handlers-emergency-lifecycle-no-gate

**Vector:** security
**Score:** 2
**Source:** code-review:api-handlers/emergency/incidents.rs
**Confidence:** high

## Hypothesis
After the PR #1798 `emergency.rs` → `emergency/` split, three lifecycle handlers — `update_incident` (`incidents.rs:156`), `resolve_incident` (`incidents.rs:235`), `close_incident` (`incidents.rs:266`) — lost the `is_emergency_manager(rls.role())` gate that `create_incident` (`incidents.rs:49`), `acknowledge_incident` (`incidents.rs:193`), and sibling `broadcasts.rs::deactivate_broadcast` (`broadcasts.rs:134`) all still hold. Any authenticated org member can now update incident metadata, resolve an incident with a free-text `resolution` string, or close it — actions that managers must still authorize for the create/acknowledge half of the same lifecycle. This is an authorization inconsistency that almost certainly regressed during the split; restoring the gate is a 3-line addition per handler.

## Evidence
- `backend/servers/api-server/src/routes/emergency/incidents.rs:156-184` — `update_incident` body has no `is_emergency_manager` check before calling `state.emergency_repo.update_incident(...)`.
- `backend/servers/api-server/src/routes/emergency/incidents.rs:235-265` — `resolve_incident` body has no gate before `state.emergency_repo.resolve_incident(...)`.
- `backend/servers/api-server/src/routes/emergency/incidents.rs:266-289` — `close_incident` body has no gate before `state.emergency_repo.close_incident(...)`.
- Comparison: `create_incident` (:49) and `acknowledge_incident` (:193) both open with `if !is_emergency_manager(rls.role()) { rls.release().await; return 403 … }`. `broadcasts.rs::deactivate_broadcast` (:134) follows the same pattern.
- `is_emergency_manager` is re-exported from `emergency/mod.rs:18` and imported by `incidents.rs:18`, so adding the call is a single-line gate per handler.
- PR #1798 (`refactor(api-server): split emergency routes into cohesive sub-modules`) merged 2026-06-24 — the most plausible regression vector.

## Files
- `backend/servers/api-server/src/routes/emergency/incidents.rs`
- `backend/servers/api-server/src/routes/emergency/broadcasts.rs`
- `backend/servers/api-server/src/routes/emergency/mod.rs`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Seed an org with two memberships: `manager@org` (emergency_manager-eligible role) and `tenant@org` (basic resident).
2. As `manager@org`, `POST /api/v1/emergency/incidents` → `201 Created` (id=I).
3. As `tenant@org`, `PATCH /api/v1/emergency/incidents/{I}` with new title/severity → **expected** `403`; **actual today** `200`.
4. As `tenant@org`, `POST /api/v1/emergency/incidents/{I}/resolve` with `{"resolution":"closed by tenant"}` → **expected** `403`; **actual today** `200`.
5. As `tenant@org`, `POST /api/v1/emergency/incidents/{I}/close` → **expected** `403`; **actual today** `200`.

## Suggested approach
1. In `incidents.rs::update_incident`, insert immediately after `let org = rls.tenant_id();`:
   ```rust
   if !is_emergency_manager(rls.role()) {
       rls.release().await;
       return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("FORBIDDEN", "Manager role required for this action"))).into_response();
   }
   ```
   Match the formatting of the existing gate in `create_incident:49` byte-for-byte (same error code, same string) so the test harness can assert on `code` not body shape.
2. Repeat for `resolve_incident:235` and `close_incident:266`. Each insertion is right after the `let org = rls.tenant_id();` line (and `let user = rls.user_id();` where present) and before the repo call.
3. Add `backend/servers/api-server/tests/emergency_incident_lifecycle_gate_tests.rs`. Four scenarios: `tenant_cannot_update`, `tenant_cannot_resolve`, `tenant_cannot_close`, plus one positive sanity (`manager_can_resolve_and_close`).
4. Run `cd backend && cargo test -p api-server --test emergency_incident_lifecycle_gate_tests`.
5. No frontend change — ppt-web Emergency screens already hide these actions for non-managers; this restores the backend defense-in-depth.

## Alternatives considered
- **Add the gate only to `resolve` + `close`** — rejected because `update_incident` can change `severity` / `assigned_team` / `title`, which has the same authorization weight as the lifecycle transitions and is just as authoritative.
- **Use a tower layer over the whole `incidents` router** — rejected because `list_incidents` and `get_incident` are intentionally accessible to any member of the org (read), so a blanket layer would break the read path. Per-handler gates mirror the established pattern in `broadcasts.rs` and elsewhere in `emergency/`.

## Root-cause trace
1. Symptom: a `tenant_resident`-role user can call `PATCH /api/v1/emergency/incidents/{id}` and get `200 OK` with the patch applied (likewise `/resolve` and `/close`).
2. ← `emergency/incidents.rs:156 / 235 / 266` — handlers omit the `is_emergency_manager` early-return that their `create_incident` / `acknowledge_incident` siblings hold (lines 49, 193).
3. ← The split landed in PR #1798 reorganized the previous `emergency.rs` monolith into `emergency/{incidents,broadcasts,protocols,mod}.rs`. The split copied handler bodies but, for the update/resolve/close trio, the manager-check guard was dropped — either intentionally (mis-judging the operation as data-edit not lifecycle) or by accident on a code move.
4. Origin: PR #1798 (`refactor(api-server): split emergency routes into cohesive sub-modules`), merged 2026-06-24. Pre-split `routes/emergency.rs` had the gates; confirm via `git log -p -- backend/servers/api-server/src/routes/emergency.rs` on the commit just before the split.

## Test plan
- [ ] `backend/servers/api-server/tests/emergency_incident_lifecycle_gate_tests.rs::tenant_cannot_update_incident` — manager creates incident I; tenant PATCH → 403; manager PATCH → 200.
- [ ] `tenant_cannot_resolve_incident` — tenant POST /resolve → 403; manager → 200.
- [ ] `tenant_cannot_close_incident` — tenant POST /close → 403; manager → 200.
- [ ] `manager_can_resolve_and_close` — positive control: manager full lifecycle succeeds end-to-end.
- [ ] Command: `cd backend && cargo test -p api-server --test emergency_incident_lifecycle_gate_tests` from repo root.

## Out of scope
- Read handlers (`list_incidents`, `get_incident`) — membership is correct, do not gate.
- `broadcasts.rs` and `protocols.rs` — those handlers already hold the gate; do not retouch.
- Audit-log emission on lifecycle changes (separate concern; the existing implementation already writes via the repo).
- Migrating to a centralized `RequireManagerLayer` (deferred; per-handler gates are the established convention).

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-emergency-lifecycle-no-gate.md`
- Mark the matching `backlog.json` row as `status: "done"`
