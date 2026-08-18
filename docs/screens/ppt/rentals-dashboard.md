---
id: ppt/rentals-dashboard
name: Short-Term Rentals Dashboard
product: ppt
implementations:
  ppt-web:
    route: "/rentals"
    component: RentalsDashboardPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/settings-integrations
    rel: sibling
sharedComponents: []
diagrams: []
useCases:
  - UC-29
  - UC-30
epics:
  - Epic-18
designSources: []
owner: pm-frontend
---

# Short-Term Rentals Dashboard

Manager-facing home for the Short-Term Rental Integration feature (Epic 18,
UC-29 Short-term Rental Management + UC-30 Guest Registration System). The
`/rentals` dashboard surfaces reservation statistics, upcoming/active bookings,
and platform-connection health, and links out to the rest of the rentals
feature. It is the canonical screen-map anchor for UC-29 — the
`ppt/settings-integrations` screen only carries the Airbnb OAuth connect flow
(UC-29.1 / UC-29.2), while the day-to-day rental management lives here.

The rentals feature (route group `frontend/apps/ppt-web/src/routes/groups/rentals.tsx`)
owns these routes, all rendered by pages under
`frontend/apps/ppt-web/src/features/rentals/`:

- `/rentals` — `RentalsDashboardPage` (stats, upcoming/active bookings, connection health)
- `/rentals/connections` — `PlatformConnectionsPage` (UC-29.1/29.2 connect, UC-29.3 sync)
- `/rentals/bookings` — `BookingsPage` (reservation list + filters)
- `/rentals/bookings/:bookingId` — `BookingDetailPage` (UC-29.9/29.10 check-in/out)
- `/rentals/calendar` — `CalendarPage` (UC-29.4 reservation calendar)
- `/rentals/guests` — `GuestRegistrationPage` (UC-29.5 + UC-30 guest registration)
- `/rentals/reports` — `TaxReportPage` (UC-29.13/29.14/29.15 statistics + tax export)

Rentals calls (via `@ppt/api-client` `ShortTermRentalsService` — `rentalsApi*`):
- `rentalsApiListReservations` — reservation list (dashboard + bookings)
- `rentalsApiGetReservation` — single reservation (booking detail)
- `rentalsApiListConnections` / `rentalsApiCreateConnection` — platform connections
- `rentalsApiSyncPlatforms` — trigger a channel sync
- `rentalsApiListGuests` — guest registrations
- `rentalsApiCheckIn` / `rentalsApiCheckOut` — guest check-in / check-out

## Notes

### Specific (recent)
- 2026-08-04 — **rentals mutations now guard auth (session-expiry UX), PR #2648.**
  The rentals route group (`groups/rentals.tsx`) no longer dereferences `auth!`
  on the request path. All 10 rentals request sites route through
  `requireRentalsAuthHeaders(auth)`, which throws a typed
  `AuthError('SESSION_EXPIRED')` when the session is lost mid-flight instead of
  raising an opaque `TypeError`. The four mutations —
  `rentalsApiCreateConnection` (create connection), `rentalsApiSyncPlatforms`
  (sync), `rentalsApiCheckIn`, `rentalsApiCheckOut` — wire
  `handleRentalsAuthError(error, logout)` as their TanStack Query `onError`: a
  session-loss `AuthError` calls AuthContext `logout()`, which clears the
  session and lets `ProtectedRoute` redirect to `/login` (the app's existing
  dropped-session UX); non-session errors are left untouched. Queries were
  already gated on `enabled: !!auth` and were converted to the same helper for
  consistency. Regression coverage:
  `frontend/apps/ppt-web/src/routes/groups/rentals.auth-guard.test.tsx`
  (`requireRentalsAuthHeaders(null)` throws `AuthError`, not `TypeError`; a
  wired `useMutation` settles in a handled `AuthError` state and never hits the
  API). `buildStatus`/`apiStatus` unchanged — this is a robustness/UX guard, not
  a surface change.
- 2026-07-09 — created this screen-map to give UC-29 (and UC-30) a canonical
  home. UC-29 had only been attached to `ppt/settings-integrations` (the Airbnb
  connect sub-flow, gap-83-1); the primary rentals management surface at
  `/rentals` and its sibling routes had no screen-map at all.
- `endpoints` left empty: the rentals endpoints are not (yet) in the
  `@ppt/sitemap` endpoint catalogue (same situation as `ppt/settings-integrations`).
- `apiStatus: partial` — reservations / connections / guests / check-in-out are
  live, but the calendar and tax-report routes still render with empty data
  (no calendar or tax-report endpoint exists yet; see the route wrappers in
  `groups/rentals.tsx`).
- No `sitemapRefs`: the `/rentals*` routes are not present in `@ppt/sitemap`
  (only `/settings/integrations` is). The rest of the rentals routes remain
  unmapped in the sitemap + screen-map tree — a follow-up drift item.

## Agent Log
- 2026-08-18 — agent: screen-map-drift-pr-2647-ppt-retry2 — noted i18n update in
  PR #2647: `BookingDetailPageRoute` in `frontend/apps/ppt-web/src/routes/groups/rentals.tsx`
  now renders its missing-param fallback via
  `t('errors.bookingNotFound', 'Booking not found')` (was hardcoded English);
  `errors.bookingNotFound` key added to all locale bundles;
  `useTranslation()` hook added inside the route wrapper. The dedicated
  `/rentals/bookings/:bookingId` (BookingDetailPage) route still has no
  standalone screen-map — logged here on the rentals-dashboard anchor as an
  interim home per the parent screen-map convention (existing follow-up drift
  for the whole `/rentals/*` sub-tree already noted below). No route or
  component change.
- 2026-08-04 — agent: screen-map-drift-pr-2648-ppt — reconciled this screen-map
  with PR #2648 (guard rentals mutation auth). Documented the mutation
  auth-guard / session-expiry redirect behavior (`requireRentalsAuthHeaders` +
  `handleRentalsAuthError` on all four rentals mutations) under Notes > Specific.
  Docs-only screen-map reconcile; no frontmatter outcome changed.
- 2026-07-09 — agent: gap-screens-link-uc-29 — created the Short-Term Rentals
  Dashboard screen-map and linked UC-29 (+ UC-30) to it, giving the
  Short-Term Rental Management use case a canonical screen-map home beyond the
  Airbnb-connect sub-flow already tracked on ppt/settings-integrations.
