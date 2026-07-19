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
- 2026-07-19 — agent: page now renders via resolved-layout section registry (defensive rendering, spec 2026-07-19-layout-content-manager-design)
- 2026-07-09 — agent: gap-screens-link-uc-29 — created the Short-Term Rentals
  Dashboard screen-map and linked UC-29 (+ UC-30) to it, giving the
  Short-Term Rental Management use case a canonical screen-map home beyond the
  Airbnb-connect sub-flow already tracked on ppt/settings-integrations.
