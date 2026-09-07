# code-review-reality-web-login-open-redirect-backslash

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review (dispatcher Tier-1d 2026-09-05 reality-web auth)
**Confidence:** high

## Hypothesis
The email/password login page reads its post-login destination from a user-controlled `redirect` query param and validates only two attack shapes (absolute URLs and protocol-relative `//evil.com`). The backslash-escape variant `/\evil.com` bypasses the guard because browsers normalise `\` to `/`, so `router.replace('/\evil.com')` resolves to a protocol-relative navigation to an attacker origin — a post-auth open redirect used for phishing. The sibling OAuth-callback page already carries the correct three-way guard and its test suite, so the fix is a shared helper wired into both pages plus mirrored test coverage.

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx:33,72-73` — reads `searchParams.get('redirect')` and after successful login runs `router.replace(redirectTo.startsWith('/') && !redirectTo.startsWith('//') ? redirectTo : '/')`; missing the `!startsWith('/\\')` clause.
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.tsx:116-125` — the sibling page's three-clause guard (`.startsWith('/') && !.startsWith('//') && !.startsWith('/\\')`).
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.test.tsx:122-135` — the canonical open-redirect test cases including "rejects a backslash-escaped redirect_uri (`/%5Cevil.example.com`)".
- Impact: any anonymous visit to `/{locale}/auth/login?redirect=/%5Cevil.example.com` bounces the user to `evil.example.com` immediately after successful login — phishing primitive on the primary auth flow across every tenant host.
- Trace: user-controlled `searchParams.get('redirect')` → `redirectTo` string → single-clause `startsWith` guard → `router.replace(safe)` — no decoding, no allow-listing, no shared helper.

## Files
- `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.test.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**

Mode: cloud-ok

## Repro steps
1. Start reality-web (`pnpm -F reality-web dev`), open `http://localhost:3001/sk/auth/login?redirect=%2F%5Cevil.example.com`.
2. Complete the email/password login form with a valid tenant account.
3. Expected: the browser stays on the tenant origin (redirect to `/`), because the backslash-prefixed path is rejected. Actual: `router.replace('/\\evil.example.com')` normalises to `//evil.example.com` and the browser navigates to the attacker origin as soon as the login succeeds.

## Suggested approach
1. Add a shared helper `isSafeInternalRedirect(path: string): boolean` under `frontend/apps/reality-web/src/lib/` (new file, e.g. `safe-redirect.ts`) that returns true only when `path.startsWith('/') && !path.startsWith('//') && !path.startsWith('/\\')`.
2. Import and use it in `login/page.tsx:72-73` in place of the inline check; return the helper's default `'/'` on rejection.
3. Refactor `callback/page.tsx:116-125` to use the same helper (drop the duplicated inline check) so the guard is single-sourced.
4. Add a Vitest suite `safe-redirect.test.ts` next to the helper covering: normal path (accept), `//evil.com` (reject), `/\evil.com` (reject), absolute URL (reject), empty string (reject), decoded `/%5Cevil.com` after `decodeURIComponent` (reject).
5. Add a `login/page.test.tsx` (new file) mirroring `callback/page.test.tsx:122-135` with "rejects a backslash-escaped redirect" and "rejects a protocol-relative redirect" using the same rendering harness the callback test uses.
6. `pnpm -F reality-web lint && pnpm -F reality-web typecheck && pnpm -F reality-web test` locally, then push.

## Alternatives considered
- **Server-side redirect allow-list resolved in a middleware** — rejected because reality-web's auth flow is client-navigated (`router.replace`) so the middleware would run only on the initial `/auth/login` request, not on the post-login navigation; the helper still has to live in the client.
- **Regex `^\/[^\/\\\\]` accept-list** — rejected because it silently accepts other exotic prefixes (`/\t`, control chars, decoded backslash on next tick) and diverges from the callback's three-clause explicit-reject shape that already ships tests.

## Root-cause trace
1. Symptom: after login at `/{locale}/auth/login?redirect=%2F%5Cevil.example.com`, browser navigates to `evil.example.com`.
2. ← `router.replace(safe)` at `login/page.tsx:72-73` receives `'/\evil.example.com'` because the `safe` computation passes the two-clause guard.
3. ← Guard at `login/page.tsx:72` is `redirectTo.startsWith('/') && !redirectTo.startsWith('//')` — matches `/\...` and accepts it.
4. Origin: `login/page.tsx` initial implementation shipped without the third `!startsWith('/\\')` clause that `callback/page.tsx:124` added later (see callback commit history); the login page never inherited that fix.

## Test plan
- [ ] `safe-redirect.test.ts` — helper unit tests for the six reject/accept cases above.
- [ ] `login/page.test.tsx` — page-level test that renders the login page with `?redirect=/%5Cevil.example.com`, submits credentials via the same mock as the callback test uses, and asserts `router.replace` was called with `'/'`.
- [ ] `pnpm -F reality-web test -- --run login/page.test callback/page.test safe-redirect.test`

## Out of scope
- Rewriting the `useRouter` / `next/navigation` locale-prefixing logic (tracked separately as `code-review-reality-web-auth-nav-drops-locale`).
- Extending the helper to the `register` / `reset-password` / `forgot-password` pages — none of them read a `redirect` param today.
- Server-side origin allow-listing or CSP-based redirect defence.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-login-open-redirect-backslash.md`
- Mark the matching `backlog.json` row as `status: "done"`
