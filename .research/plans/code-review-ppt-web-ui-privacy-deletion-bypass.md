# code-review-ppt-web-ui-privacy-deletion-bypass

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review ppt-web-ui 2026-10-28
**Confidence:** high

## Hypothesis

`PrivacySettingsPage` guards its "Delete account" action with an email-confirmation check that short-circuits to allow when `user?.email` is nullish — any unauthenticated or auth-not-yet-hydrated session passes the frontend gate. The page is also mounted on an unprotected route (`/settings/privacy` has no `<ProtectedRoute>` wrapper and the component itself never checks `isAuthenticated`), so an unauthenticated visitor can reach the delete UI and POST `/api/v1/gdpr/deletion/request` with an attacker-controlled `confirmation` body — leaving only the backend as the sole mitigation of a GDPR-grade destructive action. Fix: require `user?.email` for the confirmation gate (treat its absence as deny), and wrap the route in `ProtectedRoute` so unauthenticated users never render the page in the first place.

## Evidence

- `frontend/apps/ppt-web/src/features/privacy/pages/PrivacySettingsPage.tsx:128` — `if (user?.email && confirmation.trim().toLowerCase() !== user.email.toLowerCase()) { setError(...); return; }` short-circuits to false when `user?.email` is nullish; control falls through to the POST at `PrivacySettingsPage.tsx:136`.
- `frontend/apps/ppt-web/src/routes/groups/core.tsx:255` — `<Route path="/settings/privacy" element={<PrivacySettingsPage />} />` has no `<ProtectedRoute>` wrapper (sibling authed routes do).
- `PrivacySettingsPage.tsx:23` — the page component never checks `isAuthenticated` or redirects.
- No test file exists for `PrivacySettingsPage` (`features/privacy/pages/` has only `PrivacySettingsPage.tsx` + `index.ts`); the deletion branch has zero coverage.

## Files

- `frontend/apps/ppt-web/src/features/privacy/pages/PrivacySettingsPage.tsx`
- `frontend/apps/ppt-web/src/routes/groups/core.tsx`

## Dependencies

<!-- No dependencies. -->

## Required capabilities

- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser  · local-only
- [ ] C5 — ADB device  · local-only
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. In a fresh browser session (no `AuthProvider` state) navigate directly to `http://localhost:5173/settings/privacy`. The page renders instead of redirecting to login (`routes/groups/core.tsx:255` has no `ProtectedRoute`).
2. Open the "Delete my account" accordion, type any string (e.g. `anything`) into the confirmation field, click "Delete my account forever".
3. In DevTools Network, observe a `POST /api/v1/gdpr/deletion/request` with body `{ confirmation: "anything" }` leaves the browser — the frontend's email check was skipped because `user?.email` is `undefined`. **Expected:** the UI should block submission and no request should fire; **actual:** the request is sent with only backend-side mitigation.

## Suggested approach

1. In `PrivacySettingsPage.tsx:128`, invert the guard so a missing `user?.email` deny-closes: `if (!user?.email || confirmation.trim().toLowerCase() !== user.email.toLowerCase()) { setError(…); return; }`. Keep the surfaced copy ("Please enter your email …").
2. In the same component, add an early `if (!isAuthenticated || !user) return <Navigate to="/auth/login" replace />;` guard at the top of the render (matching the pattern already used elsewhere in `features/privacy`).
3. In `routes/groups/core.tsx:255`, wrap the `/settings/privacy` `<Route>` element in `<ProtectedRoute>` the same way the surrounding authed routes do (grep the file for `ProtectedRoute` to copy the exact shape).
4. Create `frontend/apps/ppt-web/src/features/privacy/pages/PrivacySettingsPage.test.tsx` covering: (a) unauthenticated visit → redirects (route-level guard); (b) `user.email === undefined` → "Delete" click never fires the deletion mutation and shows the "enter your email" error; (c) correct email → mutation fires once.
5. `cd frontend && pnpm -F @ppt/ppt-web test PrivacySettingsPage` — expect the three cases green; current main would fail at least (a) and (b).

## Alternatives considered

- **Backend-only fix (tighten `/api/v1/gdpr/deletion/request`)** — rejected because the backend already requires an authenticated session and should keep doing so, but the frontend is the destructive-action confirmation surface and today a UX regression (missing user context) would silently submit an account-deletion request; defense in depth belongs at the point where the user clicks the button.
- **Only tighten the client email guard, leave the route unprotected** — rejected because an unauthenticated visitor can still render the whole "Delete my account forever" UI, which is both a confusing UX and a surface for session-fixation / CSRF-adjacent flows that happen to land a cookie before the click.

## Root-cause trace

1. Symptom: unauthenticated or hydration-pending user clicks "Delete my account forever" → POST `/api/v1/gdpr/deletion/request` fires with an attacker-controlled `confirmation`.
2. ← Guard at `PrivacySettingsPage.tsx:128` uses `user?.email && …` as the whole condition. When the left operand is falsy, the whole expression is falsy, so the early-return branch is skipped and the mutation runs.
3. ← Route mount at `routes/groups/core.tsx:255` has no `ProtectedRoute` wrapper; the page component never checks `isAuthenticated` itself, so the unauthenticated path is reachable.
4. Origin: added in the initial `PrivacySettingsPage` feature commit (no later edit tightened the check); the GDPR-deletion backend route landed independently and assumed a stricter frontend gate.

## Test plan

- [ ] `frontend/apps/ppt-web/src/features/privacy/pages/PrivacySettingsPage.test.tsx` — three cases: unauthenticated → redirect; `user.email` undefined → no mutation + visible error; correct email → single mutation call.
- [ ] Regression scenario: `useAuth` returns `{ user: null, isAuthenticated: false }` → the page never renders (asserted by querying for the "Delete my account" heading absence).
- [ ] `cd frontend && pnpm -F @ppt/ppt-web test PrivacySettingsPage` locally; the new test file must fail on `origin/main` and pass on this branch (IG3).

## Out of scope

- Backend changes to `/api/v1/gdpr/deletion/request` — the backend's own session + CSRF handling is already in place.
- Styling / UX copy improvements to `PrivacySettingsPage`.
- Audit of other privacy-adjacent routes (notification preferences, exports) — belongs in a follow-up `pm-security` sweep.

## After-merge

- Move this file to `plans/_archive/code-review-ppt-web-ui-privacy-deletion-bypass.md`
- Mark the matching `backlog.json` row (`code-review-ppt-web-ui-privacy-deletion-bypass`) as `status: "done"`
