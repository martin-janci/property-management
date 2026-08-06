# code-review-ppt-web-core-ws-pong-timeout-drop

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review (Tier-1d) 2026-08-06 ppt-web-core segment
**Confidence:** high

## Hypothesis
The ppt-web notification WebSocket client sends an application-level `{type:'ping'}` heartbeat every 30 s and force-closes the socket 10 s later when no matching `{type:'pong'}` arrives. The server (`ws_notifications.rs`) never emits `{type:'pong'}` — it treats every inbound client frame as a heartbeat and only sends protocol-level `Message::Ping`. Result: every authenticated session's realtime channel churns (close → reconnect) roughly every 40 s, dropping pushes and thrashing WS session setup. Fix: either remove the client's app-level ping/pong (rely on protocol-level ping and browser auto-pong), or have the server echo `{type:'pong'}` on receipt of `{type:'ping'}` so the contract matches. Removing the client-side logic is preferable — it's redundant with the protocol-level probe the server already sends.

## Evidence
- `frontend/apps/ppt-web/src/lib/websocket.ts:688-709` — client sends app-level ping, arms 10 s pongTimeout, force-closes with code 4000 if no `{type:'pong'}` reply arrives.
- `backend/servers/api-server/src/routes/ws_notifications.rs:219-260` — server emits only notification envelopes and protocol-level `Message::Ping`; module header (`:27`) states "any client message is treated as a heartbeat and returns without replying".
- `frontend/apps/ppt-web/src/lib/websocket.ts:529-532, 711-718` — only a text frame with `data.type === 'pong'` clears `awaitingPong`; no server code path serialises that string.
- `frontend/apps/ppt-web/src/App.tsx:94-101` + `contexts/WebSocketContext.tsx:250-251` — `<WebSocketProvider>` mounts around the whole authenticated app with defaults (30 s / 10 s), so reachability is production-wide.
- Signal id `code-review-ppt-web-core-ws-pong-timeout-drop` in `.research/signals/2026-08-06-ppt-web-core-tier1d.json`.

## Files
- `frontend/apps/ppt-web/src/lib/websocket.ts:688`
- `frontend/apps/ppt-web/src/lib/websocket.ts:711`
- `backend/servers/api-server/src/routes/ws_notifications.rs`
- `frontend/apps/ppt-web/src/contexts/WebSocketContext.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Mode: cloud-ok** — pure frontend + Rust change; verifiable with unit tests, no browser DOM inspection required (the failing behaviour is deterministic in code).

## Repro steps
1. Load ppt-web as an authenticated user and open the browser DevTools → Network → WS.
2. Wait 40 s while the app is idle; observe the notification socket closing with code 4000 ("Pong timeout") and immediately reconnecting.
3. Expected: the socket stays open for the whole session (server protocol-ping + browser auto-pong keep it alive).
   Actual: close/reopen every ~40 s.

## Suggested approach
1. In `frontend/apps/ppt-web/src/lib/websocket.ts`, delete the app-level ping path: remove `sendPing()`/`awaitingPong`/`pongTimeout` state, the `heartbeatInterval` setInterval body, and the `handlePong` branch (`:529-532`, `:688-718`). Keep `startHeartbeat`/`stopHeartbeat` names if they're referenced from outside, but reduce them to no-ops (or delete them and their two call sites at `:488-493`, `:507`).
2. Delete the now-unused `WebSocketConfig.heartbeatInterval` / `pongTimeout` fields (`:288-289`) and their defaults; delete `resetReconnectAttempts()` if no caller remains (it's currently dead code per the sibling finding).
3. Confirm liveness is now driven by the server's protocol-level ping (`ws_notifications.rs:242`, `Message::Ping(vec![])`) — the browser answers automatically with a Pong frame, invisible to JS. No client change needed for that path.
4. Add a Vitest for `WebSocketService`: mock a WS transport, drive it through `connect` → wait ≥45 s of mocked time → assert no `close(4000)` was invoked and `service.getState()` remains `connected`.
5. If a manual smoke is added, verify in DevTools that the socket stays open ≥5 min while idle.

## Alternatives considered
- **Have the server echo `{type:'pong'}` on `{type:'ping'}`** — rejected because it duplicates the protocol-level ping/pong the server already runs; adds a codepath (JSON parse → serialise) purely to serve a client contract we can remove instead. Two heartbeats > one.
- **Increase `pongTimeout` to hide the drop** — rejected because the socket would still close on the first genuine backpressure event; the bug is a missing contract, not a tight timeout.

## Root-cause trace
1. Symptom: notification WS closes with code 4000 ~40 s after connect and loops reconnect for the entire session.
2. ← `frontend/apps/ppt-web/src/lib/websocket.ts:705` — `this.socket?.close(4000, 'Pong timeout')` fires when `awaitingPong` is still true after `pongTimeout`.
3. ← `frontend/apps/ppt-web/src/lib/websocket.ts:711-718` — `awaitingPong` is only cleared by an incoming text frame with `data.type === 'pong'`.
4. ← `backend/servers/api-server/src/routes/ws_notifications.rs:219-260` + module header (`:27`) — server never emits an app-level `{type:'pong'}`; the client's contract has no server counterpart.
5. Origin: the app-level ping was added client-side without a matching server handler. No single introducing commit was cited by the reviewer; the mismatch has been latent since the WS handler shipped (Epic 8A / Story 8A.3, per the file header).

## Test plan
- [ ] Vitest — `frontend/apps/ppt-web/src/lib/__tests__/websocket.test.ts` (new) — mock transport, advance timers ≥45 s, assert `.close` not called with code 4000 and state stays `connected`.
- [ ] Manual smoke — open ppt-web as an authenticated user; observe DevTools → Network → WS; the notification socket stays open ≥5 min while idle; emit a notification server-side and verify it arrives on the still-open socket.
- [ ] Command: `pnpm --filter @ppt/ppt-web test -- src/lib/__tests__/websocket.test.ts`

## Out of scope
- The reconnect-give-up / no-online-resume behaviour flagged by `code-review-ppt-web-core-ws-giveup-no-resume` (score 2, sibling finding); handle in a separate plan once bumped.
- Server-side WS handler refactor.
- Auth-token rotation (covered by the pre-existing `ws-token-rotation-stale` finding).

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-ws-pong-timeout-drop.md`
- Mark the matching `backlog.json` row as `status: "done"`
