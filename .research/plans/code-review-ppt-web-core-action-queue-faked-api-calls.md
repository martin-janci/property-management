# code-review-ppt-web-core-action-queue-faked-api-calls

**Vector:** bug
**Score:** 3
**Source:** code-review-ppt-web-core 2026-07-01
**Confidence:** high

## Hypothesis
`useActionQueue()` in `frontend/apps/ppt-web/src/features/dashboard/hooks/useActionQueue.ts` is the production dashboard queue for managers and residents, but its `queryFn` is entirely mocked: `await new Promise(resolve => setTimeout(resolve, 300))` followed by `generateMockData(role)`, with a `TODO: Replace with actual API call when backend is ready` comment. The dashboard shows fabricated items, not real work — filters, priorities, and counts are all synthesised in the browser. Users act on data that doesn't exist.

## Evidence
- `frontend/apps/ppt-web/src/features/dashboard/hooks/useActionQueue.ts:240-269` — `useQuery({ queryKey: ['actionQueue', role, filters], queryFn: async () => { ...setTimeout(resolve, 300)... const data = generateMockData(role); ... }})`.
- `useActionQueue.ts:243-245` — inline comment: `// TODO: Replace with actual API call when backend is ready // const response = await fetch(\`/api/v1/action-queue?role=${role}\`);`
- `useActionQueue.ts:230` `generateMockData` is defined earlier in the same file — the mock generator ships in the client bundle.
- Grep the dashboard for callers: any dashboard page mounting this hook renders the mock.

## Files
- `frontend/apps/ppt-web/src/features/dashboard/hooks/useActionQueue.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

**Execution mode:** cloud-ok
Mode: cloud-ok

## Repro steps
1. Boot ppt-web against any api-server; open the manager dashboard as an org_admin.
2. Observe the "Action queue" widget. It always renders the same shape of ~8 items regardless of tenant, seed data, or fault/vote state in the DB.
3. `curl -H "$AUTH_HEADER" http://localhost:8080/api/v1/action-queue?role=manager` → **404 or 501** (endpoint has never been wired).
4. **Expected:** either the widget calls the real endpoint (and is empty for an empty tenant), or it clearly renders a "coming soon" state — not a mock that reads like production data.

## Suggested approach
1. Decide with the product/backend owner (comment on the resulting PR) which of the two ships:
   - **A. Wire the real endpoint.** Verify `/api/v1/action-queue` exists in `backend/servers/api-server/src/routes/*` — grep for it. If absent, this becomes a two-PR task (backend endpoint first). Do NOT ship the endpoint from this PR.
   - **B. Feature-flag the widget.** Replace `queryFn` body with `throw new Error('not-implemented')` and gate the widget behind an `isEnabled` prop that defaults to false (or an env `VITE_FEATURE_ACTION_QUEUE=false`). Render a clearly-labeled "Coming soon" placeholder.
2. Delete the `generateMockData` helper and its supporting mock types from the file — no mock data should ship in the production bundle regardless of which path (A) or (B) we pick.
3. If (A): keep filters/priority merging client-side (that logic is fine), but source `data.items` from `apiFetch<ActionQueueData>('/api/v1/action-queue?role=…')`.
4. Update the callers to handle the new error state (should already be zero-diff since `useQuery` exposes `isError`).

## Alternatives considered
- **Ship the mock as-is with a "demo mode" badge** — rejected: real users on the dashboard read the fake data as real work. A visible badge doesn't prevent them from acting on it.
- **Delete the widget entirely** — rejected as scope creep; the widget's shape and filter logic look correct, only the data source is fake. Preserve the shape; swap the source.

## Root-cause trace
1. Symptom: dashboard action queue always shows the same synthetic items across tenants and roles.
2. ← `useActionQueue.ts:248` `queryFn` returns `generateMockData(role)` after a fake 300 ms delay.
3. ← `useActionQueue.ts:243` explicit `TODO: Replace with actual API call when backend is ready` — landed as a placeholder during scaffolding and never revisited.
4. Origin: initial dashboard scaffold; the placeholder was intended to be gated behind an env flag but was shipped enabled.

## Test plan
- [ ] Vitest test in `frontend/apps/ppt-web/src/features/dashboard/hooks/__tests__/useActionQueue.test.ts`: with `apiFetch` mocked to a fixed payload, `useActionQueue('manager')` returns that payload (proves the mutation actually hit the network stub, not the removed `generateMockData`).
- [ ] Regression: if the endpoint 500s, `useActionQueue` surfaces `isError=true`.
- [ ] Manual: after change, open the dashboard against a clean-DB tenant → widget shows 0 items or a "Coming soon" state, NOT the same 8 mocked entries.
- [ ] Local: `cd frontend && pnpm --filter @ppt/web test -- useActionQueue`

## Out of scope
- Building the `/api/v1/action-queue` backend endpoint (if it doesn't exist yet — that's a separate backend PR, filed as a follow-up issue from this PR).
- Reworking the dashboard layout / other widgets.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-action-queue-faked-api-calls.md`
- Mark the matching `backlog.json` row as `status: "done"`
