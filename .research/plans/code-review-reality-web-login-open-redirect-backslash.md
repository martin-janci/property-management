# code-review-reality-web-login-open-redirect-backslash

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review 2026-09-05 reality-web auth (signals/2026-09-05-reality-web-tier1d.json)
**Confidence:** high

## Hypothesis
The reality-web login page (`app/[locale]/auth/login/page.tsx`) accepts a `redirect` query param and after a successful login navigates to it with a two-clause guard (`startsWith('/') && !startsWith('//')`). Its sibling callback page uses a **three**-clause guard that additionally rejects the backslash-escape form `/\`. Browsers normalise `\` to `/` during URL resolution, so `router.replace('/\\evil.com')` resolves to `//evil.com` — a protocol-relative navigation to an attacker origin immediately after authentication. The smallest resolving change is to extract the callback's proven 3-way check into a shared `isSafeInternalRedirect` helper and apply it in both pages.

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx:33,72-73` — reads `searchParams.get('redirect')`, guards only two clauses, then `router.replace(safe)`.
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.tsx:116-125` — proven 3-clause guard (`.startsWith('/') && !.startsWith('//') && !.startsWith('/\\\\')`).
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.test.tsx:122-135` — dedicated cases for both `//` and `/\evil.com`; no equivalent test exists on the login page.
- Attack path is trivial: any anonymous visit to `https://<tenant>/sk/auth/login?redirect=/%5Cevil.example.com` becomes a post-auth open redirect on successful login.

## Files
- `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx:72`
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.tsx:116`
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.test.tsx:122`
- `frontend/apps/reality-web/src/lib/auth-api.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug/security; needed to confirm guard chain in both files)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived):** `Mode: cloud-ok` (frontend-only Next.js change; jest/vitest verifiable under the cloud runner's frontend toolchain).

Mode: cloud-ok

## Repro steps
1. Start reality-web dev server locally or use a staging deploy.
2. Visit `https://<host>/sk/auth/login?redirect=%2F%5Cevil.example.com` (URL-encoded `/\evil.example.com`).
3. Complete a successful login with a valid test account.
4. Observed today: `router.replace('/\\evil.example.com')` — browser resolves this to `//evil.example.com`, navigating off-origin after authentication.
5. Expected after fix: the guard rejects the backslash-prefixed path and falls back to the default `/`; the user lands on the localised home instead of an attacker origin.

## Suggested approach
1. Create `frontend/apps/reality-web/src/lib/redirect.ts` exporting `isSafeInternalRedirect(path: string | null | undefined): boolean` returning true iff `path.startsWith('/') && !path.startsWith('//') && !path.startsWith('/\\')`.
2. Import the helper in `app/[locale]/auth/login/page.tsx` and replace the inline `safe` computation at line 72 with `const safe = isSafeInternalRedirect(redirectTo) ? redirectTo : '/'`.
3. Import the same helper in `app/[locale]/auth/callback/page.tsx` and replace the local 3-clause check at lines 116-125 with a call to `isSafeInternalRedirect` — no behaviour change, just one source of truth.
4. Copy the two failing-on-main tests (`rejects a protocol-relative redirect_uri`, `rejects a backslash-escaped redirect_uri`) from `callback/page.test.tsx:122-135` into a new `login/page.test.tsx`.
5. Add a `lib/redirect.test.ts` covering the canonical safe cases (`/`, `/dashboard`, `/sk/settings`) and the two rejected forms (`//evil.com`, `/\evil.com`).

## Alternatives considered
- **Inline the third clause in login/page.tsx only, leaving callback's inline guard unchanged** — rejected because it leaves two divergent implementations of the same security check; the next feature that adds a redirect param will diverge again. Centralising via a shared helper is the same LOC and future-proofs.
- **Server-side allowlist of destination paths** — rejected because reality-web is a Next.js public site with a broad set of legitimate post-auth destinations (localised home, listing detail, saved-search page); an allowlist is disproportionately restrictive vs. rejecting the two known unsafe prefixes.

## Root-cause trace
1. Symptom: authenticated user is bounced to attacker origin on successful login with a crafted `redirect` param.
2. ← `router.replace(safe)` in `login/page.tsx:73` where `safe` was computed by an incomplete guard at `login/page.tsx:72`.
3. ← The login page's guard (two clauses) diverged from the callback page's guard (three clauses), which was hardened in a prior fix but not back-ported to the sibling.
4. Origin: the login page's guard was introduced separately from the callback page's guard; the callback got the `/\` clause during Epic-10A SSO hardening and the login page did not. `git log frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx` will pinpoint the commit that added the two-clause form.

## Test plan
- [ ] `frontend/apps/reality-web/src/app/[locale]/auth/login/page.test.tsx` — new file mirroring `callback/page.test.tsx:122-135`, asserting both `//` and `/\` redirect params fall back to `/`.
- [ ] `frontend/apps/reality-web/src/lib/redirect.test.ts` — pure-function tests for `isSafeInternalRedirect`.
- [ ] Run: `pnpm -F @ppt/reality-web test`

## Out of scope
- Locking the redirect down to a route allowlist.
- Fixing the sibling `code-review-reality-web-auth-nav-drops-locale` finding (locale-drop on `router.push`) — separate plan, separate PR.
- Any changes to ppt-web's authentication flow.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-login-open-redirect-backslash.md`
- Mark the matching `backlog.json` row as `status: "done"`
