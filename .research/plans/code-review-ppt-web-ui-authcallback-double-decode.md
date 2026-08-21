# code-review-ppt-web-ui-authcallback-double-decode

**Vector:** bug
**Score:** 3
**Source:** code-review-ppt-web-ui 2026-08-21 rotating review
**Confidence:** high

## Hypothesis
`AuthCallbackPage.tsx:100` wraps the SSO provider's `error_description` query param in `decodeURIComponent(...)`, but `useSearchParams()` / `URLSearchParams.get()` already URL-decodes the value. On any provider payload containing a literal `%` after the first decode (e.g. `error_description=Consent%20100%25%20denied` → `100% denied` → then `%20d` decoded, but `%2` is *not* followed by two hex digits after the leading trim), the second `decodeURIComponent` throws `URIError` inside the `useEffect` callback. There is no `try/catch` on this branch, so the effect aborts before `setStatus('error')` completes — the callback screen stays on the "pending" spinner forever and the user is stuck. Dropping the redundant decode (or gating it with a `try/catch` fallback to the raw string) resolves the issue in one line while keeping the visible error path intact.

## Evidence
- `frontend/apps/ppt-web/src/pages/AuthCallbackPage.tsx:96-104` — the offending `errorParam` branch wraps `errorDescription` in `decodeURIComponent(...)`, but the value came from `useSearchParams()` which decodes once already.
- MDN: "URLSearchParams.prototype.get() — The value returned is already URL-decoded." Calling `decodeURIComponent` a second time on any value carrying a literal `%` yields `URIError: URI malformed`.
- The same file's non-error path (line 118) does *not* wrap `code`/`state` in `decodeURIComponent`, which is the correct pattern and confirms the double-decode on the error branch is a stray.
- Rotating expert review (Phase 1.5) of the `ppt-web-ui` segment on 2026-08-21 surfaced this as its highest-severity cite-able finding.

## Files
- `frontend/apps/ppt-web/src/pages/AuthCallbackPage.tsx:100`

## Dependencies
<!-- No blocking dependencies. -->

## Required capabilities
- [x] C1 — Systematic debugging (bug vector, reproduce URIError deterministically)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. In a Vitest / React Testing Library test, render `<AuthCallbackPage />` inside a `MemoryRouter` at path `/auth/callback?error=access_denied&error_description=Consent%20100%25%20denied` (URL-encoded `%`), then wait for the effect.
2. Expected: the page renders the error state with the decoded message "Consent 100% denied".
3. Actual (pre-fix): the effect throws `URIError: URI malformed` while running `decodeURIComponent`, the exchange effect aborts, and `status` stays `'pending'` (the loading spinner stays on screen indefinitely). Under `renderHook` you also see the uncaught rejection in the test log.

## Suggested approach
1. In `frontend/apps/ppt-web/src/pages/AuthCallbackPage.tsx:100`, drop the outer `decodeURIComponent(...)` and use `errorDescription` directly: the value is already decoded once by `URLSearchParams`.
2. Optionally, if the codebase's SSO provider ever double-encodes intentionally (it does not today), wrap the read in a helper `safeDecode(v: string): string` that returns `try { decodeURIComponent(v) } catch { return v }` — but do *not* apply it inline in this branch.
3. Add a Vitest case under `frontend/apps/ppt-web/src/pages/__tests__/` (create a new spec if the folder is empty) that mounts `<AuthCallbackPage />` with `?error=access_denied&error_description=Consent%20100%25%20denied` and asserts (a) the loading state resolves to error within one tick and (b) the visible error message contains `100%`.
4. Regression-test the happy error path too: `?error=access_denied&error_description=Consent+denied` should still render "Consent denied" verbatim.
5. Run `pnpm -F ppt-web test -- AuthCallbackPage` and confirm both cases pass; then `pnpm check && pnpm typecheck`.

## Alternatives considered
- **Wrap the second decode in `try/catch` and fall back to the raw string** — rejected because it silently converts a genuine bug into a partial-decode masquerade; if a provider ever *does* double-encode, the raw string leaks percent-escape noise to the user. The right shape is "decode zero-or-one times, not two-or-more".
- **Re-encode the URL before `URLSearchParams` parses it** — rejected because it inverts the wrong side of the pipeline; `URLSearchParams` is doing exactly what the spec asks, and re-encoding in the caller adds a layer that must be maintained.

## Root-cause trace
1. Symptom: SSO callback screen stuck on the loading spinner forever when the provider returns `error_description` containing a literal `%` (URL-encoded as `%25`).
2. ← `decodeURIComponent` at `frontend/apps/ppt-web/src/pages/AuthCallbackPage.tsx:100` throws `URIError` on already-decoded input.
3. ← The `errorParam` branch of the exchange effect (`AuthCallbackPage.tsx:96-104`) has no `try/catch`, so the throw escapes the effect callback and prevents `setStatus('error')` / `setErrorMessage(...)` from firing.
4. Origin: the double-decode has been present since the SSO callback page was added; no recent PR introduced it — this is a latent bug that a rotating review surfaced on 2026-08-21. A `git log -p -- frontend/apps/ppt-web/src/pages/AuthCallbackPage.tsx | grep -n decodeURIComponent` will point at the introducing commit for the changelog entry.

## Test plan
- [ ] New Vitest spec: `frontend/apps/ppt-web/src/pages/__tests__/AuthCallbackPage.test.tsx` — covers the `%25` payload case (fails on pre-fix code).
- [ ] Regression scenario: the same spec also asserts the plain-`+` payload still renders the decoded string.
- [ ] Command: `pnpm -F ppt-web test -- AuthCallbackPage.test.tsx` (or `pnpm -F ppt-web test`).

## Out of scope
- Reworking the SSO callback state machine (retry, in-flight token exchange guards).
- Introducing a `safeDecode` helper across the codebase — call-sites elsewhere already handle their own decoding correctly, so this stays a single-file fix.
- i18n rework of the error-copy strings.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-ui-authcallback-double-decode.md`
- Mark the matching `backlog.json` row as `status: "done"`
