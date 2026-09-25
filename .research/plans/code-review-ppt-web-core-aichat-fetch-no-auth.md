# code-review-ppt-web-core-aichat-fetch-no-auth

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-09-24 (dispatcher Tier-1d, ppt-web-core, ai-chat segment)
**Confidence:** high

## Hypothesis
The AI-chat feature in ppt-web (Epic 127) is completely non-functional for authenticated users because `useAiChat.ts` calls a home-grown `apiFetch()` helper that never attaches an ``Authorization` header (Bearer scheme)` header. The shared axios client in `lib/api.ts` owns bearer injection (from `localStorage`) plus the 401→refresh replay; the bare `fetch()` in this hook bypasses it, so every one of the six AI-chat endpoints returns 401. Routing all AI-chat calls through `getApiClient()` restores auth (bearer, refresh, ErrorResponse→ApiError, retry) and pulls the file into the same auth transport the sibling `useSentiment.ts` uses. A second, distinct silent-failure bug in the same file — `useDeleteSession` uses `fetch()` directly with no headers and no `response.ok` check, so a failed DELETE reports success — is folded into the same fix.

## Evidence
- `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.ts:30-46` — `apiFetch()` calls `fetch(url, { headers: { 'Content-Type': 'application/json', ...options?.headers } })` with no `Authorization` header. All six hooks (`useAiChatSessions`, `useAiChatSession`, `useAiChatMessages`, `useCreateSession`, `useSendMessage`, `useMessageFeedback`, `useEscalatedMessages`) route through it against `API_BASE = '/api/v1/ai/chat'` (:19), which requires JWT auth on api-server.
- `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.ts:131-142` — `useDeleteSession` calls raw `fetch(${API_BASE}/sessions/${sessionId}, { method: 'DELETE' })` at :136 with no headers **and** no `response.ok` check, so a failed 401/403/network error resolves as a successful mutation and the UI reports the session deleted when it was not.
- `frontend/apps/ppt-web/src/features/sentiment/hooks/useSentiment.ts:17-20,35-40` — the correct in-repo pattern: routes through `getApiClient()` so "the shared axios interceptors apply: Bearer-token injection, ErrorResponse → ApiError, 401 → onUnauthorized, and transient 5xx/429 retry with backoff".
- `frontend/apps/ppt-web/src/features/notification-analytics/hooks/useNotificationAnalytics.ts:7-9,35-41` — the manual-attach fallback pattern: reads `ppt_access_token` from `localStorage` and adds ``Authorization` header (Bearer scheme) ${token}` with the comment "the handler requires authentication — a bare fetch would 401".
- Same defect class as backlog `code-review-ppt-web-core-raw-fetch-bypasses-jwt-interceptor` (status `done`, 2026-08-29 fix for `person-months` + `useBuildingUnits`); the fix never touched this file, so the class recurred here.

## Files
- `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.ts`

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
1. Sign into ppt-web against api-server (localStorage carries `ppt_access_token`).
2. Open any AI-chat surface (`useAiChatSessions` on the AI-chat page) and inspect the network tab.
3. **Expected:** the request to `/api/v1/ai/chat/sessions?...` carries ``Authorization` header (Bearer scheme) <jwt>` and returns 200.
   **Actual:** the request goes out with only `Content-Type: application/json`; api-server returns 401 and the sessions list stays empty. Same failure repeats for `useCreateSession` (POST `/sessions`), `useSendMessage`, `useMessageFeedback`, `useDeleteSession` (DELETE — no ok-check, so UI still reports success), and `useEscalatedMessages`.

## Suggested approach
1. Add `import { getApiClient } from '../../../lib/api';` and remove the home-grown `apiFetch<T>()` helper at `useAiChat.ts:30-46`.
2. Rewrite each `queryFn` / `mutationFn` in `useAiChat.ts` to call `getApiClient().get<T>(path)` / `.post<T>(path, body)` / `.delete(path)` etc., returning `res.data`. Keep the `AiChatKeys` factory and hook signatures unchanged so callers do not have to move.
3. For `useDeleteSession` (:131-142): replace the raw `fetch(...)` with `await getApiClient().delete(${API_BASE}/sessions/${sessionId})` — axios rejects on non-2xx by default, so the mutation now correctly enters `onError` on failure and the UI can render the error.
4. Keep `API_BASE = '/api/v1/ai/chat'` relative to the axios `baseURL` (already `/api/v1` per `lib/api.ts:75-77`) — trim the redundant `/api/v1` prefix so the effective path stays the same.
5. Update the file's top-of-module doc comment to name `getApiClient()` as the auth transport, mirroring the `useSentiment.ts:17-20` comment.
6. Add a Vitest that stubs `axios` (or the shared client), calls `useAiChatSessions`/`useCreateSession`/`useDeleteSession`, and asserts (a) each outbound request carries an ``Authorization` header (Bearer scheme) <token>` header, (b) `useDeleteSession` surfaces the mutation error when the DELETE returns 401.
7. Run `pnpm --filter @ppt/web typecheck` and `pnpm --filter @ppt/web test` locally; run Biome (`pnpm check`) before pushing.

## Alternatives considered
- **Manual-attach fallback like `useNotificationAnalytics.ts`** — rejected because it duplicates the auth transport per-file, misses 401→refresh replay, misses transient-error retry, and future token-rotation changes have to be repeated everywhere. The archived fix for `person-months` also chose `getApiClient()` for the same reason.
- **Keep the `apiFetch` helper and inject the bearer token inside it** — rejected because it does not fix `useDeleteSession` (a separate raw `fetch` outside the helper) and it re-implements the axios interceptor already tested by `lib/api.ts`. Half a fix invites the third recurrence.

## Root-cause trace
1. Symptom: authenticated users see empty AI-chat lists / hanging mutations / silently "successful" deletes — the whole Epic-127 feature dead on arrival.
2. ← `useAiChat.ts:32` — `fetch(url, { headers: { 'Content-Type': 'application/json', ...options?.headers } })` — no `Authorization` header appended, so api-server returns 401 on every call.
3. ← `useAiChat.ts:19,45` — the module ignores the shared axios client at `lib/api.ts:322` (`getApiClient()`), where the request interceptor injects ``Authorization` header (Bearer scheme) <token>` from the token getter wired at app boot.
4. ← `useAiChat.ts:136` — same raw-`fetch` shortcut for DELETE, with the extra defect of no `response.ok` check, so a 401/403/network error is a silent success.
5. Origin: `useAiChat.ts` was authored for Epic 127 (AI Chatbot Interface) as a standalone hook file without adopting the codebase's shared auth transport; the sister fix for `useBuildingUnits`/`person-months` (backlog `code-review-ppt-web-core-raw-fetch-bypasses-jwt-interceptor`, `status: done`, 2026-08-29) resolved the same class in one file but did not sweep sibling features. `getApiClient()` is a project-wide invariant: any hook that hits an authenticated `/api/v1/*` endpoint must route through it (or replicate the interceptor manually, per `useNotificationAnalytics`).

## Test plan
- [ ] `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.test.tsx` — RTL/Vitest: renders each hook against a mocked axios client, asserts the outbound request carries ``Authorization` header (Bearer scheme) <token>` and that a rejected DELETE surfaces the error to `onError`.
- [ ] Regression: verify the sibling `useNotificationAnalytics` / `useSentiment` transports still behave as before (should be untouched).
- [ ] Command: `pnpm --filter @ppt/web test -- useAiChat` and `pnpm --filter @ppt/web typecheck`.

## Out of scope
- Any change to `lib/api.ts` or the axios client itself (already-established transport).
- The parallel `useOcrMeterReading.ts` raw-fetch issue (tracked separately as backlog `code-review-ppt-web-core-ocr-fetch-no-auth`) — landing it in this PR would widen the diff beyond one feature.
- Any UI/UX change to the AI-chat pages — this plan is strictly the network transport.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-aichat-fetch-no-auth.md`
- Mark the matching `backlog.json` row as `status: "done"`
