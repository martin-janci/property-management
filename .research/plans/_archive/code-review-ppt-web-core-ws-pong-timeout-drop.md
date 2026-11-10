# code-review-ppt-web-core-ws-pong-timeout-drop

**Vector:** bug
**Score:** 3
**Source:** Tier1d ppt-web-core review 2026-08-06 (rotating-expert-review, frontend expert)
**Confidence:** high

## Hypothesis
The ppt-web notification WebSocket client force-closes and reconnects roughly every 40 s for the entire authenticated session because it sends an application-level `{type:"ping"}` frame every 30 s and expects an application-level `{type:"pong"}` reply the server never emits. `ws_notifications.rs` uses protocol-level `Ping` and treats every inbound client frame as a heartbeat without ever serialising the string `"pong"` back. The `awaitingPong` flag therefore stays true, `pongTimeout` fires 10 s later, `socket.close(4000, "Pong timeout")` runs, and the reconnect loop starts over. The smallest correct fix is to remove the client-side app-level ping/pong entirely and rely on the server's protocol-level `Ping` + browser auto-pong for liveness.

## Evidence
- `frontend/apps/ppt-web/src/lib/websocket.ts:688-709` — heartbeat sends `{type:'ping', payload:null, timestamp}` every `heartbeatInterval` (default 30 000 ms, :288), sets `awaitingPong = true`, arms `pongTimeout` (default 10 000 ms, :289); the only clearing path is `handleMessage` :529-532 → `handlePong` :711-718 on receipt of `data.type === 'pong'`.
- `frontend/apps/ppt-web/src/lib/websocket.ts:59-76` — `parseServerMessage` maps every incoming `{event, payload}` envelope onto `type = <event name>`; there is no branch that produces `type: 'pong'`, so a real notification cannot clear `awaitingPong` either.
- `backend/servers/api-server/src/routes/ws_notifications.rs:219-260` — server only emits `Message::Text` for `WsEvent` envelopes and `Message::Ping(vec![])` for its own liveness probe; module header comment (`:27`) documents “Client → server: any message is treated as a heartbeat (pong)”; no code path serialises the string `"pong"` back.
- `frontend/apps/ppt-web/src/App.tsx:94-101` and `frontend/apps/ppt-web/src/contexts/WebSocketContext.tsx:250-251` — `<WebSocketProvider>` wraps the entire authenticated app with the default 30 s/10 s intervals; `service.connect()` runs whenever `auth.isAuthenticated && accessToken`, so every authenticated session is affected.
- Consequence chain: on-connect → 30 s later app-ping sent → server swallows → 10 s later `close(4000, "Pong timeout")` → `scheduleReconnect` → `connect` → repeat. Constant socket churn, missed pushes during each reconnect gap, spurious `onDisconnected`/`onReconnected` fires every ~40 s.

## Files
- `frontend/apps/ppt-web/src/lib/websocket.ts:59`
- `frontend/apps/ppt-web/src/lib/websocket.ts:288`
- `frontend/apps/ppt-web/src/lib/websocket.ts:688`
- `frontend/apps/ppt-web/src/contexts/WebSocketContext.tsx`
- `backend/servers/api-server/src/routes/ws_notifications.rs:27`

## Dependencies


## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Start the api-server and ppt-web locally (`stack up pm-local`), log in, and open the browser DevTools **Network → WS** tab on the notifications socket.
2. Wait 30–45 s while doing nothing else. Expected: single long-lived WS. Actual: at ~40 s the socket closes with code `4000 "Pong timeout"` and the client reconnects; the pattern repeats every ~40 s. In parallel, `onDisconnected`/`onReconnected` fire on every cycle (visible via `WebSocketContext` state changes).

## Suggested approach
1. In `frontend/apps/ppt-web/src/lib/websocket.ts`, delete `sendPing()`, the `pongTimeout` / `awaitingPong` machinery, and `handlePong` (approx :688-718 plus the `awaitingPong` field on the class and its two references in `startHeartbeat`/`stopHeartbeat`).
2. Rename `startHeartbeat`/`stopHeartbeat` (or remove them entirely) so the client no longer runs any application-level heartbeat — protocol-level `Ping` from the server + browser auto-pong is already the liveness mechanism (`ws_notifications.rs:242`).
3. Drop the now-dead `type: 'pong'` branch from `parseServerMessage` (`:59-76`) and remove the corresponding switch case in `handleMessage` (`:529-532`).
4. Remove/soften the `heartbeatInterval` / `pongTimeout` config fields on the `WebSocketConfig` type and their defaults (`:288-289`); if `WebSocketContext` sets them, drop those overrides.
5. Add a unit test (`frontend/apps/ppt-web/src/lib/__tests__/websocket.test.ts` or nearest existing sibling) that: (a) opens a mock WS, (b) advances fake timers by 60 s, (c) asserts the socket was **not** closed by the client and no reconnect was scheduled.
6. Manual verification: repeat the *Repro steps* — expected single long-lived socket for 5 minutes with no `4000 "Pong timeout"` close event, `onDisconnected`/`onReconnected` never firing.
7. Update the module header comment in `ws_notifications.rs:27` if needed to note the removal of the client-side app-level ping (documentation-only).

## Alternatives considered
- **Server echoes application `{type:"pong"}` on receipt of `{type:"ping"}`** — rejected because it doubles the heartbeat traffic (protocol-level `Ping` from server + application-level ping from client) while adding a second liveness contract that must be kept in sync between two languages. The minimal correct system has one liveness mechanism, and it already exists at the protocol layer.
- **Raise `pongTimeout` to a large value (e.g. 5 minutes) as a stop-gap** — rejected because it only hides the bug: `awaitingPong` still never clears, so the very first ping starts a 5-minute timer that always fires; you would still get a periodic force-close, just slower, and the client would still be running dead code.

## Root-cause trace
1. Symptom: authenticated ppt-web sessions show WS reconnect every ~40 s and briefly stop receiving realtime notifications on every reconnect.
2. ← `frontend/apps/ppt-web/src/lib/websocket.ts:705` — `this.socket?.close(4000, 'Pong timeout')` fires because `awaitingPong` is still `true` 10 s after the client sent `{type:'ping'}`.
3. ← `frontend/apps/ppt-web/src/lib/websocket.ts:688-709` — the client is the only party that can clear `awaitingPong`, but its clearing branch (`handlePong`, :711-718) is only reached when the server returns a text frame with `type === 'pong'`, which never happens.
4. ← `backend/servers/api-server/src/routes/ws_notifications.rs:219-260` — server only sends `WsEvent` text frames and protocol-level `Message::Ping`; it explicitly documents (`:27`) that every client frame is treated as a heartbeat and returns without replying.
5. Origin: the notification WS was implemented with application-level heartbeat on the client but the server was written to rely on the protocol-level heartbeat only; the two contracts never met. No single introducing PR — the mismatch has always been there since the WS notification client shipped.

## Test plan
- [ ] Add `frontend/apps/ppt-web/src/lib/__tests__/websocket.test.ts` case `does not force-close the socket after heartbeatInterval + pongTimeout with no server reply` (must fail on `main` because the current code closes with 4000 at ~40 s).
- [ ] Manual: run `stack up pm-local`, log in to ppt-web, keep the notifications WS open for 5 minutes; assert exactly one `open` event and zero `close` events in DevTools → Network → WS.
- [ ] Commands: `pnpm -F @ppt/ppt-web test -- websocket` (unit); `pnpm -F @ppt/ppt-web typecheck`; `pnpm -F @ppt/ppt-web build` (bundle sanity after code removal).

## Out of scope
- The related "reconnect give-up with no auto-resume" gap (backlog item `code-review-ppt-web-core-ws-giveup-no-resume`) — separate lifecycle bug, handled in its own plan.
- Server-side WS refactors beyond the optional documentation-only comment update at `ws_notifications.rs:27`.
- Any change to the `WsEvent` envelope, event names, or notification payload shape.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-ws-pong-timeout-drop.md`
- Mark the matching `backlog.json` row as `status: "done"`
