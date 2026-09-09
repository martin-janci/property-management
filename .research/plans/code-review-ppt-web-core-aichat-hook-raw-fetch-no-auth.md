# code-review-ppt-web-core-aichat-hook-raw-fetch-no-auth

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review Tier-1d 2026-09-08 (ppt-web-core feature-hooks) + routine 2026-09-09 direct-read verification
**Confidence:** high

## Hypothesis
`frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.ts` defines a module-local `apiFetch<T>()` (lines 30-46) that calls global `fetch()` with only `Content-Type: application/json` and never attaches an `Authorization` header. Every AI-chat query and mutation (Epic 127) routes through `apiFetch` to `/api/v1/ai/chat/*`, so every request goes out unauthenticated → the server returns 401 → the AI-chat feature is broken end-to-end for authenticated sessions. The smallest fix is to swap `apiFetch` for the shared `getApiClient()` (like `features/sentiment/hooks/useSentiment.ts` and `features/predictive-maintenance/hooks/usePredictiveMaintenance.ts` already do), which routes through the axios interceptor stack that attaches the Bearer token, the `X-Tenant-ID` header, and the single-flight 401→refresh→replay recovery landed in PR #2942.

## Evidence
- `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.ts:30-46` — `apiFetch<T>()` sends only `{'Content-Type':'application/json', ...options?.headers}`; no Authorization; no interceptor path. Verified 2026-09-09 by direct file read.
- Callers routing through `apiFetch`: `useAiChatSessions` (L49-60), `useAiChatSession` (L63-72), `useAiChatMessages` (L75-88), `useCreateSession` (L91+), plus the remaining `useMutation` hooks — every AI-chat endpoint goes out unauthenticated.
- Sibling hooks done correctly: `frontend/apps/ppt-web/src/features/sentiment/hooks/useSentiment.ts:7,18` and `frontend/apps/ppt-web/src/features/predictive-maintenance/hooks/usePredictiveMaintenance.ts:5,9` both import `getApiClient` from `../../../lib/api` and carry the `Bearer-token` interceptor comment.
- The raw `fetch()` also bypasses the single-flight 401→refresh→replay path in `frontend/apps/ppt-web/src/lib/api.ts:249-270` (PR #2942) — so a token that expires mid-session yields a hard 401 instead of a transparent refresh.
- Same defect class already filed for `routes/groups/person-months.tsx` under `code-review-ppt-web-core-raw-fetch-bypasses-jwt-interceptor` (2026-08-29), but in a previously-unreported file.

## Files
- `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.ts`
- `frontend/apps/ppt-web/src/lib/api.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):** Mode: cloud-ok

## Repro steps
1. Log in to ppt-web as an authenticated manager; open DevTools Network tab.
2. Navigate to any AI-chat surface (Epic 127 UI — the chat sidebar or dedicated route).
3. Trigger any of: list sessions, open a session, send a message.
4. Observed: request goes to `/api/v1/ai/chat/*` with NO `Authorization` header (Bearer scheme missing) → response is HTTP 401 → UI shows a load/error state or empty list.
5. Expected: request includes the Bearer token via the shared axios interceptor → HTTP 200 → the chat renders.

## Suggested approach
1. Delete the module-local `apiFetch<T>()` helper in `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.ts:30-46`.
2. Import `getApiClient` from `../../../lib/api` at the top of the file (mirroring `features/sentiment/hooks/useSentiment.ts:7`).
3. Rewrite each `useQuery` / `useMutation` `queryFn`/`mutationFn` to call `getApiClient().<method>(<url>, <body?>)` — axios sets `Content-Type` for JSON automatically; the request interceptor attaches the `Authorization` header (Bearer scheme) and `X-Tenant-ID`.
4. Keep the existing TanStack Query keys (`aiChatKeys`) unchanged so cache invalidation still works.
5. Add a Vitest unit test under `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.test.ts` that mocks axios and asserts a `useAiChatSessions()` render triggers a `GET /api/v1/ai/chat/sessions` with the Bearer header attached (verify via the axios request interceptor mock).
6. Run `pnpm -F @ppt/web typecheck` and `pnpm -F @ppt/web test` locally; then `pnpm -F @ppt/web check`.

## Alternatives considered
- **Inject a Bearer-scheme `Authorization` header via `${tokenStorage.getAccessToken()}` directly inside `apiFetch`** — rejected because it duplicates the interceptor stack the rest of the app already relies on: it wouldn't get the `X-Tenant-ID` header (AuthContext `setOrgProvider`, #1522), wouldn't participate in the single-flight 401→refresh→replay recovery, and wouldn't get the transient-error retry/backoff.
- **Move `apiFetch` into `lib/api.ts` as a fetch-based sibling of `getApiClient`** — rejected because two API-client paths in one app is what created this bug; convergence on `getApiClient()` is cheaper long-term than divergence with a second interceptor stack.

## Root-cause trace
1. Symptom: AI-chat surface (Epic 127) fails to load / send messages in an authenticated session.
2. ← `apiFetch<T>()` at `useAiChat.ts:30-46` calls `fetch()` without an Authorization header.
3. ← `apiFetch` was introduced as a feature-local helper instead of importing the shared `getApiClient` used by sibling feature hooks.
4. Origin: initial Epic 127 scaffold (introduced with the AI-chat feature branch) — the sibling `features/sentiment` and `features/predictive-maintenance` hooks that took the correct path were added later and did not retrofit AI-chat.

## Test plan
- [ ] `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.test.ts` — new unit test asserts `useAiChatSessions()` triggers a `GET /api/v1/ai/chat/sessions` whose request carries a Bearer-scheme `Authorization` header (mocked axios interceptor).
- [ ] Regression: with the fix reverted, the new test must fail (IG3).
- [ ] Run `pnpm -F @ppt/web test -- useAiChat` locally, then `pnpm -F @ppt/web typecheck` and `pnpm -F @ppt/web check`.

## Out of scope
- OCR meter-reading hook (`useOcrMeterReading.ts`) — same root cause but a separate backlog row (`code-review-ppt-web-core-ocr-hook-raw-fetch-no-auth`) with its own plan.
- Any change to `lib/api.ts` axios interceptor stack itself.
- Broader Epic 127 API surface changes.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-aichat-hook-raw-fetch-no-auth.md`
- Mark the matching `backlog.json` row as `status: "done"`
