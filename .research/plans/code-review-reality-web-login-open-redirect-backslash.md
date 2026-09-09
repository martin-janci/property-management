# code-review-reality-web-login-open-redirect-backslash

**Vector:** security
**Score:** 2
**Source:** dispatcher Tier-1d rotating-expert-review 2026-09-05 (reality-web auth/login)
**Confidence:** high

## Hypothesis
`app/[locale]/auth/login/page.tsx` reads a user-controlled `redirect` search param and, after a successful `login()`, navigates via `router.replace(safe)` where `safe` only rejects `/` + `//` prefixes. The `/\evil.com` backslash-escape form slips through: browsers normalise `\` to `/` during URL resolution, so `router.replace('/\evil.com')` resolves to `//evil.com`, i.e. a protocol-relative navigation to an attacker origin. The sibling `app/[locale]/auth/callback/page.tsx:116-125` already implements the correct 3-clause guard and has a dedicated test at `callback/page.test.tsx:122-135`. The login page just missed the third clause (and has no test coverage of the redirect param). The smallest fix is to extract a shared `isSafeInternalRedirect(path)` helper (reject non-`/` prefix, `//`, and `/\`), use it in both pages, and mirror the callback's backslash-rejection test onto the login page. Security fast-track: score 2 + confidence high + vector security is enough to promote.

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx:33,72-73` — reads `searchParams.get('redirect')`, guards only for `startsWith('/') && !startsWith('//')`, then `router.replace(safe)` (no backslash rejection).
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.tsx:116-125` — guards `rawRedirect.startsWith('/') && !rawRedirect.startsWith('//') && !rawRedirect.startsWith('/\\')` — the third clause that login is missing.
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.test.tsx:122-135` — test cases "rejects a protocol-relative redirect_uri" AND "rejects a backslash-escaped redirect_uri (`/%5Cevil.example.com`)".
- Attack: `https://<tenant>/sk/auth/login?redirect=/%5Cevil.example.com` → user logs in → `router.replace('/\evil.example.com')` → browser resolves to `//evil.example.com` → cross-origin navigation to attacker origin (post-auth open redirect / phishing primitive).

## Files
- `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx:72`
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.tsx:116`

## Dependencies
- (none)

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Start reality-web locally (or use a preview build). Visit `http://localhost:3000/sk/auth/login?redirect=%2F%5Cevil.example.com` and log in with any test account.
2. Expected (after fix): `router.replace('/')` — the backslash-escaped value is rejected and the login lands on the safe default. Actual (today): `router.replace('/\evil.example.com')` — the browser resolves this to `//evil.example.com` and the user is bounced cross-origin immediately after auth.

## Suggested approach
1. Extract a shared helper `isSafeInternalRedirect(path: string): boolean` — likely under `frontend/apps/reality-web/src/lib/auth/isSafeInternalRedirect.ts` — that returns true only when `typeof path === 'string' && path.startsWith('/') && !path.startsWith('//') && !path.startsWith('/\\')`. Additionally decode-once and re-check to defeat `%5C`-encoded backslashes.
2. Use the helper in `app/[locale]/auth/login/page.tsx:72-73` and in `app/[locale]/auth/callback/page.tsx:116-125`, replacing the inline guards.
3. Fall back to `'/'` when the helper returns false (matches today's callback behaviour); keep the fallback consistent between both pages.
4. Add a login-page unit test `app/[locale]/auth/login/page.test.tsx` mirroring `callback/page.test.tsx:122-135` — assert that `?redirect=/%5Cevil.example.com`, `?redirect=//evil.example.com`, and `?redirect=http://evil.example.com` all resolve to `router.replace('/')`; and that `?redirect=/dashboard/manager` is preserved verbatim.
5. Add a helper-level test at `lib/auth/__tests__/isSafeInternalRedirect.test.ts` covering the decode-then-recheck edge (`%5C`) so future callers don't regress.
6. Grep for other `router.replace(redirect...)` / `router.push(redirect...)` occurrences under `frontend/apps/reality-web/src` and note in the PR whether any other sites also need the helper (do NOT widen this plan — file follow-ups).

## Alternatives considered
- **Just add `!startsWith('/\\')` inline to the login page** — rejected because the callback already carries a duplicated version of the same guard; consolidating into one helper prevents the same drift from happening again the next time a third clause is added (e.g. `\\`, `data:`, or newline-embedded paths).
- **Regex-based allowlist (`^/[a-zA-Z0-9/_-]+$`)** — rejected because reality-web routes include locale-scoped paths with encoded segments (`/sk/listings/ba-byt-3i`, unicode slugs); an allowlist here would cause legitimate redirects to be dropped and reintroduce the "stuck at `/` after login" class of bug.

## Root-cause trace
1. Symptom: victim opens a crafted `redirect=/%5Cevil.example.com` link, logs into their account, and is silently taken to the attacker origin immediately after authentication.
2. ← `app/[locale]/auth/login/page.tsx:72` — `const safe = redirectTo.startsWith('/') && !redirectTo.startsWith('//') ? redirectTo : '/'; router.replace(safe);`. The `/\evil.com` form passes both clauses.
3. ← `app/[locale]/auth/login/page.tsx:33` — `const redirectTo = searchParams.get('redirect') ?? '/'`. Raw user-controlled query param is trusted through to the guard.
4. Origin: the callback page later added the third `!startsWith('/\\')` clause plus its test (see `callback/page.test.tsx:129`), but the login page's guard was never updated in lock-step — introduced when the two flows drifted apart.

## Test plan
- [ ] New unit test `frontend/apps/reality-web/src/app/[locale]/auth/login/page.test.tsx` — cases: `//evil.example.com`, `/\evil.example.com` (raw + `%5C`-encoded), `http://evil.example.com`, and a legitimate `/dashboard/manager` are all handled correctly.
- [ ] New helper test `frontend/apps/reality-web/src/lib/auth/__tests__/isSafeInternalRedirect.test.ts` — decode-once-then-recheck, mixed-case escapes, empty string, non-string input.
- [ ] Command: `cd frontend && pnpm -F reality-web test -- auth/login/page.test.ts auth/isSafeInternalRedirect.test.ts`

## Out of scope
- Broader redirect hardening elsewhere in reality-web (ppt-web is a separate app; agency-portal callback flows are separate). File follow-ups if grep finds them.
- Rate-limiting or CAPTCHA on the login endpoint itself — this plan is about the post-auth navigation only.
- Any reality-server-side changes; the vector is a client-side navigation issue only.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-login-open-redirect-backslash.md`
- Mark the matching `backlog.json` row as `status: "done"`
