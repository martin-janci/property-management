# code-review-ppt-web-core-useaichat-unauthed

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 review of ppt-web-core segment (2026-07-05)
**Confidence:** high

## Hypothesis

The `useAiChat.ts` hook family builds every AI-chat request through a local `apiFetch<T>()` helper (line 31) that sets `Content-Type: application/json` and nothing else — the JWT bearer token is never attached. Every call to `/api/v1/ai/chat/*` (list/create/delete sessions, send message, feedback, escalation) therefore reaches the api-server without `Authorization`, will 401, and the UI silently surfaces "Request failed / HTTP error 401" through TanStack Query. `useDeleteSession` (line 134) compounds the problem: it calls `fetch(...)` directly, never checks `response.ok`, and unconditionally invalidates the sessions cache — so a 401/500 delete looks like a success. The fix is to route these calls through the same `readAccessToken()` + bearer-in-`Authorization` pattern already used by `useNotificationAnalytics.ts` / `authApiClient.ts`, and to gate `useDeleteSession`'s `onSuccess`-equivalent on `response.ok`.

## Evidence

- `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.ts:31-46` — local `apiFetch<T>()` reads no token from `localStorage` / `@ppt/api-client`; every mutation and query below routes through it.
- `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.ts:134-141` — `useDeleteSession` calls `fetch(...)` directly, does not check `response.ok`, then fires `queryClient.invalidateQueries` unconditionally.
- Contrast pattern (works): `frontend/apps/ppt-web/src/features/notification-analytics/hooks/useNotificationAnalytics.ts:30-45` — reads `ppt_access_token` from `localStorage` and sets the `Authorization` header to `Bearer <token>` before `fetch`.
- Contrast pattern (works): `frontend/apps/ppt-web/src/features/auth/authApiClient.ts:13-25` — `readAccessToken()` helper + `createAuthApi({ accessToken: … })` from `@ppt/api-client`.

## Files

- `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.ts`
- `frontend/apps/ppt-web/src/features/notification-analytics/hooks/useNotificationAnalytics.ts`
- `frontend/apps/ppt-web/src/features/auth/authApiClient.ts`

## Dependencies

<!-- none -->

## Required capabilities

- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. `pnpm -F ppt-web dev` (or hit any deployed ppt-web instance) with a logged-in user.
2. Open the AI chat panel; observe DevTools → Network on the first `GET /api/v1/ai/chat/sessions` call. The `Authorization` header is absent; the response is `401 Unauthorized`.
3. Expected: request carries the `Authorization` header with a bearer JWT and returns `200` with the sessions list. Actual: no header, 401, empty list surfaced through TanStack Query's `error` state.

## Suggested approach

1. In `useAiChat.ts`, hoist the same `readAccessToken()` + `ACCESS_TOKEN_KEY` pattern already used by `authApiClient.ts` / `useNotificationAnalytics.ts` into a small local helper (or import `readAccessToken` from a shared `lib/auth-token.ts` if one exists — grep for `ppt_access_token`).
2. Rewrite `apiFetch<T>()` (line 31) to build a `Headers` object, attach the `Authorization` header with a `Bearer <token>` value when the token is present, and preserve the existing `Content-Type: application/json` + caller-supplied `headers` merge.
3. Refactor `useDeleteSession` (line 134) to use the same `apiFetch` helper (so it inherits auth) OR keep the raw `fetch` but check `response.ok` and throw a typed error before falling through to `onSuccess` / `invalidateQueries`.
4. Confirm each of the ~7 hooks (`useAiChatSessions`, `useCreateSession`, `useDeleteSession`, `useAiChatMessages`, `useSendMessage`, `useSendFeedback`, `useEscalatedList` / any others in the file) still compiles and returns the same types.
5. Add a Vitest that stubs `localStorage.getItem('ppt_access_token')` and mocks `global.fetch`, asserting each hook's request carries an `Authorization` header equal to `Bearer test-token`.

## Alternatives considered

- **Route through the generated `@ppt/api-client`** — rejected because the AI chat surface has no TypeSpec / generator coverage today (grep confirms no `AiChat*` symbols in `frontend/packages/api-client/src/generated`). Adding the TypeSpec is a separate, larger change and would balloon this fix beyond one file.
- **Add a global fetch interceptor via TanStack Query default `queryFn`** — rejected because ppt-web already has multiple auth-fetch patterns (auth client, per-feature helpers) and swapping in a global would drag every un-tested feature through a behavior change. The minimal, contained fix is one file.

## Root-cause trace

1. Symptom: AI chat panel shows "Request failed" for every call; DevTools shows no `Authorization` header on `/api/v1/ai/chat/*`.
2. ← Immediate cause: `apiFetch()` at `features/ai-chat/hooks/useAiChat.ts:31` builds `headers: { 'Content-Type': 'application/json', ...options?.headers }` with no bearer token attachment.
3. ← Upstream cause: the ai-chat feature was authored without following the `authApiClient.ts` / `useNotificationAnalytics.ts` pattern for token attachment; there is no shared `authenticatedFetch()` helper, so each new feature re-implements (or omits) auth.
4. Origin: the file was introduced whole (git blame the entire file), so this is a debut-time defect rather than a regression.

## Test plan

- [ ] Add `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.test.ts` (Vitest) — mock `localStorage` to return `'ppt_access_token' → 'jwt-fixture'`, mock `global.fetch`, render each hook (via `@tanstack/react-query`'s `QueryClientProvider` + `renderHook`), and assert the request's `Authorization` header equals `Bearer jwt-fixture`.
- [ ] Case: token missing → hook still fires but omits `Authorization` (no crash); the api-server response is surfaced as-is (401).
- [ ] Case: `useDeleteSession` — mock `fetch` to return `{ ok: false, status: 500 }`; assert the mutation's `error` fires and `queryClient.invalidateQueries` was NOT called.
- [ ] Local command: `pnpm -F ppt-web test src/features/ai-chat/hooks/useAiChat.test.ts`.

## Out of scope

- Adding TypeSpec + regenerating `@ppt/api-client` for the `/api/v1/ai/chat/*` surface. Track separately.
- Introducing a project-wide `authenticatedFetch()` helper. Track separately once the pattern has ≥3 duplicated callers to consolidate.

## After-merge

- Move this file to `plans/_archive/code-review-ppt-web-core-useaichat-unauthed.md`
- Mark the matching `backlog.json` row as `status: "done"`
