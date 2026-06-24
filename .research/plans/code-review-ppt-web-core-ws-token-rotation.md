# code-review-ppt-web-core-ws-token-rotation

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 review of ppt-web-core segment (2026-06-24)
**Confidence:** high

## Hypothesis
After an access-token refresh, the live `WebSocketService` keeps its old JWT and is never reconnected. `WebSocketContext` runs an effect on `[auth.isAuthenticated, auth.accessToken]` that calls `service.connect()`, but `WebSocketService.connect()` short-circuits with `if (this.socket && this.socket.readyState === WebSocket.OPEN) return` — so a still-open socket is never replaced. When the original JWT eventually expires, the server drops the session and pushes stop arriving; the client keeps `isConnected = true` until the next page reload. Fix: on `accessToken` change while connected, call `service.disconnect()` then `service.connect()` so the new token is sent in the WS handshake.

## Evidence
- `frontend/apps/ppt-web/src/contexts/WebSocketContext.tsx:246-256` — effect deps include `auth.accessToken` but only calls `service.connect()` (no disconnect-then-reconnect path).
- `frontend/apps/ppt-web/src/lib/websocket.ts:291-294` — `connect()` returns early when `socket.readyState === OPEN`, so the new token is never used.
- `frontend/apps/ppt-web/src/lib/websocket.ts:295-301` — token is read fresh inside `connect()` via `this.getToken()`, confirming the intent is "reconnect with current token" but the gate at 291 defeats it.
- Phase 1.5 (2026-06-24) ppt-web-core review identified this as medium-severity (upgraded to high after manual line read — security-relevant because stale token equals expired auth in production).

## Files
- `frontend/apps/ppt-web/src/contexts/WebSocketContext.tsx:246`
- `frontend/apps/ppt-web/src/lib/websocket.ts:291`

## Dependencies
<!-- none -->

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
1. Mock `auth.accessToken` to value `"jwt-A"`, mount `WebSocketProvider`, wait for `service.isConnected() === true` (socket OPEN).
2. Update mock `auth.accessToken` to `"jwt-B"` while the socket is still OPEN.
3. Inspect the underlying `WebSocketService`'s last-handshake URL (or spy on `getToken`): expected the WS to have reopened with `jwt-B`; actual it still holds the `jwt-A` handshake.

## Suggested approach
1. In `WebSocketContext.tsx` (around line 250), capture the previous `accessToken` via a ref. When the token changes from a non-null value to a different non-null value while `service.isConnected()`, call `service.disconnect()` then `service.connect()`.
2. Alternative (cleaner): add a `reconnect(): void` method to `WebSocketService` that always disconnects-then-connects, ignoring the OPEN guard. Wire the effect to call `reconnect()` on token change (and `connect()` on first authentication only).
3. Update `frontend/apps/ppt-web/src/lib/websocket.ts:288-291` JSDoc to clarify: "`connect()` is a no-op if already connected — call `reconnect()` to force a new handshake with current credentials."
4. Add a Vitest case in a new `WebSocketContext.test.tsx` that mocks `WebSocketService` and verifies `reconnect()` (or disconnect+connect) fires on token change.

## Alternatives considered
- **Server-pushed `token_expired` event triggering client reconnect** — rejected because it requires a server-side change (new WS message type, broadcast logic) for a problem we can fix client-side; also fails if the token expires *during* a connection-drop window.
- **Periodic poll of `service.isConnected()` and force-reconnect every refresh interval** — rejected because it adds wasted reconnects in the happy path and doesn't actually solve the contract (token must change before reconnect).

## Root-cause trace
1. Symptom: after a token refresh, the user's WS messages stop arriving silently; reload restores the stream.
2. ← `WebSocketContext.tsx:251` — effect calls `service.connect()` on `accessToken` change.
3. ← `lib/websocket.ts:291-294` — `connect()` short-circuits because socket is OPEN; the new token is never used.
4. Origin: introduced when the OPEN guard was added to `connect()` (likely an over-eager dedup; check `git log -L 291,294:frontend/apps/ppt-web/src/lib/websocket.ts`).

## Test plan
- [ ] New `frontend/apps/ppt-web/src/contexts/WebSocketContext.test.tsx` mounts the provider with a mocked `WebSocketService`, simulates a token change, asserts `disconnect()` + `connect()` were called (fails on `main`).
- [ ] Regression: same test plus a "token unchanged" case — asserts no churn (no disconnect/connect cycle).
- [ ] Local command: `pnpm -F @ppt/web test WebSocketContext`

## Out of scope
- Handling the broader `useWebSocket(eventType, handler)` subscription bug (see sibling plan `code-review-ppt-web-core-useWebSocket-stale-handler`).
- Server-side token-revocation/blacklist behaviour.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-ws-token-rotation.md`
- Mark `code-review-ppt-web-core-ws-token-rotation` in `backlog.json` as `status: "done"`
