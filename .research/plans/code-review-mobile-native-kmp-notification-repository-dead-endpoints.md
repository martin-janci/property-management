# code-review-mobile-native-kmp-notification-repository-dead-endpoints

**Vector:** bug
**Score:** 3
**Source:** mobile-native-kmp segment review 2026-08-23
**Confidence:** high

## Hypothesis
The KMP `NotificationRepository` targets `$baseUrl/api/v1/notifications*` on the Reality Portal
backend, but `backend/servers/reality-server/src/main.rs` (the router assembly at lines 475-519)
never nests a `/api/v1/notifications` sub-router — grepping the reality-server source for
`.route.*notifications` returns zero matches. Every KMP notification call (`getNotifications`,
`getUnreadCount`, `markAsRead`, `markAllAsRead`, `deleteNotification`, `registerPushToken`,
`unregisterPushToken`, `getPreferences`, `updatePreferences`) 404s against the real backend. Tests
pass only because the KMP `MockEngine`-driven suites do not verify the path against the actual
reality-server router. The smallest correct fix is to either (a) add a `/api/v1/notifications`
router to reality-server that matches the KMP surface, or (b) point `NotificationRepository` at the
api-server push-token/notification-preferences routes that actually exist and delete the
unreachable helpers.

## Evidence
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/notifications/NotificationRepository.kt:35` — `client.get("$baseUrl/api/v1/notifications")`
- `backend/servers/reality-server/src/main.rs:475-519` — router-nest block; enumerates listings, users, favorites, saved-searches, inquiries, sso, agencies, realtors, imports, my/listings, compare, reports, price-map, articles, layout — no `/api/v1/notifications` nest
- `grep -rn "\.route.*notifications\|nest.*notifications\|/notifications" backend/servers/reality-server/src` returns 0 matches
- Existing KMP tests under `mobile-native/shared/src/commonTest/kotlin/.../notifications/` drive `MockEngine` (no contract check against the real router)
- Sibling favorites-alerts push wiring in `backend/servers/reality-server/src/routes/favorites.rs` is the real subscription surface — the mobile push flow probably belongs there or on `api-server`, not on a phantom nest

## Files
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/notifications/NotificationRepository.kt`
- `backend/servers/reality-server/src/main.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Point a debug KMP build at a real reality-server instance (a `stack up pm-local` instance on `:8081`, or the ops team's shared dev host), signed in with a valid session token.
2. Trigger any notification path from the KMP shared code — e.g. `NotificationRepository(baseUrl, token).getUnreadCount()` from a Kotlin unit runner or the Android app's notifications tab.
3. Expected: `Result.success(0)` or an `unread_count` payload. Actual: the response is HTTP 404, the `Result` fails with `NotificationException("Failed to load ... 404 Not Found")`, and the Android/iOS notifications UI shows the empty error state on every open.

## Suggested approach
1. Decide contract owner: audit which app currently owns notifications for realtor/portal users. If reality-server owns it, add `notifications::router()` at `backend/servers/reality-server/src/routes/notifications.rs` with GET `/`, GET `/unread-count`, PATCH `/{id}/read`, POST `/read-all`, DELETE `/{id}`, POST `/push-token`, DELETE `/push-token`, GET `/preferences`, PATCH `/preferences` and `.nest("/notifications", ...)` at `main.rs:475-519`. If api-server owns it, delete the reality-server-bound calls.
2. In `NotificationRepository.kt`, either (a) keep the URLs and rely on the new router, or (b) rewrite the `baseUrl` and paths to the api-server endpoints that already exist (see `backend/servers/api-server/src/routes/notifications*.rs`). Preserve the existing `Result<..>` shape and `NotificationException` messages.
3. Regenerate KMP data classes if you switch to api-server (its OpenAPI shape differs): re-run the openapi-generator step per `mobile-native/CLAUDE.md § API Client Generation`.
4. Extend the MockEngine tests to also assert `request.url.encodedPath == "<the real path>"` so any future contract drift is caught by unit tests (currently only body shape is checked).
5. Add a hermetic contract test in `backend/servers/reality-server/tests/suites/` that hits every KMP path with a manager-role token and asserts a non-404 status (200/401/403/404-body — anything but router-level 404) — this fails on `main` today for all nine paths.
6. Update `docs/screens/reality/` entries whose `implementations.mobile-native` references the notifications flow, per the screen-map protocol in `mobile-native/CLAUDE.md`.
7. Run `./gradlew :shared:allTests` and `cargo test -p reality-server routes::notifications` and confirm both suites are green.

## Alternatives considered
- **Add push-only endpoint on api-server, gut the rest** — rejected because the Android UI already renders in-app notifications lists and unread badges; the KMP surface is deliberately broader than "just push token", so we would still need `getNotifications` / `getUnreadCount` on *some* server (dropping them ships a regression, not a fix).
- **Leave the KMP calls, add a URL rewrite in `HttpClientProvider`** — rejected because rewriting URLs at the client layer hides the contract mismatch in a middleware, still leaves the KMP paths pointing at names no server exposes, and makes future audits worse rather than better.

## Root-cause trace
1. Symptom: KMP `NotificationRepository.get*()` calls return `Result.failure(NotificationException("... 404"))` against the real reality-server.
2. ← Immediate cause at `NotificationRepository.kt:35` — path `"/api/v1/notifications"` is not mounted on the reality-server axum router.
3. ← Upstream cause at `backend/servers/reality-server/src/main.rs:475-519` — the router-nest block never adds a notifications sub-router.
4. Origin: initial KMP notifications feature commit landed with the URL scheme assumed rather than verified against `main.rs`; no cross-repo contract test caught the drift (search: `git log --oneline --all -- mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/notifications/NotificationRepository.kt`).

## Test plan
- [ ] Extend `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/notifications/NotificationRepositoryTest.kt` so every method asserts `request.url.encodedPath` against the finalized real path — the current tests only assert body shape.
- [ ] Add `backend/servers/reality-server/tests/suites/notifications_router_contract_tests.rs` with one test per KMP call: authed request → status is NOT 404. Fails on `main` for all nine paths, passes after the fix.
- [ ] `./gradlew :shared:allTests` — must be green.
- [ ] `cargo test -p reality-server notifications_router_contract` — must be green.

## Out of scope
- Rewriting the mobile-native notifications UI itself.
- Migrating push-token storage between reality-server and api-server beyond what the router-fix requires.
- Reworking `HttpClientProvider` timeout/retry policy (already tracked as `code-review-mobile-native-kmp-httpclient-no-timeout`).

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-notification-repository-dead-endpoints.md`
- Mark the matching `backlog.json` row as `status: "done"`
