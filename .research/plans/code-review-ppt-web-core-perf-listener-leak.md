# code-review-ppt-web-core-perf-listener-leak

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 rotating expert review 2026-07-06 (segment: ppt-web-core)
**Confidence:** high

## Hypothesis
`usePerformanceMetrics.ts` adds two window listeners (`visibilitychange` at line ~153, `load` at line ~142) inside a `useEffect`, but the cleanup only calls `observer.disconnect()` — the window listeners are never removed. Every time the effect re-runs (deps `[onReport, reportOnUnload]` change identity, or React StrictMode double-mount), the old listeners leak while a new pair is attached. Each stale listener captures an old `reportMetrics`/`onReport` closure and continues firing on visibility flips forever. The smallest fix is to store the two handler refs in `useRef` (stable identity) and add matching `window.removeEventListener` calls in the cleanup.

## Evidence
- `frontend/apps/ppt-web/src/hooks/usePerformanceMetrics.ts:142` — `window.addEventListener('load', ...)` inside effect
- `frontend/apps/ppt-web/src/hooks/usePerformanceMetrics.ts:153` — `window.addEventListener('visibilitychange', ...)` inside effect
- `frontend/apps/ppt-web/src/hooks/usePerformanceMetrics.ts:160-164` — cleanup only calls `observer.disconnect()`; no `removeEventListener`
- Effect deps `[onReport, reportOnUnload]` — parents commonly pass an inline `onReport={data => ...}` so identity changes each render, forcing re-runs
- StrictMode double-mounts every effect in dev — the leak is measurable on first mount even without prop changes

## Files
- `frontend/apps/ppt-web/src/hooks/usePerformanceMetrics.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived):** Mode: cloud-ok

## Repro steps
1. In `usePerformanceMetrics` provide a parent that re-renders with a different inline `onReport` callback identity every second (e.g. `useState` interval → `onReport={data => setState(data)}`).
2. Spy on `window.addEventListener` and `window.removeEventListener` in a Vitest test.
3. Expected: `add` count == `remove` count after N re-runs; each visibilitychange dispatches the *current* callback only.
4. Actual: `add` count == 2·N, `remove` count == 0; multiple stale callbacks fire on each visibility flip.

## Suggested approach
1. Extract stable handler refs: `const loadHandlerRef = useRef<() => void>();` and `const visHandlerRef = useRef<() => void>();`
2. Inside the effect assign `loadHandlerRef.current = () => { ... }` (the current handler body).
3. `window.addEventListener('load', loadHandlerRef.current, { once: true })`.
4. `window.addEventListener('visibilitychange', visHandlerRef.current!)`.
5. In cleanup: `window.removeEventListener('load', loadHandlerRef.current!)` (only if not fired) and `window.removeEventListener('visibilitychange', visHandlerRef.current!)`.
6. If `once: true` is used for `load`, the cleanup safely can skip its removal — but keep it symmetric for clarity.
7. Add a `useEffect` deps note explaining why the deps intentionally include `onReport` and why the ref pattern prevents stale-closure calls.

## Alternatives considered
- **Move listeners outside the effect** (module-scope) — rejected because they'd fire before any consumer mounted and lose per-instance `onReport` context.
- **Debounce `onReport` identity via `useCallback` in every parent** — rejected because it pushes the fix responsibility onto callers; the hook should be memory-safe regardless of prop stability.

## Root-cause trace
1. Symptom: Chrome DevTools shows a growing count of anonymous listeners on `window` after tab-switch cycles; hidden-tab reports fire stale callbacks (visible via console.log traps).
2. ← `useEffect` cleanup at `usePerformanceMetrics.ts:160-164` calls only `observer.disconnect()` — the two `addEventListener` calls at lines 142/153 have no matching `removeEventListener`.
3. ← Author added the visibility/load listeners without the paired cleanup; effect deps include `onReport` so re-runs are expected but each leaves the previous listener orphaned.
4. Origin: initial commit of `usePerformanceMetrics.ts`. Pin the exact commit with `git log --diff-filter=A --follow -- frontend/apps/ppt-web/src/hooks/usePerformanceMetrics.ts` before landing the fix.

## Test plan
- [ ] `frontend/apps/ppt-web/src/hooks/__tests__/usePerformanceMetrics.test.ts` — new file: spy on `window.add/removeEventListener`, mount hook, re-render with new `onReport`, assert add == remove.
- [ ] Second scenario: mount → unmount, assert both listeners removed.
- [ ] Command: `pnpm -F @ppt/ppt-web test -- usePerformanceMetrics`

## Out of scope
- Fixing similar patterns in other hooks (`useVirtualizer`, `useIntersectionObserver` if any) — file separate plans if the audit surfaces them.
- Redesigning the metrics-reporting API surface — keep the fix minimal.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-perf-listener-leak.md`
- Mark the matching `backlog.json` row (`id=code-review-ppt-web-core-perf-listener-leak`) as `status: "done"`
