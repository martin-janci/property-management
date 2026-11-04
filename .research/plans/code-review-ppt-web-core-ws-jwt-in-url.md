# code-review-ppt-web-core-ws-jwt-in-url

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review 2026-11-03 ppt-web-core (dispatcher Tier-1d); upgraded to confidence=high in routine run 2026-11-04 (buffer-low refill) after re-reading websocket.ts
**Confidence:** high

## Hypothesis
`buildWebSocketUrl` places the full access JWT in the WebSocket URL query string (`frontend/apps/ppt-web/src/lib/websocket.ts:94-102`): `wss://host/ws/notifications?token=<jwt>`. The URL is persisted in api-server access logs, any reverse-proxy / CDN logs, APM/trace spans, and browser history; a leaked access JWT is a usable bearer credential until expiry (short — but non-zero window). Browsers do not allow an `Authorization` header on WS upgrades, but a short-lived single-use **ws-ticket** removes the long-usable token from the URL. Server-side change: add `POST /api/v1/auth/ws-ticket` returning an opaque, Redis-backed, ~30s one-time token the SPA passes as `?ticket=<opaque>`; the `ws_handler` resolves the ticket (atomic GETDEL in Redis) to the authenticated user and proceeds. Backwards-compatible: during rollout the handler accepts both `?token=<jwt>` (legacy) and `?ticket=<opaque>` (new) until the SPA fully migrates.

## Evidence
- `frontend/apps/ppt-web/src/lib/websocket.ts:94-102` — `buildWebSocketUrl` does `url.searchParams.set('token', token)` placing raw JWT in URL.
- `backend/servers/api-server/src/routes/ws_notifications.rs:78-84` — `WsQuery { pub token: String }` — the server consumes the JWT from the query string.
- `backend/servers/api-server/src/routes/ws_notifications.rs:108-114` — `ws_handler` reads `params.token` and validates it as a JWT.

## Files
- `backend/servers/api-server/src/routes/ws_notifications.rs`
- `backend/servers/api-server/src/routes/auth/mod.rs`
- `frontend/apps/ppt-web/src/lib/websocket.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [x] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Log in to ppt-web with DevTools → Network open. Watch for the WS upgrade on `/ws/notifications`.
2. Expected: the upgrade URL is `wss://host/ws/notifications?ticket=<opaque-32-chars>` — no JWT visible.
3. Actual: the upgrade URL is `wss://host/ws/notifications?token=<eyJhbGc...>` — the full JWT is visible in DevTools, history, and any access log.

## Suggested approach
1. In `backend/servers/api-server/src/routes/auth/mod.rs`, add `issue_ws_ticket` handler: authenticated-only `POST /api/v1/auth/ws-ticket`. Generate a 32-byte random opaque ticket; store `ws_ticket:<ticket>` → user_id in Redis with `EX 30`; return `{ ticket: string, expires_in: 30 }`.
2. Mount the new route under the authenticated `/auth` routes group alongside `/refresh`, `/logout`.
3. In `backend/servers/api-server/src/routes/ws_notifications.rs`, extend `WsQuery` to `{ token: Option<String>, ticket: Option<String> }`. When `ticket` is present, resolve it via atomic Redis GETDEL (`ws_ticket:<ticket>`) → user_id; on success proceed as if the JWT had resolved to that user. When only `token` is present, keep the existing JWT path (legacy). Reject when both or neither is present.
4. In `frontend/apps/ppt-web/src/lib/websocket.ts`, change `buildWebSocketUrl(base, token)` to `async buildWebSocketUrl(base, api)` that calls `POST /api/v1/auth/ws-ticket` first, then sets `url.searchParams.set('ticket', data.ticket)`.
5. Update every call site of `buildWebSocketUrl` to await the async ticket fetch; cache the ticket per connection attempt only (no pooling — each new upgrade gets a fresh ticket).
6. Add regression test in `backend/servers/api-server/tests/` that a `ws-ticket` is one-shot: a second upgrade with the same ticket returns 401.
7. Leave the `?token=<jwt>` compatibility branch behind a deprecation log + metric for one release; remove in the follow-up plan.

## Alternatives considered
- **Mask the token in access logs via a tracing filter** — rejected because it does not cover reverse-proxy, CDN, APM, or browser history; only addresses in-process logs.
- **Switch to a Sec-WebSocket-Protocol subprotocol carrying the JWT** — rejected because the subprotocol header is still printed in a subset of logs/traces and APM spans; the ticket design fully separates auth material from the URL/handshake payload.

## Root-cause trace
1. Symptom: access JWT visible in access logs + browser history for every WS upgrade.
2. ← `frontend/apps/ppt-web/src/lib/websocket.ts:100` — `url.searchParams.set('token', token)` embeds JWT in URL.
3. ← Browser WebSocket API limitation — no `Authorization` header allowed on handshake — forced the SPA to use a URL param.
4. Origin: the initial WS-auth commit (Story 8A.3 — notification sync). The constraint is the browser's; the fix is a server-side ticket indirection, never introduced.

## Test plan
- [ ] Unit test in api-server: `ws_handler` accepts `?ticket=<valid>` and upgrades; a second upgrade with the same ticket returns 401.
- [ ] Unit test in api-server: `ws_handler` still accepts `?token=<jwt>` (legacy branch) during the deprecation window.
- [ ] Unit test in ppt-web: `buildWebSocketUrl` calls the ticket endpoint before returning a URL; no `token` query param present.
- [ ] Run: `cargo test -p api-server -- ws_notifications::` and `pnpm -F @ppt/web test -- websocket`.

## Out of scope
- `backend/servers/api-server/src/routes/iot/realtime.rs` (SensorWsQuery) — separate sensor WS path, same pattern but different consumer; follow-up plan.
- Mobile RN / KMP WebSocket connections — plan is for ppt-web only. KMP + RN apps will migrate once the ticket endpoint is live.
- Removing the legacy `?token=` branch — one-release deprecation window; separate cleanup plan after dashboards confirm zero legacy traffic.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-ws-jwt-in-url.md`
- Mark the matching `backlog.json` row as `status: "done"`
