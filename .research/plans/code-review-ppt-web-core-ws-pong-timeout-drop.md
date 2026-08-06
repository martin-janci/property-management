# code-review-ppt-web-core-ws-pong-timeout-drop

**Vector:** bug
**Score:** 3
**Source:** Tier1d review 2026-08-06 (ppt-web-core) — signal `code-review-ppt-web-core-ws-pong-timeout-drop`
**Confidence:** high

## Hypothesis
The ppt-web WebSocket client asks the server to reply with an application-level `{type:'pong'}` frame every 30s and force-closes the socket (code 4000, `Pong timeout`) 10s later if none arrives. The api-server notification WS handler never emits an application `pong` — it treats every inbound frame as a heartbeat and uses PROTOCOL-level `Message::Ping` for liveness. The contract mismatch means every fresh authenticated socket reconnects roughly every 40s for the whole session, missing pushes in each gap and generating server churn. Fix: drop the client's application-level ping (rely on the browser's automatic protocol pong to the server's `Message::Ping`) — the smallest change that resolves the loop without a new server contract.

## Evidence
- `frontend/apps/ppt-web/src/lib/websocket.ts:688-709` — heartbeat sends `{type:'ping', payload:null, timestamp}` via `send()`, sets `awaitingPong = true`, arms `pongTimeout` (default 10000ms).
- `frontend/apps/ppt-web/src/lib/websocket.ts:529-532, 711-718` — `handlePong` only fires on a text frame whose `data.type === 'pong'`; no other code path clears `awaitingPong`.
- `backend/servers/api-server/src/routes/ws_notifications.rs:27, 219-260` — module header explicitly documents "any client message is treated as a heartbeat (pong)"; the inbound arm receives Text/Binary/Ping/Pong and returns without replying. Outbound uses `Message::Ping(vec![])` (:242), never a `{type:'pong'}` payload.
- `frontend/apps/ppt-web/src/App.tsx:94-101` — `<WebSocketProvider>` wraps the authenticated app with no interval overrides, so 30s heartbeat / 10s pong timeout apply in production; `WebSocketContext.tsx:250-251` calls `service.connect()` whenever `auth.isAuthenticated && accessToken`, so the loop reaches every logged-in user.
- Signal file: `.research/signals/2026-08-06-ppt-web-core-tier1d.json` (id `code-review-ppt-web-core-ws-pong-timeout-drop`).

## Files
- `frontend/apps/ppt-web/src/lib/websocket.ts`
- `frontend/apps/ppt-web/src/lib/websocket.test.ts`
- `backend/servers/api-server/src/routes/ws_notifications.rs`

## Dependencies
<none>

## Required capabilities
- [x] C1 — Systematic debugging (bug fix — bisect the 40s loop against message-flow log)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
`Mode: cloud-ok` — no C4/C5; a jest/vitest unit test drives a mock `WebSocket` and asserts the client never sends an application-level ping frame after the fix; backend touch is a comment-only clarification that can be verified via `cargo check -p api-server`.

## Repro steps
1. `cd frontend && pnpm dev:ppt` and log in with any test tenant so `<WebSocketProvider>` mounts around the authenticated app.
2. Open browser devtools → Network → WS, filter the notification socket, and watch outgoing frames for ~90s.
3. **Expected after fix:** no `{type:"ping"}` payload frames leave the client; the socket stays open. `connection:disconnected`/`connection:reconnected` events don't fire in `WebSocketContext` traces during the idle window.
4. **Actual today:** an outgoing `{type:"ping"}` at ~30s, followed by a client-initiated close (`code 4000, reason "Pong timeout"`) at ~40s, followed by `connect()` → `onopen` → the cycle repeats indefinitely.

## Suggested approach
1. In `frontend/apps/ppt-web/src/lib/websocket.ts`, delete the app-level ping/pong lifecycle: remove `sendPing()`, the `pongTimeout` timer, `awaitingPong`, `handlePong`, and the `heartbeatInterval`/`pongTimeout` config wiring. Rely on the server's PROTOCOL `Message::Ping` + the browser's automatic protocol pong (invisible to JS) for liveness.
2. Keep `WebSocketOptions` typing stable — if `heartbeatInterval`/`pongTimeout` were exported types, mark them deprecated (JSDoc `@deprecated` only, no runtime shim) so consumers don't get a silent behavior change.
3. Update `frontend/apps/ppt-web/src/lib/websocket.test.ts` (or add if absent): mock `WebSocket`, drive `connect()` through a fake `open`, advance fake timers past the old 30s+10s window, and assert `mockSocket.send` was never called with any payload whose parsed `type === 'ping'` and that `mockSocket.close` was NOT called with code 4000.
4. In `backend/servers/api-server/src/routes/ws_notifications.rs:27`, tighten the module comment to explicitly state that clients MUST NOT expect an application `pong` reply — the browser's protocol pong to the server's `Message::Ping` is the sanctioned liveness signal (documentation-only touch; keeps the contract explicit for the next reader).
5. Verify: `cd frontend && pnpm -F @ppt/web test -- websocket` (the new+existing WS tests), `pnpm -F @ppt/web typecheck`, `cd backend && cargo check -p api-server`.
6. Regression guard: keep the failing-on-main assertion `expect(sentTypes).not.toContain('ping')` as an explicit named `it()` so any future re-introduction of application-level pinging fails fast.

## Alternatives considered
- **Server echoes `{type:'pong'}` on receipt of `{type:'ping'}`** — rejected because it enshrines a bespoke application heartbeat when the browser + server already exchange a working protocol-level ping/pong. It also adds a per-connection code path server-side, expanding attack surface (a client can force serialization work with a tight ping loop), for zero user-visible gain.
- **Increase `heartbeatInterval` to a value larger than any realistic session** — rejected because it only delays the same failure mode (the socket still force-closes as soon as the timer fires without a real pong contract in place); it also hides the underlying contract bug and makes the eventual debugger's job harder.

## Root-cause trace
1. Symptom: notification socket cycles disconnect→connect roughly every 40s for an authenticated session; missed realtime pushes in each ~500ms reconnect gap; browser devtools show a client-initiated `close(4000, "Pong timeout")`.
2. ← Immediate cause at `frontend/apps/ppt-web/src/lib/websocket.ts:705` — `this.socket?.close(4000, 'Pong timeout')` fires because `awaitingPong` was never cleared inside `pongTimeout` (default 10000ms).
3. ← Upstream cause at `frontend/apps/ppt-web/src/lib/websocket.ts:711-718, 529-532` — `handlePong` is the ONLY place that flips `awaitingPong = false`, and it requires a text frame with `data.type === 'pong'`.
4. ← Contract cause at `backend/servers/api-server/src/routes/ws_notifications.rs:27, 257-260` — the server explicitly treats every inbound frame as a heartbeat and never replies with an application payload; it only sends outbound `Message::Ping(vec![])` (protocol-level).
5. Origin: the app-level heartbeat was introduced in the ppt-web client without a matching server-side echo. A backend audit at the time would have caught the mismatch; the two sides drifted because neither had an integration-level assertion that a client `ping` yields a client-visible `pong`.

## Test plan
- [ ] `frontend/apps/ppt-web/src/lib/websocket.test.ts` — new `it("never sends an application-level ping frame")` that instantiates the service, drives a fake `open`, advances fake timers past 90s, and asserts the array of stringified `send()` payloads has NO `JSON.parse(x).type === 'ping'` and `close()` was not called with code 4000.
- [ ] Retain any existing `handlePong` unit test as an obsolete-contract sentinel: replace it with an assertion that `handleMessage` treats `{type:'pong'}` as a no-op (safety net if a stale server ever emits one).
- [ ] Local runs:
  - `cd frontend && pnpm -F @ppt/web test -- websocket`
  - `cd frontend && pnpm -F @ppt/web typecheck`
  - `cd backend && cargo check -p api-server`

## Out of scope
- The reconnect-give-up / online-resume gap (backlog item `code-review-ppt-web-core-ws-giveup-no-resume`, score 2). That's a distinct lifecycle bug; keep it in its own plan when it hits the readiness threshold.
- Any change to `reality-web`'s WS client (this plan is scoped to ppt-web + api-server).
- Rewriting the WS event envelope or introducing a `pong` server responder.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-ws-pong-timeout-drop.md`
- Mark the matching `backlog.json` row (`code-review-ppt-web-core-ws-pong-timeout-drop`) as `status: "done"`
