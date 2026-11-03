# code-review-ppt-web-ui-aichat-first-msg-stale-sid

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review (dispatcher Tier-1d 2026-11-01 ppt-web-ui)
**Confidence:** medium

## Hypothesis
`ChatInterface.tsx` binds `useSendMessage` to the session id captured at render time (`useSendMessage(currentSessionId || '')` at line 48). When the user starts a brand-new chat with no `currentSessionId`, `handleSendMessage` creates a session, calls `setCurrentSessionId(session.id)`, then via a 100 ms `setTimeout` calls `sendMessage.mutateAsync(...)`. The setTimeout closes over the stale `sendMessage` instance — still bound to the empty string — so the first message of a new chat POSTs to `/ai/chat/sessions//messages` (empty session id) and silently fails. Smallest fix: thread the freshly-created `session.id` into the send call (e.g. make `useSendMessage` accept the id as a `mutate` argument, or wire a dedicated `sendMessageInSession(sessionId, content)` helper), and drop the setTimeout.

## Evidence
- `frontend/apps/ppt-web/src/features/ai-chat/components/ChatInterface.tsx:48` — `const sendMessage = useSendMessage(currentSessionId || '')`; the mutation hook is parameterised by the session id captured at render time.
- `frontend/apps/ppt-web/src/features/ai-chat/components/ChatInterface.tsx:95-120` (`handleSendMessage`) — on a new chat the code awaits `createSession.mutateAsync(...)`, calls `setCurrentSessionId(session.id)`, then `setTimeout(async () => { await sendMessage.mutateAsync({ content }) }, 100)`. The inline comment at :104-105 admits the shortcut: "This is a simplified version - in production you'd want better state management."
- The `setTimeout` is a timing race in addition to the stale-closure bug — a 100 ms gate on the hook re-render arriving before the timer fires.
- Impact: the happy-path first message of every brand-new AI chat silently 404s against `/ai/chat/sessions//messages`; the user types, nothing lands, no error surfaces until the mutation's `onError` runs — and `handleSendMessage` catches the throw with an empty `catch {}` (`:108-110`), so even that is swallowed.

## Files
- `frontend/apps/ppt-web/src/features/ai-chat/components/ChatInterface.tsx:48`
- `frontend/apps/ppt-web/src/features/ai-chat/components/ChatInterface.tsx:95`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** no C4/C5 ticks.

Mode: cloud-ok

## Repro steps
1. In a dev stack with ppt-web pointed at a running api-server, log in as a manager with no prior AI chat sessions.
2. Navigate to the AI chat view and type a first message in a brand-new session (no `initialSessionId`).
3. Submit the message. The UI appears idle; capture the network trace.
4. Expected after fix: a single `POST /ai/chat/sessions/<new-session-id>/messages` call to the just-created session, 201/200 response, assistant reply streams back. Today: the create-session POST succeeds, but the follow-up messages POST lands on `/ai/chat/sessions//messages` (empty id → 404) and the user sees nothing.

## Suggested approach
1. Open `ChatInterface.tsx` and change `useSendMessage` so the session id is a `mutate` argument, not a hook arg — new signature: `useSendMessage()` returning `{ mutateAsync({ sessionId, content }) }` (plumb `sessionId` into the POST URL inside the mutation). Alternately, if the hook must stay session-scoped, add a thin `postMessageToSession(sessionId, content)` imperative helper used only on the new-session branch.
2. In `handleSendMessage` (new-session branch, around :95-108), after `createSession.mutateAsync(...)` returns `session`, call the helper/mutation with `session.id` directly: `await sendMessage.mutateAsync({ sessionId: session.id, content })`. Delete the `setTimeout` wrapper.
3. Keep the existing-session branch unchanged (`currentSessionId` is already real).
4. Surface errors: replace the empty `catch {}` at :108-110 with the same error-handling pattern used elsewhere (sibling: `ChatInterface.tsx:254` already renders `sendMessage.isError`). Add an inline toast or the shared error renderer so a 4xx from the new flow is visible.
5. Verify callers of `useSendMessage` elsewhere in the feature compile; if any still pass a session id to the hook, adapt them to the new call shape.
6. Add an IG3 regression test: a Testing Library test for `ChatInterface` that mocks `useCreateSession`/`useSendMessage`, renders the component with no `initialSessionId`, types + submits one message, and asserts the mock `sendMessage.mutate` was called with the id returned from `createSession.mutate`.

## Alternatives considered
- **Add an effect that re-triggers the send once `currentSessionId` updates** — rejected because it introduces a second re-render + implicit timing dependency (same class of bug the current `setTimeout` is masking), and bloats the component state machine. Threading the id through the call is simpler and local.
- **Make `useSendMessage` recreate its mutation when the id changes and `useRef` the latest instance** — rejected because it keeps the hook signature session-scoped (which is the structural source of the bug) and still ships the stale-closure risk inside `handleSendMessage`'s `useCallback` deps.

## Root-cause trace
1. Symptom: first message of a brand-new AI chat silently fails; nothing visible in the UI, server never receives a `messages` POST for the freshly-created session id.
2. ← `ChatInterface.tsx:106` — the `setTimeout` body calls `sendMessage.mutateAsync({ content })` without passing the just-created session id.
3. ← `ChatInterface.tsx:48` — `sendMessage` is bound at render time to `currentSessionId || ''`, and the setTimeout callback closes over that stale instance; `setCurrentSessionId(session.id)` schedules a re-render but does not retroactively update the already-captured mutation.
4. ← The 100 ms `setTimeout` was added (comment at :104-105) as a workaround for the state-timing issue instead of threading the id directly.
5. Origin: initial scaffold of the AI-chat feature — the hook was parameterised by session id up front without a path for "send into the session you just created." No introducing PR is cited in the signal evidence; blame the file for the exact introducing commit if backfill is wanted.

## Test plan
- [ ] New RTL test in `frontend/apps/ppt-web/src/features/ai-chat/components/ChatInterface.test.tsx` (create if absent): `renders without initialSessionId → first send posts to the newly-created session id`. Mock `useCreateSession` to resolve `{ id: 'abc' }`, mock `useSendMessage` to return a tracked mock, type + submit, assert the mock was called with `{ sessionId: 'abc', content: '…' }` (and NOT with session id `''`).
- [ ] Regression scenario: existing-session branch still routes through the hook unchanged — a second test with `initialSessionId='existing'` asserts the send mock is called with that id.
- [ ] Local command: `pnpm --filter @ppt/web test src/features/ai-chat/components/ChatInterface` (and `pnpm --filter @ppt/web typecheck` before pushing).

## Out of scope
- Rewriting `useCreateSession`/`useSendMessage` beyond what's needed to pass `sessionId` through the mutation.
- Fixing the empty `catch {}` blocks elsewhere in `handleSendMessage`/`handleDeleteSession` beyond adding the visible error path for this flow.
- Streaming/token-level improvements to the AI chat path.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-ui-aichat-first-msg-stale-sid.md`
- Mark the matching `backlog.json` row as `status: "done"`
