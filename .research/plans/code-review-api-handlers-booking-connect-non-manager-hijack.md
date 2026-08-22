# code-review-api-handlers-booking-connect-non-manager-hijack

**Vector:** security
**Score:** 3
**Source:** api-handlers segment review 2026-08-22 (Phase 1.5 code-review slice); install.rs:907
**Confidence:** high

## Hypothesis
`connect_booking` (POST `/api/v1/integrations/organizations/{org_id}/booking/connect`) enforces only `verify_org_access`, so any authenticated org member — including a plain resident with no admin role — can POST attacker-controlled Booking.com credentials and have them persisted as the organization's active OTA integration. The sibling OAuth-flow writer `booking_token_exchange` at `integrations/oauth.rs:140` and the direct-connect `direct_connect_airbnb` at `install.rs:1868` both explicitly require `verify_manager_role_in_org`; the direct-connect Booking path in `install.rs` was skipped. Once the attacker's creds are stored, subsequent `push_booking_availability` / `push_booking_rates` runs push the organization's real property data into the attacker's Booking.com account, and `disconnect_booking` (also plain `verify_org_access`) lets any member wipe the legitimate connection.

## Evidence
- `backend/servers/api-server/src/routes/integrations/install.rs:907` — `connect_booking` calls only `verify_org_access(&state, auth.user_id, path.org_id).await?;` then persists caller-supplied hotel_id/username/password via `create_or_update_booking_connection`.
- `backend/servers/api-server/src/routes/integrations/oauth.rs:140` — `booking_token_exchange` gates on `verify_manager_role_in_org` before writing tokens (the OAuth path enforces the invariant; the direct-connect path did not).
- `backend/servers/api-server/src/routes/integrations/install.rs:1868` — `direct_connect_airbnb` gates on `verify_manager_role_in_org` before storing Airbnb credentials — same class of write, different integration, correctly gated.
- `backend/servers/api-server/src/routes/integrations/install.rs:1247` — `disconnect_booking` has the same missing-gate pattern; `install.rs:756` `disconnect_airbnb` too. `oauth.rs:773` (`airbnb_oauth_callback`) mirrors the gap.
- Blast radius: `push_booking_availability` / `push_booking_rates` (`install.rs:1318` / `:1466`) send the org's real property availability + rates to whatever credentials are stored — attacker-controlled OTA creds → data exfiltration + integration hijack (attacker becomes the org's outward-facing Booking.com channel).

## Files
- `backend/servers/api-server/src/routes/integrations/install.rs`
- `backend/servers/api-server/src/routes/integrations/oauth.rs`
- `backend/servers/api-server/src/routes/integrations/sync.rs`

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Seed an organization with a Manager user (who legitimately calls `POST .../booking/connect` with valid hotel_id / user / password — the org's Booking connection is stored).
2. Seed a plain-member user in the same org (`TenantRole::Member` — the resident role; NOT Manager / OrgAdmin / PlatformAdmin / SuperAdmin).
3. Authenticate as the plain member and POST `/api/v1/integrations/organizations/{org_id}/booking/connect` with a distinct `{ hotel_id, username, password }` payload (attacker-controlled OTA account).
4. Expected: `403 FORBIDDEN` with `code = "FORBIDDEN"`, legitimate connection untouched.
5. Actual (today): `200 OK`; `rental_repo.get_booking_connection(org_id)` returns the attacker's `external_property_id` / encrypted username / encrypted password — the manager's credentials were silently overwritten. `DELETE /api/v1/integrations/organizations/{org_id}/booking` from the same plain member likewise succeeds — the legitimate connection is wiped.

## Suggested approach
1. In `backend/servers/api-server/src/routes/integrations/install.rs`, add `super::sync::verify_manager_role_in_org(&state, auth.user_id, path.org_id).await?;` immediately after the existing `verify_org_access` call in **both** `connect_booking` (line 907) and `disconnect_booking` (line 1247). Mirror the pattern already used by `direct_connect_airbnb` (line 1868).
2. Close the same gap on the Airbnb side of `install.rs`: `disconnect_airbnb` (line 756) needs the manager gate too.
3. In `backend/servers/api-server/src/routes/integrations/oauth.rs`, add `verify_manager_role_in_org` after `verify_org_access` in `airbnb_oauth_callback` (line 773) — the OAuth callback binds tokens to the org and must be manager-only, matching `booking_token_exchange` (line 140) and `airbnb_token_exchange` (line 310).
4. Do not touch `list_*` / `get_*` reads (verify_org_access remains correct for reads). Do not touch the sync router (unmounted per `integrations/mod.rs:29-36`).
5. Confirm no other handlers in `install.rs` / `oauth.rs` mutate credentials while gating only on `verify_org_access` — audit the 12 `verify_org_access` call sites once, note the read-only ones, and gate the remaining write-side ones (there should be no additional writers left after steps 1–3).

## Alternatives considered
- **Enforce at the repo layer (`RentalRepo::create_or_update_booking_connection` refuses to write unless the caller supplies a manager-role token)** — rejected because the repo has no principal context; role checks belong in the handler / router layer where the auth extractor lives. Threading roles into every repo call multiplies surface area for no defense in depth beyond a duplicate check.
- **Introduce a shared `verify_org_manager_write` extractor / middleware for the whole integrations router** — rejected as scope creep for this hotfix. Worth doing once the fix lands (existing per-handler pattern is what the sibling gated writers use), but shipping the extractor here would balloon the diff and delay the security patch.

## Root-cause trace
1. Symptom: authenticated plain org member can overwrite the org's Booking.com integration credentials via `POST /api/v1/integrations/organizations/{org_id}/booking/connect`.
2. ← Immediate cause: `backend/servers/api-server/src/routes/integrations/install.rs:907` — `connect_booking` calls `verify_org_access` only; `verify_org_access` returns success for any org member, regardless of role.
3. ← Upstream cause: when the direct-connect Booking path was authored, the manager gate applied to `direct_connect_airbnb` (`install.rs:1868`) and to the OAuth-flow `booking_token_exchange` (`oauth.rs:140`) was not mirrored on this handler. `verify_manager_role_in_org` exists (`integrations/sync.rs:161`) and is imported in the sibling files; `install.rs` never imports it.
4. Origin: introduced when the Booking.com direct-connect path was split into `install.rs` (Issue #765's initial pass added the `verify_org_access` line as a comment-annotated IDOR fix but did not upgrade to the manager gate that later landed for the Airbnb sibling).

## Test plan
- [ ] Add `backend/servers/api-server/tests/suites/integrations_connect_booking_authz_tests.rs` (or extend the closest existing suites file — the repo already carries `integrations_*` tests via `tests/suites/`): seed an org with a Manager + a plain-member user; from the plain member, `POST .../booking/connect` with a distinct payload — expect `403 FORBIDDEN` (code `FORBIDDEN`); assert `rental_repo.get_booking_connection(org_id)` still returns the Manager's original `external_property_id`.
- [ ] Symmetric test for `DELETE .../booking` — plain member cannot wipe the connection.
- [ ] Symmetric test for the Airbnb siblings: plain member cannot POST `disconnect_airbnb` or complete `airbnb_oauth_callback` after the fix.
- [ ] Regression: existing Manager-role happy path (`create_or_update_booking_connection`) still returns 200 and stores the credentials.
- [ ] Command: `cd backend && cargo test -p api-server --test integrations_connect_booking_authz_tests -- --nocapture` (or the equivalent `--test suites` runner name once the file lands under `tests/suites/`).

## Out of scope
- Refactoring `verify_org_access` / `verify_manager_role_in_org` into shared middleware — worth doing later, out of scope for this security hotfix.
- Remounting the unmounted `sync` router (PAP-122 backlog item); its handlers have their own gaps but are not currently reachable.
- Auditing non-integration handlers that use `verify_org_access` alone — a separate sweep, tracked as follow-up if signal warrants.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-booking-connect-non-manager-hijack.md`
- Mark the matching `backlog.json` row as `status: "done"` (dispatcher reconciler usually does this automatically once the PR merges)
