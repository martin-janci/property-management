# code-review-mobile-rn-messages-format-relative-invalid-date

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 mobile-rn segment review 2026-08-05
**Confidence:** high

## Hypothesis
`MessagesScreen.formatRelative(iso)` calls `new Date(iso).getTime()` without validating the input, so a malformed or missing `lastMessage.createdAt` produces `diffMs = NaN`. Every `< 60` / `< 24` / `< 7` comparison is false, execution falls through to `new Date(iso).toLocaleDateString('en-US', …)`, and the thread card renders the literal string `"Invalid Date"` next to the preview. PR #2659 just introduced the same guard (`formatMessageTime` returning `''` on NaN) one file over in `ThreadDetailScreen.tsx`; mirror it here so the thread list stops surfacing junk timestamps for any malformed `createdAt` the api-server or a stale cached response might send.

## Evidence
- `frontend/apps/mobile/src/screens/messages/MessagesScreen.tsx:53-62` — `formatRelative` derives `diffMs = Date.now() - new Date(iso).getTime()`; on a non-date `iso` (e.g. `'x'` or an empty string), `getTime()` is `NaN`, all subsequent `< N` predicates evaluate false, and the fallback `toLocaleDateString` renders `"Invalid Date"`.
- `frontend/apps/mobile/src/screens/messages/ThreadDetailScreen.tsx:104-124` (post PR #2659) — the sibling screen now uses `formatMessageTime()` which returns `''` when `Number.isNaN(new Date(sentAt).getTime())`. Same defensive pattern; not yet applied to the thread list.
- The api-server response is fetched via `useApiQuery` (`MessagesScreen.tsx:9`) and typed as `ThreadWithPreview.lastMessage?.createdAt: string`, but no runtime shape-guard sits between the wire response and `formatRelative` — the type declaration doesn't prevent stale-cache or backend regression from producing non-date strings.

## Files
- `frontend/apps/mobile/src/screens/messages/MessagesScreen.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. In a Jest test, construct a `ThreadListResponse` whose one `thread.lastMessage.createdAt` is the non-date string `'x'` (mirrors the exact fixture PR #2659 uses for `ThreadDetailScreen`).
2. Render `MessagesScreen` with that response mocked into `useApiQuery`.
3. Read back the meta text on the thread card. Expected: an empty string or a safe placeholder. Actual (today): the literal `"Invalid Date"` from `toLocaleDateString`.

## Suggested approach
1. Extract a `formatThreadMeta(iso: string | undefined): string` helper alongside `formatRelative` (same file). On `undefined`, empty string, or `Number.isNaN(new Date(iso).getTime())` return `''`; otherwise reuse the existing `formatRelative` body.
2. Replace the call site (currently `formatRelative(thread.lastMessage.createdAt)` in the thread-card row) with the new helper. Keep `formatRelative` for any purely-internal use, or fold the guard into `formatRelative` itself — the important thing is the boundary check happens before `getTime()`.
3. Export `formatThreadMeta` from `MessagesScreen.tsx` for unit testing, matching how PR #2659 exports `formatMessageTime`.
4. Add unit tests to `frontend/apps/mobile/src/screens/messages/MessagesScreen.test.tsx` (create if missing) covering: valid ISO → non-empty label without `"Invalid Date"`; `'x'` / `''` / `'not-a-date'` → `''`; a `< 60m` age → `'…m'` output; a `> 7d` age → the fallback date label.

## Alternatives considered
- **Fix inside `formatRelative` and keep one function** — rejected because the caller passes `iso: string` (non-optional) today; changing the signature bleeds `undefined` handling into every caller, whereas an explicit `formatThreadMeta` wraps the boundary check where it belongs and keeps `formatRelative`'s contract intact (matches the sibling file's split).
- **Guard the render site with a ternary (`typeof iso === 'string' && !isNaN(...) ? formatRelative(iso) : ''`)** — rejected because the guard duplicates on every future call and stays out of the unit-testable surface; the sibling file's own choice (a dedicated exported helper) proved better in PR #2659.

## Root-cause trace
1. Symptom: thread-card meta on `MessagesScreen` shows the literal `"Invalid Date"` when a message's `createdAt` isn't a parseable ISO string.
2. ← `MessagesScreen.tsx:61` — `new Date(iso).toLocaleDateString('en-US', {…})` runs unconditionally after all `< N` branches fall through.
3. ← `MessagesScreen.tsx:53-54` — `formatRelative` computes `diffMs = Date.now() - new Date(iso).getTime()`; on non-date `iso`, `getTime()` is `NaN`, and every `Math.floor(NaN / …)` is `NaN`, making every `< N` comparison false.
4. ← There is no runtime shape-guard between the wire response (`useApiQuery` at `MessagesScreen.tsx:9`) and the formatter — `MessagePreview.createdAt: string` is a compile-time claim, not a runtime check. Same latent gap PR #2659 fixed in `ThreadDetailScreen.tsx` before applying `sentAtSortKey` / `formatMessageTime`.
5. Origin: PR that first wired `formatRelative` in `MessagesScreen.tsx` — pre-existing since the screen was introduced; surfaced now by the paired defensive fix in PR #2659, which explicitly notes the pattern needs to be mirrored to sibling screens.

## Test plan
- [ ] `frontend/apps/mobile/src/screens/messages/MessagesScreen.test.tsx` — new file (or new `describe('formatThreadMeta')` block if the file exists) with a table-driven `it.each([['non-date string','x'],['empty','']])('returns \'\' for %s', …)` mirroring the PR #2659 pattern.
- [ ] Regression scenario: mount `MessagesScreen` with a mocked `ThreadListResponse` whose `lastMessage.createdAt='x'`; assert `screen.queryByText(/Invalid Date/)` is `null`.
- [ ] Run locally with: `cd frontend && pnpm --filter @ppt/mobile test -- MessagesScreen`

## Out of scope
- Broader i18n sweep of `MessagesScreen.tsx` (there are hardcoded strings like the empty-state copy; that's a separate refactor).
- Backend-side validation of `createdAt` in the messages route (this plan is a UI defensive guard; the wire type stays as declared).
- Any change to `ThreadDetailScreen.tsx` (already fixed by PR #2659).

## After-merge
- Move this file to `plans/_archive/code-review-mobile-rn-messages-format-relative-invalid-date.md`
- Mark `backlog.json` row `code-review-mobile-rn-messages-format-relative-invalid-date` as `status: "done"`
