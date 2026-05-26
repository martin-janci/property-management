# security-integrations-install-bola

**Vector:** security
**Score:** 5
**Source:** code-review api-handlers 2026-05-26 | install.rs:600 | PR #538 | PR #544
**Confidence:** high

## Hypothesis
The Airbnb (and adjacent Booking) channel-integration handlers in `routes/integrations/install.rs` accept an authenticated `AuthUser` plus a path `org_id` but never verify that `auth.tenant_id == path.org_id`, so any authenticated user can connect, disconnect, sync, or read another organization's channel integration by editing the URL org id (BOLA / cross-tenant write). The smallest fix is to add the same tenant guard already used elsewhere in the file (`if auth.tenant_id != Some(path.org_id) && !auth.is_platform_admin() { return 403 }`) to every handler in the 340–833 range, matching the pattern the Booking-channel and `sync.rs` siblings already enforce.

## Evidence
- `backend/servers/api-server/src/routes/integrations/install.rs:600` — `disconnect_airbnb(State, auth: AuthUser, Path<OrgIdPath>)` calls `rental_repo.revoke_airbnb_connection(path.org_id)` at :647 with no `auth.tenant_id`/org check; the destructive revoke runs on a caller-supplied org id.
- `backend/servers/api-server/src/routes/integrations/install.rs:340-833` — grep for `auth.tenant_id != Some | verify_org_access | is_platform_admin` over this range returns **0** matches; the same range holds `status_airbnb` (~349), `connect_airbnb` (421), `sync_airbnb` (479), `disconnect_airbnb` (600) and the booking connect/status handlers (684/746/830), all taking `Path<OrgIdPath>`.
- `backend/servers/api-server/src/routes/integrations/install.rs:1128` — a correct guard already exists in the same file: `if auth.tenant_id != Some(path.org_id) && !auth.is_platform_admin()` (repeated ~:1292/:1677), proving the omission on the Airbnb handlers is accidental, not by design.
- `backend/servers/api-server/src/routes/integrations/booking_channel.rs:181,412` — the sibling Booking-channel module guards every handler; `routes/integrations/sync.rs` uses `verify_org_access()` per handler — the in-file inconsistency confirms the gap.
- Found by rotating expert review (api-handlers segment, rust + security lenses) 2026-05-26; handlers are live (Airbnb backend wired by PR #538, Booking OTA by PR #544).

## Files
- `backend/servers/api-server/src/routes/integrations/install.rs:600`
- `backend/servers/api-server/src/routes/integrations/sync.rs`
- `backend/crates/db/src/repositories/rental.rs`

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [x] C2 — Seed data
- [x] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Seed two organizations A and B, each with an authenticated manager user (user_A in org A, user_B in org B), and create an Airbnb connection for org A.
2. As user_B, send `DELETE /api/v1/organizations/{org_A_id}/integrations/airbnb` (the `disconnect_airbnb` route) using user_B's valid token but org A's id in the path.
3. Expected: `403 Forbidden` (or `404` to avoid existence disclosure). Actual on `main`: `200`/`204` — org A's Airbnb connection is revoked by a user from org B (cross-tenant write).

## Suggested approach
1. Read the guard at `install.rs:1128` and confirm `AuthUser` exposes `tenant_id: Option<Uuid>` and `is_platform_admin()` (it does — used there).
2. Add the guard `if auth.tenant_id != Some(path.org_id) && !auth.is_platform_admin() { return Err((StatusCode::FORBIDDEN, Json(ErrorResponse::new("FORBIDDEN", "...")))); }` as the first statement of each unguarded handler in lines 340–833: `status_airbnb`, `connect_airbnb` (421), `sync_airbnb` (479), `disconnect_airbnb` (600), and the booking connect/status handlers (684/746/830).
3. Prefer extracting a small private helper `fn ensure_org_access(auth: &AuthUser, org_id: Uuid) -> Result<(), (StatusCode, Json<ErrorResponse>)>` in this module (or reuse `sync.rs`'s `verify_org_access` if it is `pub(crate)`) and call it from each handler, so the check cannot drift again.
4. Run the existing `backend/scripts/lints/check-discarded-principal.sh` lint to confirm no remaining auth-discard pattern.
5. Add the integration regression test (see Test plan).
6. Run `cargo sqlx prepare` if any query text changed (it shouldn't — no SQL change) and `cargo test`.

## Alternatives considered
- **RLS-only reliance (no handler guard)** — rejected because these handlers call `rental_repo` methods (`revoke_airbnb_connection(org_id)`, `find_airbnb_connection_by_org(org_id)`) that take the org id as a parameter rather than running through a tenant-scoped RLS connection; RLS does not constrain a query that explicitly selects by caller-supplied `org_id`.
- **Middleware-level org check on the whole `integrations` router** — rejected because the router mixes org-scoped and platform-admin routes and some handlers already carry the guard at :1128; a blanket middleware would either double-guard or wrongly block the platform-admin paths. Per-handler (or shared helper) is the surgical, low-blast-radius fix matching the file's existing convention.

## Root-cause trace
1. Symptom: a user authenticated to org B can revoke org A's Airbnb connection via `DELETE .../{org_A}/integrations/airbnb`.
2. ← `install.rs:647` `revoke_airbnb_connection(path.org_id)` runs on the path-supplied org id with no preceding authorization check.
3. ← `install.rs:600-606` `disconnect_airbnb` binds `auth: AuthUser` and `Path<OrgIdPath>` but omits the `auth.tenant_id != Some(path.org_id)` guard that the same file applies at :1128.
4. Origin: the Airbnb channel handlers added in PR #538 (and Booking OTA in PR #544) were written without the tenant guard that the older handlers in the file carry; the inconsistency was not caught because the guarded and unguarded handlers live in the same large module.

## Test plan
- [ ] `backend/servers/api-server/tests/integration/airbnb_connection_idor_tests.rs` — new: `user_b_cannot_disconnect_org_a_airbnb` (expect 403/404), modeled on `backend/servers/api-server/tests/dispute_cross_org_idor_tests.rs` and `backend/crates/db/tests/inquiry_mark_read_idor_tests.rs`.
- [ ] Regression: a same-org user (or platform admin) still gets `200`/`204` for connect/disconnect/sync/status — guard must not break the happy path.
- [ ] `cargo test -p api-server --test integration airbnb_connection_idor` (and the existing `check-discarded-principal.sh` lint) pass after the change; the IDOR test fails on `main` before the fix.

## Out of scope
- The Booking-channel handlers in `routes/integrations/booking_channel.rs` (already guarded) — no change needed there.
- The debug-level logging of the OAuth `state` token at `install.rs:456` — tracked separately as `security-install-oauth-state-debug-log`.
- Any module-split / refactor of `install.rs` — orthogonal churn concern.

## After-merge
- Move this file to `plans/_archive/security-integrations-install-bola.md`
- Mark the matching `backlog.json` row as `status: "done"`
