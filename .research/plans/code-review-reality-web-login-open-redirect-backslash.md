# code-review-reality-web-login-open-redirect-backslash

**Vector:** security
**Score:** 3
**Source:** Tier1d review 2026-08-12 (reality-web)
**Confidence:** high

## Hypothesis
The reality-web login page's `?redirect=` sanitizer at `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx:72` rejects `//` but accepts `/\` (backslash). Browsers normalize backslash to forward-slash in URL parsing, so `router.replace('/\\evil.com')` navigates to `//evil.com` — a protocol-relative URL pointing at an attacker origin. Sibling `auth/callback/page.tsx:117-122` already blocks this exact class explicitly, so the two flows have drifted. Post-login open-redirect is a phishing / credential-harvest vector; must fail closed against every path shape browsers normalize.

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx:72` — `const safe = redirectTo.startsWith('/') && !redirectTo.startsWith('//') ? redirectTo : '/'`
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.tsx:117-122` — sibling flow rejects both `//` and `/\` (documented anti-open-redirect guard from an earlier hardening).
- No test file exists at `frontend/apps/reality-web/src/app/[locale]/auth/login/page.test.tsx`; the callback flow ships a test suite (`callback/page.test.tsx`) that already codifies the redirect-sanitizer contract.
- Sibling `authCallbackHelpers` under `lib/` OR the callback page's helper is the natural extraction site.

## Files
- `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.tsx`

## Dependencies
_(none)_

## Required capabilities
- [x] C1 — Systematic debugging (security bug — required)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Serve reality-web (or run the vitest harness) and open `/[locale]/auth/login?redirect=/\evil.com`.
2. Log in with any valid credentials (or in a test: mock `auth-api::login` success + `useAuth().refreshSession`).
3. **Expected:** `router.replace('/')` (sanitizer strips backslash-prefixed target). **Actual (today, without the fix):** `router.replace('/\\evil.com')` → browser normalizes to `//evil.com` → external redirect to `evil.com`.

## Suggested approach
1. Extract a pure helper `isSafeInternalRedirect(path: string): boolean` into `frontend/apps/reality-web/src/lib/redirects.ts` (create the file). Contract: returns `true` iff `path.startsWith('/') && !path.startsWith('//') && !path.startsWith('/\\')`.
2. Import and use it in `login/page.tsx:72` — replace the inline check with `const safe = isSafeInternalRedirect(redirectTo) ? redirectTo : '/'`.
3. Refactor `callback/page.tsx:117-122` to consume the same helper — the two sanitizers must not drift again.
4. Add regression tests: (a) unit test on `isSafeInternalRedirect` covering `/`, `/foo`, `//evil`, `/\evil`, `https://evil`, empty string, and a percent-encoded `%2F%5Cevil.com` (URL-decoded to `/\evil.com`); (b) integration test `login/page.test.tsx` mirroring `callback/page.test.tsx` with parametrized redirect values, asserting `router.replace` is called with the sanitized value.
5. Verify: `cd frontend && pnpm --filter @ppt/reality-web typecheck && pnpm --filter @ppt/reality-web test`.

## Alternatives considered
- **Inline-fix only in login/page.tsx** — rejected because the drift will recur; the two flows must share the sanitizer.
- **Server-side redirect validation** — rejected because `router.replace` is a client-side navigation; adding a server hop would break the SPA UX and doesn't cover the raw `router.replace` call site.

## Root-cause trace
1. Symptom: after login with `?redirect=/\evil.com`, the browser navigates to `//evil.com` (an external origin).
2. ← `router.replace(safe)` at `login/page.tsx:73` passes an unnormalised path through.
3. ← `const safe = redirectTo.startsWith('/') && !redirectTo.startsWith('//') ? redirectTo : '/'` at `login/page.tsx:72` — the check omits the `/\` prefix branch that the browser then normalises.
4. Origin: the sanitizer was originally added on the login page as a two-clause check; the follow-up hardening that added the `/\` clause landed only on `callback/page.tsx` (see Git blame on `callback/page.tsx:117-122`), leaving login behind.

## Test plan
- [ ] Unit: `frontend/apps/reality-web/src/lib/redirects.test.ts` — parametrised over `['/', '/dashboard', '//evil', '/\\evil', 'https://evil', '', 'evil.com', '/foo/bar?x=1']`, asserts the helper's boolean.
- [ ] Integration: `frontend/apps/reality-web/src/app/[locale]/auth/login/page.test.tsx` — renders LoginForm, mocks `auth-api::login` + `useAuth().refreshSession`, drives a successful submit with `?redirect=/\evil.com` and asserts `router.replace` was called with `'/'`.
- [ ] Regression: the same integration test with `?redirect=/dashboard` must pass through unchanged (positive case).
- [ ] `cd frontend && pnpm --filter @ppt/reality-web test`

## Out of scope
- Changing the auth-context `refreshSession()` sequencing (unrelated; keep behaviour).
- Adding native translations for login page copy (tracked separately under `code-review-reality-web-agency-import-i18n` follow-ups).
- Backend URL validation for `/api/v1/auth` — this is a client-side navigation concern only.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-login-open-redirect-backslash.md`
- Mark the matching `backlog.json` row as `status: "done"`
- Also mark `code-review-reality-web-login-page-zero-tests` as `status: "done"` (this plan's tests resolve it)
