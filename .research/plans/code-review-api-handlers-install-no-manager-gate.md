# code-review-api-handlers-install-no-manager-gate

**Vector:** security
**Score:** 3
**Source:** code-review:api-handlers/integrations/install.rs
**Confidence:** high

## Hypothesis
The Airbnb / Booking.com / portal-connection install handlers in `integrations/install.rs` only verify org membership (`verify_org_access`) and never call the sibling `verify_manager_role_in_org` gate that `integrations/sync.rs` documents as required for OTA mutations (`SECURITY #1525`). Any regular org member — tenant, viewer, basic operator — can therefore wire or unwire the organization's OAuth credentials, deleting an entire active integration, replacing OAuth secrets, or starting a connect handshake against a malicious redirect. The fix is to call `verify_manager_role_in_org` on every mutating handler in `install.rs` immediately after the existing `verify_org_access`.

## Evidence
- `backend/servers/api-server/src/routes/integrations/install.rs:431,507,575,756,840,907,1019,1247,1616,1650,1763` — `connect_airbnb`, `disconnect_airbnb`, `direct_connect_airbnb`, `connect_booking`, `disconnect_booking`, `create_portal_connection`, `delete_portal_connection` all call `verify_org_access(&state, auth.user_id, path.org_id).await?` and stop there.
- `backend/servers/api-server/src/routes/integrations/sync.rs:155-197` — `pub(super) async fn verify_manager_role_in_org` exists, derives `is_manager` from `TenantRole::is_manager`, returns `403 Manager-level access required` when the caller is not a manager. Docs above the function explicitly warn about the cross-org-via-A-role pattern (`SECURITY #1525`).
- `backend/servers/api-server/src/routes/integrations/mod.rs:34` — `install::router` is mounted under `/api/v1/integrations/organizations/{org_id}/{airbnb,booking,portals}/...`, so this is the production install/uninstall surface for OTA + portal integrations consumed by `ppt-web` Settings → Integrations.
- `backend/servers/api-server/src/routes/integrations/sync.rs:161` — gate is `pub(super)` so `install.rs` (same parent module) can call it directly as `super::sync::verify_manager_role_in_org(&state, auth.user_id, path.org_id).await?`.
- Phase 1.5 (api-handlers) review on 2026-06-25.

## Files
- `backend/servers/api-server/src/routes/integrations/install.rs`
- `backend/servers/api-server/src/routes/integrations/sync.rs`
- `backend/servers/api-server/tests/oauth_integration_tests.rs`

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
1. Seed an org with two memberships: `manager@org` (role_type=manager) and `tenant@org` (role_type=tenant_resident).
2. Sign in as `tenant@org`, mint a JWT with this user_id.
3. `curl -X POST /api/v1/integrations/organizations/{org_id}/airbnb/disconnect` with the tenant's Bearer JWT in the `Authorization` header.
4. **Expected:** `403 FORBIDDEN` (`Manager-level access required for this organization`). **Actual today:** `200 OK` — Airbnb integration is wiped.

## Suggested approach
1. In `install.rs`, immediately after every `verify_org_access(...)` call inside a *mutating* handler (`connect_airbnb`, `disconnect_airbnb`, `direct_connect_airbnb`, `connect_booking`, `disconnect_booking`, `create_portal_connection`, `delete_portal_connection`, plus any other POST/DELETE handler in the file the reviewer surfaces), append `super::sync::verify_manager_role_in_org(&state, auth.user_id, path.org_id).await?;`.
2. Leave read-only handlers (`get_airbnb_status` :420, `get_booking_status`, listing-fetch endpoints) untouched — membership is sufficient for read.
3. If `verify_manager_role_in_org` is `pub(super)` and `install.rs` is a sibling of `sync.rs` under the same parent (verified — both are children of `routes/integrations/`), the call compiles as `super::sync::verify_manager_role_in_org`. If a future split moves the gate, re-export from `routes/integrations/mod.rs` (`pub use sync::verify_manager_role_in_org;`).
4. Add `backend/servers/api-server/tests/integrations_install_manager_gate_tests.rs`. For each mutating handler, two scenarios: `tenant_resident → 403`, `manager → 200/204`. Seed two memberships in a transaction; reuse the existing OAuth-test fixtures from `oauth_integration_tests.rs`.
5. Confirm CI green: `cd backend && cargo test -p api-server --test integrations_install_manager_gate_tests`.
6. No frontend change — `ppt-web` already hides the Connect/Disconnect controls behind the manager-role guard; this hardens the backend defense-in-depth layer.

## Alternatives considered
- **Add the gate only to `disconnect_*`** — rejected because `connect_*` accepts a `redirect_uri` in the request body (`install.rs:512`), so a non-manager could redirect an OAuth handshake through their own host and capture the org's OAuth-grant code. The connect path is just as dangerous as the disconnect path.
- **Wrap all `install.rs` routes in a tower `RequireManagerLayer`** — rejected because read-only `get_*_status` handlers must remain accessible to any member; a blanket layer would break the Settings → Integrations status display for non-managers.

## Root-cause trace
1. Symptom: a `tenant_resident`-role user can call `POST /integrations/organizations/{org}/airbnb/disconnect` and receive `200 OK`, severing the org's Airbnb integration.
2. ← `install.rs:431 / 507 / 575 / 756 / 840 / 907 / 1019 / 1247 / 1616 / 1650 / 1763` — only `verify_org_access` runs; no manager check.
3. ← `verify_org_access` (membership-only) was the only gate authored when these handlers first landed; `verify_manager_role_in_org` was added later (`integrations/sync.rs:161`) specifically to close `SECURITY #1525` on OTA *mutations* but the same gate was never back-applied to the *install* surface.
4. Origin: pre-dates the recent PR #1798 / `integrations/install.rs` extraction — the gap exists in every revision since the original install handler set was authored. The recent reshuffles preserved the gap without surfacing it.

## Test plan
- [ ] `backend/servers/api-server/tests/integrations_install_manager_gate_tests.rs::tenant_cannot_disconnect_airbnb` — seed tenant+manager; tenant `disconnect_airbnb` → 403; manager → 200.
- [ ] `tenant_cannot_connect_booking` — tenant `POST /booking/connect` → 403; manager → 200 (OAuth URL response).
- [ ] `tenant_cannot_delete_portal_connection` — tenant DELETE → 403; manager → 204.
- [ ] `manager_can_still_disconnect_airbnb` — sanity that the manager path still works (regression check on the gate, not a bypass).
- [ ] Command: `cd backend && cargo test -p api-server --test integrations_install_manager_gate_tests` from the repo root.

## Out of scope
- Read-only handlers in `install.rs` (`get_airbnb_status`, `get_booking_status`, listing-fetch) — membership is the correct gate, do not add manager-role to those.
- Any change to `verify_manager_role_in_org` itself (the helper is canonical and already covered by `oauth_integration_tests`).
- Migrating `install.rs` to use `RlsConnection` (separate refactor, tracked elsewhere).
- Frontend changes — `ppt-web` already gates the Connect/Disconnect UI on manager role.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-install-no-manager-gate.md`
- Mark the matching `backlog.json` row as `status: "done"`
