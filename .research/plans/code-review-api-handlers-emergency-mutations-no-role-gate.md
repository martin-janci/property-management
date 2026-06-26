# code-review-api-handlers-emergency-mutations-no-role-gate

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 rotating expert review of `api-handlers` segment (2026-06-26)
**Confidence:** medium

## Hypothesis

Mutating endpoints under `backend/servers/api-server/src/routes/emergency/{protocols,contacts,drills}.rs` accept create/update/delete from any authenticated org member, skipping the `is_emergency_manager(rls.role())` role-gate that sibling modules `broadcasts.rs` (line 40) and `incidents.rs` (line 49) already enforce. The pattern is consistent across all three files: they call shared helpers and rely on RLS for org isolation but never check the caller's role inside the org, so any tenant member can author/overwrite/delete emergency protocols, contacts, or drills. The fix is to add the same one-line role gate at the top of each mutating handler — no schema or repo changes required.

## Evidence

- `backend/servers/api-server/src/routes/emergency/protocols.rs:28` — `create_protocol` has no role check
- `backend/servers/api-server/src/routes/emergency/protocols.rs:107` — `update_protocol` allows any org member to overwrite
- `backend/servers/api-server/src/routes/emergency/protocols.rs:137` — `delete_protocol` allows any org member to wipe
- `backend/servers/api-server/src/routes/emergency/contacts.rs:28` + `drills.rs:35` — `create_*/update_*/delete_*` similarly skip the gate
- Sibling `backend/servers/api-server/src/routes/emergency/broadcasts.rs:40` shows the intended pattern: `if !is_emergency_manager(rls.role()) { return Err(...) }`

## Files

- `backend/servers/api-server/src/routes/emergency/protocols.rs`
- `backend/servers/api-server/src/routes/emergency/contacts.rs`
- `backend/servers/api-server/src/routes/emergency/drills.rs`
- `backend/servers/api-server/src/routes/emergency/shared.rs`
- `backend/servers/api-server/src/routes/emergency/broadcasts.rs`

## Dependencies

## Required capabilities

- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. Start the api-server: `cargo run -p api-server` (or use bridge-MCP `ppt_dev_up` if cloud-only).
2. As a non-emergency-manager org member (regular tenant role), call `POST /api/v1/emergency/protocols` with a valid `CreateProtocolRequest` body.
3. Expected: `403 Forbidden` with role-mismatch error.
4. Actual on `dev`: `201 Created` — the protocol is persisted; the caller can then `PATCH` and `DELETE` it via the same routes.

## Suggested approach

1. Open `backend/servers/api-server/src/routes/emergency/protocols.rs`. At the top of `create_protocol`, `update_protocol`, and `delete_protocol`, add the same guard used in `broadcasts.rs:40`:
   ```rust
   if !is_emergency_manager(rls.role()) {
       return Err(ApiError::forbidden("emergency-manager role required"));
   }
   ```
   (Use the existing error helper / status mapping — match `broadcasts.rs` exactly.)
2. Repeat for `create_contact`, `update_contact`, `delete_contact` in `contacts.rs`.
3. Repeat for `create_drill`, `update_drill`, `delete_drill` in `drills.rs`.
4. Confirm `is_emergency_manager` is already exported from `shared.rs` (it is — see `broadcasts.rs:16`); no new helper needed.
5. Write a regression test at `backend/servers/api-server/tests/emergency_role_gate_tests.rs`: seed two memberships in one org (manager + regular), assert manager gets 201/204 and regular gets 403 for each mutating endpoint (9 cases).
6. Run `cargo fmt --all && cargo clippy -p api-server -- -D warnings && cargo test -p api-server emergency_role_gate`.
7. Open the PR; include the cross-role 403 matrix in the description.

## Alternatives considered

- **Wrap the gate in an extractor (`EmergencyManager`)** — rejected because the existing sibling files use the inline `if !is_emergency_manager(...)` pattern; introducing a new extractor for 3 files would diverge from the consistent module-local style and force a separate refactor PR.
- **Use RLS policy on the `emergency_*` tables** — rejected because RLS already enforces *org* isolation; the missing layer is the *role-within-org* check, which is correctly an application-layer concern (RLS rows are scoped by org_id, not role).

## Root-cause trace

1. Symptom: regular org member can create/update/delete emergency protocols/contacts/drills.
2. ← Handler at `protocols.rs:28` (and siblings) extracts `rls: RlsConnection` but never inspects `rls.role()`.
3. ← The module was authored alongside `broadcasts.rs` / `incidents.rs` which DO check the role, but the role-gate idiom wasn't ported to `protocols.rs` / `contacts.rs` / `drills.rs` when the emergency module was split into a directory (pre-2026-06 module-split batch).
4. Origin: routes/emergency directory restructure (subdirectory creation predates the 2026-06-24 hotspot-split batch; the `is_emergency_manager` helper was introduced when broadcasts.rs split out, but other siblings were not retrofitted).

## Test plan

- [ ] `backend/servers/api-server/tests/emergency_role_gate_tests.rs` — new integration test, 9 cases (3 endpoints × 3 files), asserts 201/204 for emergency_manager and 403 for regular member.
- [ ] Existing `cargo test -p api-server` suite stays green.
- [ ] `cargo test -p api-server --test emergency_role_gate_tests` is the focused command.

## Out of scope

- Read endpoints (`list_*`, `get_*`) — those legitimately serve all org members and the route-level RLS already isolates by org; only mutating endpoints need the role gate.
- Refactoring `is_emergency_manager` into an extractor or layer-level middleware.
- The separate `code-review-api-handlers-ets-screening-results-no-role-gate` finding (same class of bug in `enhanced_tenant_screening/`) — handled by its own plan.

## After-merge

- Move this file to `plans/_archive/code-review-api-handlers-emergency-mutations-no-role-gate.md`
- Mark the matching `backlog.json` row as `status: "done"`
