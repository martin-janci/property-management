# code-review-reality-web-login-open-redirect-backslash

**Vector:** security
**Score:** 2
**Source:** reality-web segment review 2026-10-01 (oldest-unreviewed; cursor 2026-08-29)
**Confidence:** high

## Hypothesis
The reality-web `/[locale]/auth/login` form's post-login redirect validator only rejects URLs starting with `//`, but accepts `/\` (backslash-prefix). Browsers normalize `/\host.example.com` into a protocol-relative URL, so the client-side `router.replace(safe)` sends the just-authenticated user off-site to attacker-controlled infrastructure, enabling credential/phishing follow-ups. The sibling SSO callback (same app) already guards against both `//` and `/\` — porting that guard onto the login flow and adding the regression test the callback already carries closes the gap.

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx:72` — `const safe = redirectTo.startsWith('/') && !redirectTo.startsWith('//') ? redirectTo : '/';` — no backslash check.
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.tsx:127-131` — the SSO callback uses the full guard: `rawRedirect.startsWith('/') && !rawRedirect.startsWith('//') && !rawRedirect.startsWith('/\\')`.
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.test.tsx:166` — regression test `"rejects a backslash-escaped redirect_uri (open-redirect guard)"` exists for the callback path, nothing analogous for the login path.
- Login page imports `useRouter` from `next/navigation` (page.tsx:20), so navigation goes through the Next.js client router unchanged — no server-side policy intercepts the backslash URL.

## Files
- `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx:72`
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.tsx:127`
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.test.tsx:166`

## Dependencies

(none)

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Open the reality-web dev build locally (or any deployed instance) at `/sk/auth/login?redirect=%2F%5Cevil.example.com` — the URL encodes `/\evil.example.com`.
2. Submit valid credentials.
3. Expected: after login the user lands on `/` (safe fallback).
4. Observed (today): `router.replace('/\\evil.example.com')` runs; the browser interprets the backslash as a slash and navigates to `//evil.example.com`, i.e. `https://evil.example.com` — an open redirect from an authenticated session.

## Suggested approach
1. In `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx:72`, replace the inline `safe` computation with the same three-clause guard the callback uses: `const isSafeRedirect = typeof redirectTo === 'string' && redirectTo.startsWith('/') && !redirectTo.startsWith('//') && !redirectTo.startsWith('/\\');` then branch `router.replace(isSafeRedirect ? redirectTo : '/')`.
2. Factor the guard into a tiny helper (e.g. `frontend/apps/reality-web/src/lib/safeRedirect.ts` → `export function isSafeRelativeRedirect(value: unknown): value is string`) and consume it from both the login page and the callback page (removing the duplicated inline check at `callback/page.tsx:127-131`). The helper has no runtime dependencies, so it is safe to share.
3. Add a unit test `frontend/apps/reality-web/src/lib/safeRedirect.test.ts` that covers: safe `/dashboard`, unsafe `//evil`, unsafe `/\evil`, unsafe `http://evil`, non-string input, empty string. Mirror the matrix from `callback/page.test.tsx`.
4. Add a login-flow regression test `frontend/apps/reality-web/src/app/[locale]/auth/login/page.test.tsx` that mounts the login page with `?redirect=/\\evil.example.com`, submits valid credentials via a mocked `login()`, and asserts `router.replace` is called with `'/'`, not `'/\\evil.example.com'`.
5. Run `pnpm -F @ppt/reality-web test` and ensure the new tests fail on `main` and pass with the fix.
6. Lint with `pnpm check` to keep Biome quiet.

## Alternatives considered
- **Server-side middleware check on the Next.js middleware** — rejected because the redirect is applied purely client-side after `login()` succeeds; a middleware check cannot see the post-login navigation and would need a new round-trip to validate.
- **Allow-list of exact safe paths (`/dashboard`, `/favorites`, `/account`)** — rejected because the app genuinely uses deep-link redirects (e.g. `/listings/123`), and an allow-list would over-restrict legitimate flows. The three-clause relative-path guard is the standard and matches the callback.

## Root-cause trace
1. Symptom: `router.replace('/\evil.example.com')` navigates to `https://evil.example.com` after login because browsers normalize `\` to `/` in URL paths, producing a protocol-relative URL.
2. ← Immediate cause at `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx:72` — the `safe` guard only tests `startsWith('//')`, not `startsWith('/\\')`.
3. ← Upstream cause: the callback page introduced the correct three-clause guard (callback/page.tsx:127-131) but the login page was never updated in lock-step; there is no shared helper, so the two paths drifted.
4. Origin: the backslash variant of the open-redirect guard was added only to the SSO callback (callback/page.tsx + callback/page.test.tsx) during the auth-callback hardening wave; the login form predates that wave and still carries the two-clause check from the original implementation.

## Test plan
- [ ] `frontend/apps/reality-web/src/lib/safeRedirect.test.ts` — unit test for the helper, covering safe + both unsafe-prefix cases.
- [ ] `frontend/apps/reality-web/src/app/[locale]/auth/login/page.test.tsx` — regression: `?redirect=/\evil.example.com` must resolve to `router.replace('/')`, NOT `router.replace('/\\evil.example.com')`.
- [ ] `pnpm -F @ppt/reality-web test` — run locally; new tests fail without the login-page fix.

## Out of scope
- Server-side `Location` header validation on the API (api-server has its own redirect validator; not affected by this bug).
- Other `router.replace` call sites inside reality-web that take user input (none observed in reality-web; a separate audit if raised in a follow-up).
- Refactoring the broader auth i18n routing to use the locale-aware router (`@/i18n/routing`) — the callback already migrated; the login page can move in a later PR without blocking this fix.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-login-open-redirect-backslash.md`
- Mark the matching `backlog.json` row as `status: "done"`
