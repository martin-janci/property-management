# code-review-ppt-web-core-dashboard-route-no-guard

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review Tier-1d 2026-09-09 (ppt-web-core dashboard-routes) + routine 2026-09-09 direct-read verification of core.tsx:160-168
**Confidence:** high

## Hypothesis
`frontend/apps/ppt-web/src/routes/groups/core.tsx:166-168` renders `/dashboard/manager` (`<ManagerDashboardPage/>`) and `/dashboard/resident` (`<ResidentDashboardPage/>`) with NO `<ProtectedRoute>` wrapper. The inline comment on lines 162-165 claims "the ProtectedRoute / role check on the target page handles auth + role-based fan-out" — but the target page (`ManagerDashboardPage.tsx`) reads `user?.role` only to toggle a customize link and otherwise renders unconditionally. Backend RLS still gates the actual data via section API calls, so this is a defense-in-depth gap and a documented-but-false guarantee (comment vs code drift), not a confirmed data leak. Fix: wrap `/dashboard/manager` in `<ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>`, wrap `/dashboard/resident` in a bare `<ProtectedRoute>`, and correct the misleading comment. Security fast-track (vector=security, confidence=high, score>=2).

## Evidence
- `frontend/apps/ppt-web/src/routes/groups/core.tsx:166-168` — `<Route path="/dashboard/manager" element={<ManagerDashboardPage />} />` and `<Route path="/dashboard/resident" element={<ResidentDashboardPage />} />` with no wrapping guard. Verified 2026-09-09 by direct file read.
- `frontend/apps/ppt-web/src/routes/groups/core.tsx:171-178` — `/dashboard/customize` shows the correct pattern with `<ProtectedRoute requiredRoles={['org_admin','super_admin']}>`. Every other sibling manager surface in the same file uses `<ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>`.
- `frontend/apps/ppt-web/src/features/dashboard/pages/ManagerDashboardPage.tsx:21-49` — no auth/role guard on the page; reads `user?.role` only to toggle the 'customize' link via `CUSTOMIZE_ROLES` (lines 19, 27-28). No `isAuthenticated`/role check, no redirect to `/forbidden`.
- `frontend/apps/ppt-web/src/routes/AppRoutes.tsx:37-68` — no wrapping guard. Public routes (`/`, `/login`, `/register`) sit in the same flat `<Routes>` as the dashboard routes, so nothing upstream enforces auth on `/dashboard/manager`. Net effect: any authenticated resident (and, since no auth gate exists, an unauthenticated visitor) can render the manager dashboard shell.

## Files
- `frontend/apps/ppt-web/src/routes/groups/core.tsx`
- `frontend/apps/ppt-web/src/features/dashboard/pages/ManagerDashboardPage.tsx`
- `frontend/apps/ppt-web/src/routes/AppRoutes.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (security-adjacent bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):** Mode: cloud-ok

## Repro steps
1. Build ppt-web in dev mode (`pnpm -F @ppt/web dev`) and open it in a private browser window.
2. Do NOT log in. Navigate directly to `http://localhost:5173/dashboard/manager`.
3. Observed: the manager dashboard shell renders (menus, layout, empty state widgets) instead of redirecting to `/login`.
4. Log in as a plain resident (no manager role). Navigate to `/dashboard/manager`.
5. Observed: the manager dashboard shell renders in full (data still gated by RLS, but the manager UI is exposed).
6. Expected in both cases: `<Navigate to="/login" />` (unauthenticated) or `<Navigate to="/forbidden" />` (resident on a manager route).

## Suggested approach
1. In `frontend/apps/ppt-web/src/routes/groups/core.tsx`, replace lines 166-168 with:
   - `<Route path="/dashboard/manager" element={<ProtectedRoute requiredRoles={[...MANAGER_ROLES]}><ManagerDashboardPage /></ProtectedRoute>} />`
   - `<Route path="/dashboard/resident" element={<ProtectedRoute><ResidentDashboardPage /></ProtectedRoute>} />`
   - Import `ProtectedRoute` and `MANAGER_ROLES` from the same locations used on lines 171-178 / the sibling `shared.tsx`.
2. Remove the misleading comment block on lines 162-165 (the claim that the target page guards is now false-and-obsolete; the guard lives here).
3. Keep the `/dashboard` → `/dashboard/manager` redirect on line 166 (the bare-URL landing) — but wrap the destination now that the guard is in place.
4. Add a `frontend/apps/ppt-web/src/routes/groups/core.dashboard-guard.test.tsx` Vitest render test: rendering `/dashboard/manager` with (a) no user redirects to `/login`, (b) a resident redirects to `/forbidden`, (c) a manager renders the page.
5. Run `pnpm -F @ppt/web test -- core.dashboard-guard`, then `pnpm -F @ppt/web typecheck` and `pnpm -F @ppt/web check`.

## Alternatives considered
- **Add the guard inside `ManagerDashboardPage.tsx` (page-level check + `<Navigate>` on failure)** — rejected because it duplicates a pattern the routing layer already owns everywhere else in `core.tsx`, and the codebase's convention (per the sibling `/dashboard/customize` route) is to guard at the `<Route>` boundary so a `useNavigate` inside the page can't accidentally run render-time side-effects on unauthorized users.
- **Wrap the entire `AppRoutes.tsx` output in one giant `<ProtectedRoute>`** — rejected because AppRoutes intentionally exposes public routes (`/`, `/login`, `/register`, listing pages) that must render without a session; a global guard would break the public entry points.

## Root-cause trace
1. Symptom: unauthenticated visitor or plain resident can render the manager dashboard shell at `/dashboard/manager`.
2. ← `core.tsx:166-168` has no `<ProtectedRoute>` wrapping the `<Route>` elements.
3. ← Comment at `core.tsx:162-165` documents an intent that the target page will guard — but the target page (`ManagerDashboardPage.tsx:21-49`) never adds the guard.
4. Origin: the dashboard routes shipped before `<ProtectedRoute>` was in place elsewhere; when `<ProtectedRoute>` landed for `/dashboard/customize` and the settings surfaces the manager/resident routes were not retrofitted. The stale comment codified the miss.

## Test plan
- [ ] `frontend/apps/ppt-web/src/routes/groups/core.dashboard-guard.test.tsx` — new Vitest render test with three cases (no user / resident / manager) asserting the guard behavior above.
- [ ] Regression: with the fix reverted (guard removed), the (no user) and (resident) cases must fail (IG3).
- [ ] Run `pnpm -F @ppt/web test -- core.dashboard-guard`, then `pnpm -F @ppt/web typecheck` and `pnpm -F @ppt/web check`.

## Out of scope
- Any change to `ProtectedRoute` itself or `MANAGER_ROLES` definition.
- Redesigning the resident dashboard page.
- Adding server-side guards (backend RLS already gates the data — this plan is defense-in-depth only).
- The related `home-role-hardcoded` finding (`code-review-ppt-web-core-home-role-hardcoded`) — separate backlog row with its own future plan.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-dashboard-route-no-guard.md`
- Mark the matching `backlog.json` row as `status: "done"`
