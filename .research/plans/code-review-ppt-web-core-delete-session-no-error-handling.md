# code-review-ppt-web-core-delete-session-no-error-handling

**Vector:** bug
**Score:** 3
**Source:** code-review-ppt-web-core 2026-07-01
**Confidence:** high

## Hypothesis
`useDeleteSession()` in `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.ts` fires `DELETE ${API_BASE}/sessions/${sessionId}` with a bare `fetch()`, never inspects `response.ok`, and unconditionally runs `queryClient.invalidateQueries({ queryKey: aiChatKeys.sessions() })` in `onSuccess`. The UI reads "session removed" for any 4xx/5xx (auth failure, tenant guard, backend down), and the invalidated cache re-fetches the still-present session with no error surface. Users end up staring at a session that reappears after the "delete" spinner clears, with no explanation.

## Evidence
- `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.ts:130-142` — `useDeleteSession` body: `await fetch(...)` then `onSuccess: queryClient.invalidateQueries(...)`. No `response.ok` check, no `throw`, no `onError`.
- Compare with the sibling `useMessageFeedback` in the same file (`useAiChat.ts:145-159`) which routes through `apiFetch<void>` — the file-local wrapper (defined at `useAiChat.ts:31`) that throws on non-2xx.
- `apiFetch<T>` is defined at `useAiChat.ts:31` and used at lines 53, 68, 80, 96, 113, 148, 165 — every other mutation/query in the file goes through it. Only `useDeleteSession` bypasses it.
- Symptomatic UX: no toast, no re-render of an error state, cache invalidation always fires.

## Files
- `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode:** cloud-ok
Mode: cloud-ok

## Repro steps
1. In `useAiChat.ts` `useDeleteSession`, temporarily point `API_BASE` at a URL that returns 500 for `DELETE /sessions/:id` (or block the endpoint with a middleware that forces 500 for one path).
2. From the ppt-web AI chat panel, click "delete session" on any existing session.
3. **Observed:** spinner clears; the session list re-fetches and the session reappears with no error toast. `queryClient.invalidateQueries` fired even though the delete failed on the server.
4. **Expected:** an error toast surfaces ("Failed to delete session"), the list is not invalidated on failure, and the mutation's `isError` branch is reachable so the caller can render a retry affordance.

## Suggested approach
1. Replace the raw `fetch` in `useDeleteSession` (`useAiChat.ts:135`) with `apiFetch<void>` (same file-local helper at `useAiChat.ts:31`, used by every other mutation in the file). `apiFetch` throws on non-2xx, which flips `useMutation` into its `isError` state and blocks the `onSuccess` invalidation.
2. Confirm `apiFetch` accepts `{ method: 'DELETE' }` for a body-less request (it forwards `options` verbatim to `fetch`, so DELETE without body is fine).
3. In the caller component that renders session actions (grep for `useDeleteSession()` under `frontend/apps/ppt-web/src`), surface an error toast on `mutation.isError` — reuse the same toast helper used by other AI-chat mutations. If none exists yet, wire it in this PR's out-of-scope note.

## Alternatives considered
- **Keep the raw fetch and just check `response.ok` inline** — rejected because `apiFetch` already centralises auth-token attachment, base-URL resolution, and error mapping; hand-rolling a second error path in this one hook drifts the codebase from the standard.
- **Add an `onError` callback that shows a toast** — rejected as *sufficient but not root-cause*: leaves the `onSuccess` invalidation reachable when the API rejects auth, so we'd still be reasoning about "did the fetch succeed" in two places.

## Root-cause trace
1. Symptom: user clicks Delete, spinner clears, session reappears, no error surface.
2. ← `useAiChat.ts:140` `onSuccess` fires unconditionally because the mutation's `mutationFn` returned success (bare `fetch()` resolves on any HTTP status).
3. ← `useAiChat.ts:136` `await fetch(...)` — no `.ok` check, no `throw`.
4. Origin: initial scaffold of the AI-chat hooks (predates `apiFetch` wrapper adoption in the rest of the file). The wrapper landed later; `useDeleteSession` was missed in the sweep.

## Test plan
- [ ] Vitest unit test in `frontend/apps/ppt-web/src/features/ai-chat/hooks/__tests__/useAiChat.test.ts` (create if absent): mock `apiFetch` to reject; assert `useDeleteSession()` mutation lands in `isError`, and `queryClient.invalidateQueries` was NOT called for `aiChatKeys.sessions()`.
- [ ] Regression scenario: on 2xx, `invalidateQueries` still fires exactly once (proves the happy path is intact).
- [ ] Local: `cd frontend && pnpm --filter @ppt/web test -- useAiChat`

## Out of scope
- Adding a global toast helper if one doesn't exist. Note it as a follow-up; this plan uses whatever exists.
- Reworking other bare-`fetch` sites elsewhere in ppt-web (own audit).

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-delete-session-no-error-handling.md`
- Mark the matching `backlog.json` row as `status: "done"`
