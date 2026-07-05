# code-review-ppt-web-core-dashboard-route-unprotected

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review ppt-web-core 2026-07-05 · frontend/apps/ppt-web/src/routes/groups/core.tsx:149
**Confidence:** high

## Hypothesis

The `/dashboard/manager` and `/dashboard/resident` routes in `frontend/apps/ppt-web/src/routes/groups/core.tsx:149-150` are mounted as bare `<Route element={<ManagerDashboardPage />} />` — no `<ProtectedRoute>` wrapper, no `useAuth` check inside the page. The comment at `core.tsx:144-148` claims the target page handles auth, but `ManagerDashboardPage.tsx` only calls `useTranslation()` and renders localized chrome plus `<ActionQueue />` — no `useAuth`, no `isAuthenticated` gate, no redirect. An unauthenticated visitor to `/dashboard/manager` therefore renders the full manager shell (titles, stat tiles, action queue skeletons). Data fetches under it will 401, but the layout leak by itself discloses that a manager surface exists — enough of a shape hint for enumeration. The smallest fix is to wrap the two dashboard routes in the existing `<ProtectedRoute>` component at `frontend/apps/ppt-web/src/components/ProtectedRoute.tsx`, mirroring how other authenticated groups do it.

## Evidence

- `frontend/apps/ppt-web/src/routes/groups/core.tsx:149` — `<Route path="/dashboard/manager" element={<ManagerDashboardPage />} />` — no wrapper.
- `frontend/apps/ppt-web/src/routes/groups/core.tsx:150` — `<Route path="/dashboard/resident" element={<ResidentDashboardPage />} />` — same shape, same gap.
- `frontend/apps/ppt-web/src/features/dashboard/pages/ManagerDashboardPage.tsx` — component body has zero references to `useAuth`, `isAuthenticated`, `Navigate`, or `Login`. Only `useTranslation`.
- `frontend/apps/ppt-web/src/components/ProtectedRoute.tsx` — the intended wrapper already exists (`export function ProtectedRoute`) with `ProtectedRouteProps` supporting an optional `requiredRole`.
- Phase 1.5 signal id `code-review-ppt-web-core-dashboard-route-unprotected` (score 3, high, security), backlog row updated_at 2026-07-05.

## Files
- `frontend/apps/ppt-web/src/routes/groups/core.tsx:149`
- `frontend/apps/ppt-web/src/features/dashboard/pages/ManagerDashboardPage.tsx`
- `frontend/apps/ppt-web/src/components/ProtectedRoute.tsx`

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Sign out of ppt-web (clear the auth token / open a fresh incognito window).
2. Navigate directly to `/dashboard/manager`.
3. Expected: redirected to `/login` (or the app's unauthenticated landing).
4. Actual: the Manager Dashboard chrome renders — page title, subtitle, stat tiles with placeholder values, `ActionQueue` shell. Data fetches under it 401, but the layout has already leaked the shape of the manager surface.

## Suggested approach
1. Import `ProtectedRoute` from `../../components` in `frontend/apps/ppt-web/src/routes/groups/core.tsx` (path relative to `routes/groups/`).
2. Wrap the `/dashboard/manager` route: `<Route path="/dashboard/manager" element={<ProtectedRoute><ManagerDashboardPage /></ProtectedRoute>} />`.
3. Wrap the `/dashboard/resident` route the same way. If `ProtectedRoute` supports a `requiredRole`, pass `requiredRole="manager"` / `"resident"` respectively; if the roles machinery isn't wired end-to-end yet, ship just the auth gate now — role gating is a separate follow-up (`ManagerDashboardPage` also needs a runtime role check inside, but the auth gate closes the layout-leak surface).
4. Delete the stale comment at `core.tsx:144-148` (the claim it makes is now false anyway, and the redirect target no longer needs justifying once the wrap is in place).
5. Add `frontend/apps/ppt-web/src/routes/groups/core.test.tsx` (or extend the nearest existing route-group test file if there is one) — render the routes inside a `MemoryRouter` pointed at `/dashboard/manager` with an unauthenticated `AuthProvider` and assert the redirect. Then flip the auth provider to authenticated and assert the manager chrome renders.
6. Run `pnpm --filter ppt-web check && pnpm --filter ppt-web typecheck && pnpm --filter ppt-web test -- routes/groups/core` locally before pushing.

## Alternatives considered
- **Guard inside `ManagerDashboardPage` (call `useAuth` in the component body).** Rejected: matches the current comment's stated intent but leaves the same failure mode in every other bare route, and the correct pattern in this codebase is `ProtectedRoute` at the router (used by every other authenticated group). One centrally-visible wrap is easier to audit than N in-component guards.
- **Ship the role gate together with the auth gate.** Rejected for scope: the auth-leak is the immediate risk; role-gating deserves its own follow-up so the diff stays reviewable and the role machinery gap can be triaged separately.
- **Redirect `/dashboard` → `/login` when unauthenticated instead of wrapping the child routes.** Rejected: `/dashboard` today `<Navigate>`s to `/dashboard/manager`, which the wrapper on `/dashboard/manager` will then defend — same effect with fewer moving parts.

## Root-cause trace
- `routes/groups/core.tsx:144-150` was added in the Epic-124 dashboard batch; the comment records an author intent ("target page handles auth") that was never implemented, and no test asserted the bare mount rejects unauth. Every other authenticated route group in the file wraps its children in `<ProtectedRoute>` (grep for `<ProtectedRoute` in `routes/groups/`), so this is a divergence from the local pattern, not a codebase-wide convention gap.

## Test plan
- New: `frontend/apps/ppt-web/src/routes/groups/core.test.tsx` — two Vitest cases:
  1. `render <MemoryRouter initialEntries={['/dashboard/manager']}><AuthProvider value={{isAuthenticated:false}}>…</AuthProvider></MemoryRouter>` → `expect(screen.queryByText(/dashboard.managerDashboard/)).toBeNull()` **and** assert redirect to `/login`.
  2. Flip `isAuthenticated:true` → `expect(screen.getByText(/dashboard.managerDashboard/)).toBeInTheDocument()`.
- Existing: run `pnpm --filter ppt-web test` to confirm nothing else regressed.
- The unauth case would pass today (bug present) if the assertion is inverted — write it to fail on `main` so IG3 is honored.

## Out of scope
- Role-based gating between `manager` / `resident` / other roles inside the dashboard family (needs `ProtectedRoute` `requiredRole` audit + role machinery review — separate plan).
- Auditing every other route in `routes/groups/**` for the same "bare mount" pattern (this plan fixes the two dashboards; a follow-up sweep grep for `<Route path=` in `routes/groups/**` without an adjacent `<ProtectedRoute` will surface more).
- Server-side layout hardening (returning 403 shells) — not applicable to a client-only SPA route.

## After-merge
- Retire this plan on merge — move the row to `status: done` in `backlog.json`, move `plans/<slug>.md` to `plans/_archive/`.
- Emit a follow-up backlog vector `refactor-ppt-web-routes-groups-audit-bare-mounts` (score 1, low) to sweep `routes/groups/**` for other bare mounts using the grep above. Do NOT auto-promote — the sweep needs a human triage pass first.
- Note in the `pm-security` risks feed that this class of "author-intent comment claims a guard exists but the wrap is missing" is worth watching in future review rotations.
