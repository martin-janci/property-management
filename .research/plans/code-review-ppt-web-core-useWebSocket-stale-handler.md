# code-review-ppt-web-core-useWebSocket-stale-handler

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 review of ppt-web-core segment (2026-06-24)
**Confidence:** high

## Hypothesis
`useWebSocket(eventType, handler)` keeps `handler` in a ref but its subscription effect only runs when `[eventType, subscribe]` change. If a caller initially passes `handler === undefined` (lazy / conditional) and later supplies a real callback, the effect's early-return `if (!eventType || !handlerRef.current) return` fired on the first run and never re-fires — so no subscription is ever created. Every realtime event for that subscriber is silently dropped. The fix is to include `handler` in the dependency array (or split lazy vs eager handler patterns) so the effect re-runs once the handler becomes defined.

## Evidence
- `frontend/apps/ppt-web/src/hooks/useWebSocket.ts:83-95` — effect body early-returns on `!handlerRef.current` but deps are `[eventType, subscribe]` only.
- `frontend/apps/ppt-web/src/hooks/useWebSocket.ts:80` — `handlerRef.current = handler` assignment on every render keeps the ref fresh, but the subscription gate already short-circuited on first render.
- Phase 1.5 (2026-06-24) ppt-web-core review identified this as medium-severity (upgraded to high after manual line read).
- No callers in `src/features/**` import this hook with an unconditional handler that we could rule out — the unsafe pattern is reachable.

## Files
- `frontend/apps/ppt-web/src/hooks/useWebSocket.ts:83`

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
1. In a test file under `frontend/apps/ppt-web/src/hooks/`, mount `useWebSocket('entity:update', undefined)` with a `WebSocketProvider` wrapper, assert `subscribe` is not called.
2. Re-render the hook with a real handler `(msg) => { received.push(msg) }`.
3. Publish a matching event via the provider's test harness.
4. Expected: `received.length === 1`. Actual: `received.length === 0` — no subscription was ever created.

## Suggested approach
1. Add `handler` to the effect dep array (`[eventType, subscribe, handler]`) — but wrap callers' inline arrows in `useCallback` or accept the re-subscribe cost (this is the cheapest fix; matches React's lint rules).
2. Alternative cleaner shape: drop the ref + early-return, accept that `handler` should be stable. Update the JSDoc above the hook accordingly.
3. Add a Vitest case in a new `useWebSocket.test.tsx` that reproduces the lazy-handler scenario from *Repro steps*.
4. Sweep `useWebSocket` call sites (`grep -rn 'useWebSocket(' frontend/apps/ppt-web/src/`) — flag any caller passing an inline arrow that re-allocates each render; either memoize or accept the new re-subscribe cycle.

## Alternatives considered
- **Document the constraint ("handler must be defined on first render") in JSDoc only** — rejected because the silent-drop failure mode is invisible at runtime; documentation doesn't prevent regressions.
- **Switch to a context-emitter pattern (publish/subscribe outside React)** — rejected because the existing `WebSocketContext` already centralises the socket; adding another layer for one hook is over-engineering for the buffer this run.

## Root-cause trace
1. Symptom: subscriber's `onMessage` callback never runs for events that match its `eventType`.
2. ← Inside `useWebSocket`, the subscription `useEffect` early-returned at `useWebSocket.ts:84` because `handlerRef.current` was `undefined` on first render.
3. ← The effect's dep array `[eventType, subscribe]` (`useWebSocket.ts:95`) doesn't include `handler`, so the effect never re-runs when the caller supplies the handler on a later render.
4. Origin: introduced in the initial `useWebSocket` design (predates the Phase 1.5 review window; check `git log -- frontend/apps/ppt-web/src/hooks/useWebSocket.ts` for the first commit).

## Test plan
- [ ] New `frontend/apps/ppt-web/src/hooks/useWebSocket.test.tsx` reproduces the lazy-handler bug (fails on `main`).
- [ ] Regression: same test plus a second case mounting with `handler` defined from render 1 — passes both before and after.
- [ ] Local command: `pnpm -F @ppt/web test useWebSocket`

## Out of scope
- Rewriting the broader `WebSocketContext` / `WebSocketService` lifecycle (see sibling plan `code-review-ppt-web-core-ws-token-rotation`).
- Server-side WS broadcast guarantees (separate dispatcher signal `#1792` follow-up).

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-useWebSocket-stale-handler.md`
- Mark `code-review-ppt-web-core-useWebSocket-stale-handler` in `backlog.json` as `status: "done"`
