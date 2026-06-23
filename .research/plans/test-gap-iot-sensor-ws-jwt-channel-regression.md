# test-gap-iot-sensor-ws-jwt-channel-regression

**Vector:** security
**Score:** 2
**Source:** PR #1737 (`fix(iot): remove duplicate JWT-trusting sensor WS channel, converge on DB-checked handler (#1668)`)
**Confidence:** high

## Hypothesis

PR #1737 removed a duplicate sensor WebSocket channel (`GET /api/v1/iot/ws` in deleted `ws_sensor.rs`) that trusted the JWT `tenant_id` claim with **no DB membership check**, converging traffic on the surviving DB-checked handler (`GET /api/v1/iot/sensors/ws` in `routes/iot.rs`). The fix landed without a regression test that pins the JWT-only path is gone — if a future refactor re-introduces a similar bypass (or restores the deleted file from history), no failing test would catch it. Add a focused handler-level regression that asserts (a) the deleted route no longer mounts, and (b) the surviving handler rejects a request whose JWT claims membership in an org the DB does not confirm.

## Evidence

- PR #1737 body documents the bypass: "trusts JWT `tenant_id` claim, **no DB check**" — explicit security gap closed by deletion.
- Deleted file: `backend/servers/api-server/src/routes/ws_sensor.rs` (full handler removed).
- Surviving handler: `backend/servers/api-server/src/routes/iot.rs::sensor_ws_handler` — uses `OrganizationMemberRepository::is_member` for DB-backed authz.
- Cross-product impact: `frontend/apps/ppt-web/src/features/iot/hooks/useIotWebSocket.ts` and `frontend/apps/ppt-web/src/routes/groups/iot.tsx` were updated to point at the surviving endpoint — clients migrated, but server-side guard against re-introducing the bypass is the missing piece.
- No new test file in the PR diff matching `*iot*ws*` or `*sensor*ws*`.

## Files

- `backend/servers/api-server/src/routes/iot.rs`
- `backend/servers/api-server/src/routes/mod.rs`
- `backend/servers/api-server/Cargo.toml`

## Dependencies

(none — PR #1737 already merged; this is post-merge test coverage)

## Required capabilities

- [x] C1 — Systematic debugging (security regression, root-cause-traced via PR body)
- [ ] C2 — Seed data (test creates org + non-member user inline via fixtures)
- [ ] C3 — Dev instance running (handler-level test, no live stack)
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. Check out current `dev`. Confirm `backend/servers/api-server/src/routes/ws_sensor.rs` does not exist (`git ls-files | grep ws_sensor` returns empty).
2. Run `cargo test -p api-server iot_sensor_ws_jwt_bypass_regression` — expect: **no such test** (failing-on-main / IG3 condition).
3. Manually verify the surviving handler reads `OrganizationMemberRepository::is_member` before opening the WS — open `routes/iot.rs::sensor_ws_handler` and grep for `is_member`. Expect: present.
4. The regression test added by this plan should: (a) attempt to mount the deleted route → assert 404, (b) call the surviving handler with a JWT that claims `tenant_id = org_X` while the DB has no row in `organization_members(user_id, organization_id = org_X)` → assert 403/Close-with-policy-violation.

## Suggested approach

1. Add `backend/servers/api-server/tests/iot_sensor_ws_jwt_bypass_regression.rs`. Use the existing `#[sqlx::test]` pattern (mirror `backend/servers/api-server/tests/fault_notification_recipient_tests.rs` for fixture style).
2. **Test 1: deleted route is gone.** Make a WS upgrade request to `/api/v1/iot/ws` against the test app. Assert response is `404 Not Found` (the route is no longer mounted in `routes/mod.rs`).
3. **Test 2: surviving handler enforces DB membership.** Build a test JWT for `user_alice` that *claims* `tenant_id = org_X`, but do NOT insert into `organization_members` for that pair. Issue a WS upgrade to `/api/v1/iot/sensors/ws?organization_id=org_X`. Assert: handler closes with policy-violation OR responds 403 before upgrade — whichever the current handler does. Pin the chosen behaviour.
4. **Test 3 (defense in depth): surviving handler accepts true members.** Insert a real membership row for `user_bob ∈ org_Y`. Issue the same WS upgrade with `user_bob`'s JWT and `organization_id = org_Y`. Assert: upgrade succeeds (101). Send one `sensor.reading.created` event via Redis pub/sub on `sensors:org_Y`; assert the client receives it.
5. Wire all three into the existing `tests/common/mod.rs` test-app builder. Re-run `cargo test -p api-server` locally before pushing.
6. Verify clippy + fmt pass: `cargo fmt --all -- --check && cargo clippy -p api-server -- -D warnings`.
7. PR title: `test(api-server): pin sensor-ws JWT-bypass regression after #1737`. Reference #1737 and this plan slug in the body.

## Alternatives considered

- **Add a service-level test on `OrganizationMemberRepository::is_member` only** — rejected because the bypass was at the *handler* layer (skipping the repo entirely). A repo-level test wouldn't catch a re-introduction of a JWT-only-trusting handler that simply doesn't call the repo.
- **Add a compile-time deny lint (`#[forbid(jwt_only_authz)]`)** — rejected because there's no existing lint infrastructure for this domain semantic, and authoring a custom clippy lint for a single occurrence is excessive scope for a test-gap plan.

## Root-cause trace

1. Symptom: PR #1737 deletes a security-relevant handler; deletion alone is fragile — a revert would silently re-open the bypass.
2. ← Origin handler at deleted `backend/servers/api-server/src/routes/ws_sensor.rs` — trusted JWT `tenant_id` claim without DB membership check.
3. ← Upstream: Story 14.3 sensor-WS work duplicated by two PRs (#1640 and #1644) that landed on the same day, each subscribing to the same Redis channel with different event names. The two-PR race itself is tracked at #1668.
4. Origin: PR #1640 (Story 14.3 sensor WS, first commit). PR #1644 added the DB-checked variant; PR #1737 removed the bypass.

## Test plan

- [ ] `backend/servers/api-server/tests/iot_sensor_ws_jwt_bypass_regression.rs` — three `#[sqlx::test]` cases as described above.
- [ ] Regression scenario: revert #1737 in a local branch, re-run the test — Test 1 fails (route is back). Test 2 fails (JWT-only path accepted). Confirms the test would have caught the bypass.
- [ ] Run locally: `cd backend && cargo test -p api-server --test iot_sensor_ws_jwt_bypass_regression`

## Out of scope

- Rewriting `OrganizationMemberRepository::is_member` (the existing impl is fine — only need to *pin* that handlers call it).
- Adding a generic "every WS route checks DB membership" framework lint — see *Alternatives*.
- Test coverage for the original event-name divergence (`sensor.reading` vs `sensor.reading.created`) — that's #1668's own follow-up scope, already merged via the wire-format reconciliation in #1644.

## After-merge

- Move this file to `plans/_archive/test-gap-iot-sensor-ws-jwt-channel-regression.md`
- Mark the matching `backlog.json` row (`test-gap-hotfix-no-test-pr-1737-iot-jwt-trusting-channel`) as `status: "done"`
