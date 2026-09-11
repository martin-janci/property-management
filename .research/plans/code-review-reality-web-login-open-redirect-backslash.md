# code-review-reality-web-login-open-redirect-backslash

**Vector:** security
**Score:** 2
**Source:** dispatcher Tier-1d rotating-expert-review 2026-09-05 (reality-web auth)
**Confidence:** high

## Hypothesis
The reality-web email/password login page reads its post-login destination
from a user-controlled `redirect` query param and only rejects two of the
three known open-redirect shapes — the third (`/\evil.com`, backslash-form)
is missed. Browsers normalise `\` → `/` during URL resolution, so
`router.replace('/\\evil.com')` becomes a protocol-relative navigation to
an attacker origin. The sibling OAuth callback already guards all three
shapes AND has a matching regression test — the login page was written to
the same idea but dropped the third clause. Extracting a shared
`isSafeInternalRedirect(path)` helper and using it in both places closes
the gap in one landable frontend-only patch on `reality-web`
(unaffected by the api-server / mobile-native cloud-build blockers).

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx:33` — `const redirectTo = searchParams.get('redirect') ?? '/'`
- `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx:72` — `const safe = redirectTo.startsWith('/') && !redirectTo.startsWith('//') ? redirectTo : '/'` — the `/\\` case is not rejected
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.tsx:122-124` — the tested sibling guard already checks all three: `startsWith('/') && !startsWith('//') && !startsWith('/\\')`
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.test.tsx:122-135` — dedicated regression cases "rejects a protocol-relative redirect_uri" AND "rejects a backslash-escaped redirect_uri"
- Reproducer: `https://<tenant>/sk/auth/login?redirect=/%5Cevil.example.com` → after a successful login the user is silently bounced to `evil.example.com`

## Files
- `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.test.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug/security fix)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived):** `cloud-ok` — reality-web builds and tests
run cleanly in the cloud runner (unlike api-server per issue #2949 and
mobile-native per issue #2652).

Mode: cloud-ok

## Repro steps
1. Serve the reality-web app locally and open
   `http://localhost:3000/sk/auth/login?redirect=%2F%5Cattacker.example.com`.
2. Log in with any valid credentials.
3. Expected: navigation stays inside the tenant origin (falls back to `/`).
   Actual: the browser navigates to `//attacker.example.com` because the
   login guard misses the `/\` shape.
4. Reversed after the fix: the same URL falls back to `/` and a new unit
   test in `login/page.test.tsx` asserts `router.replace('/')` was called.

## Suggested approach
1. Introduce a shared helper — e.g. `frontend/apps/reality-web/src/lib/safeRedirect.ts` exporting `isSafeInternalRedirect(path: string): boolean` — that returns `path.startsWith('/') && !path.startsWith('//') && !path.startsWith('/\\')`. Add a second-pass URL-decode-then-recheck for defence in depth.
2. Replace `login/page.tsx:72` `redirectTo.startsWith('/') && !redirectTo.startsWith('//')` with `isSafeInternalRedirect(redirectTo)`; fall back to `/` otherwise.
3. Refactor `callback/page.tsx:122-124` to call the same helper instead of the inline 3-way boolean chain.
4. Create `login/page.test.tsx` (mirror `callback/page.test.tsx`) with three cases: (a) `/dashboard` → `router.replace('/dashboard')`; (b) `//evil.com` → `router.replace('/')`; (c) `/\evil.example.com` (`%5C`) → `router.replace('/')`.
5. Optional harden: refuse a decoded second-pass string that starts with any of the three unsafe prefixes (catches `/%5C%5Cevil.com` double-encoding).
6. `pnpm -F @ppt/reality-web test` locally to confirm both suites pass.
7. `pnpm biome check --write frontend/apps/reality-web/src/{lib,app}` to keep the format check green.

## Alternatives considered
- **Inline the missing `/\\` clause in `login/page.tsx` only** — rejected because it re-introduces the same duplicated logic across two files, which is exactly how the gap was introduced in the first place. A shared helper prevents the same drift on the next new auth surface (registration, magic-link).
- **Move the redirect validation into `next.config.mjs` middleware** — rejected because Next middleware runs before `router.replace` in the client and cannot reject a client-side navigation initiated from a click; the wrong-layer fix leaves both call sites unguarded.

## Root-cause trace
1. Symptom: authenticated user is navigated to an attacker origin after logging in via `login?redirect=/%5C…`.
2. ← `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx:72` — `const safe = redirectTo.startsWith('/') && !redirectTo.startsWith('//') ? redirectTo : '/'` — missing the `!startsWith('/\\')` clause.
3. ← implicit contract mismatch with `callback/page.tsx:122-124` which enforces all three checks; the login page duplicates the idea rather than importing a shared helper, so the checklist drifted.
4. Origin: the login page's redirect handling was authored alongside the callback (see identical guard shape) but never picked up the `/\\` case added later to the callback; blame the commit that introduced `!rawRedirect.startsWith('/\\')` in `callback/page.tsx` for closing the gap on only one of the two auth surfaces.

## Test plan
- [ ] `frontend/apps/reality-web/src/app/[locale]/auth/login/page.test.tsx` — new file, three cases: safe internal path, protocol-relative, backslash-escaped (IG3 — must fail on `main` before the guard swap).
- [ ] `frontend/apps/reality-web/src/app/[locale]/auth/callback/page.test.tsx` — unchanged, must still pass after the helper refactor.
- [ ] `pnpm -F @ppt/reality-web test -- --run` locally; `pnpm -F @ppt/reality-web typecheck`.
- [ ] `pnpm biome check frontend/apps/reality-web/src/{lib/safeRedirect.ts,app/[locale]/auth}`.

## Out of scope
- Ppt-web login redirect handling — separate stack, separate audit; file its own vector if the same shape appears there.
- Rewriting the callback OAuth flow itself; this plan only extracts the shared guard and adds the missing login-side check.
- Adding server-side `Location` header sanitisation on the reality-server — the exploit is fully client-side, and the server never emits an attacker-controlled Location for this flow.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-login-open-redirect-backslash.md`.
- Mark the matching `backlog.json` row as `status: "done"` and append the merged PR # to `sources`.
