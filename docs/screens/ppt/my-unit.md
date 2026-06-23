---
id: ppt/my-unit
name: My Unit (Resident)
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/my-unit"
    component: MyUnitPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens:
  - id: ppt/dashboard-resident
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics:
  - "3"
designSources: []
owner: pm-frontend
---

# My Unit (Resident)

Read-only, resident-scoped view of the unit(s) the signed-in user is a resident
of (Epic 3, Story 3.6 — [PM #981](https://github.com/martin-janci/property-management/issues/981) gap 5;
Paperclip [BIT-192](/BIT/issues/BIT-192) / [BIT-201](/BIT/issues/BIT-201)).

Presentational `MyUnitPage` (`features/my-unit`) is mounted by
`routes/groups/buildings.tsx` at `/my-unit` behind `ProtectedRoute` and lazy-loaded
via `routes/lazyRoutes.tsx`. It self-resolves the caller's associations from the
`@ppt/api-client` `my-units` module (`useMyUnits()` → `GET /api/v1/users/me/units`);
there is no path/route param, so there is no IDOR surface. Reached from the
Resident Dashboard "View my unit" quick-link.

For each resolved residency the page shows:

- **Unit detail** — designation, entrance, type, floor, rooms, size, ownership
  share, occupancy, description (non-PII fields only).
- **Building** — name + address.
- **Your residency** — association role, primary-contact flag, active/ended
  status, start/end dates.
- **Unit switcher** — a `<select>` rendered only when the caller is a resident of
  more than one unit; defaults to the first unit and re-defaults if the current
  selection drops out of the returned set.

## Privacy

Other residents' and owners' identities are **never** included in the response.
The filter is enforced **server-side** in the query layer
(`UnitResidentRepository::find_my_units`, scoped strictly to the caller's
`user_id`); the `MyUnitRow` projection has no field for co-residents/owners and
omits manager-internal unit `notes`. The page renders only what the API returns
and shows an explicit privacy note. Covered by `db/tests/my_units_privacy_tests.rs`
and `api-server/tests/my_units_resident_view_tests.rs` (co-resident PII never
leaks; ended residencies excluded; 401 when unauthenticated).

## States

- **Loading** — "Loading your unit…".
- **Error** — `role="alert"` with the error message.
- **Empty** — caller is not a resident of any unit; advises contacting the
  building manager.
- **Populated** — unit/building/residency sections; switcher when >1 unit.

## Notes

### Specific (recent)
- 2026-06-22 — created with the surface in PR #1701 (Story 3.6).

## Agent Log
- 2026-06-22 — FrontendEngineer: added screen-map for the new `/my-unit` resident surface (BIT-192).
