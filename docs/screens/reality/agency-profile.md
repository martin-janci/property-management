---
id: reality/agency-profile
name: Agency Profile (public)
product: reality
implementations:
  reality-web:
    component: AgencyProfilePage
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
  mobile-native:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
relatedScreens:
  - id: reality/agent-profile
    rel: sibling
  - id: reality/agency-dashboard
    rel: sibling
  - id: reality/listing-detail
    rel: child
sharedComponents:
  - portal-header
  - portal-footer
  - listing-card
useCases:
  - UC-49
  - UC-50
endpoints: []
epics: []
diagrams: []
owner: reality-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Hero
- [x] [w] Cover band (agency brand colors) + logo + H1 (agency name) + verified badge + city/country

### Section · O kancelárii
- [x] [w] Agency description paragraph (shown when present)

### Section · Náš tím
- [x] [w] Grid of team member cards (photo + name + title), each linking to `reality/agent-profile`

### Section · Aktívne ponuky
- [x] [w] Grid of active listing cards linking to `reality/listing-detail`; empty placeholder when none

### Sidebar · Kontakt
- [x] [w] Phone (tel:) + email (mailto:) + website link + license number

### Footer
- [x] [w] Standard portal footer

## States

- **Default**: agency + team + active listings
- **Empty**: "Žiadne aktívne ponuky." placeholder when no active listings
- **Loading**: "Načítavam…" while the by-slug query resolves
- **Error / not found**: "Kancelária sa nenašla." when the slug has no agency

## Notes

### Broader context

Public-facing agency profile resolved by slug (`/agency/[slug]`). Mirrors the
`reality/agent-profile` pattern: a `'use client'` component fetching data via
`@ppt/reality-api-client` hooks. Resolves the agency through
`useAgencyBySlug` (GET `/api/v1/agencies/by-slug/{slug}`, response wrapped in
`AgencyResponse { agency }`), then loads team via `useRealtors(agency.id)` and
active listings via `useAgencyListings(agency.id, { status: 'active' })`.

### Specific (recent)

- The by-slug endpoint unwraps `.agency` (unlike the bare by-id reads).
- A bare `useAgencyMembers(agencyId)` hook also exists for the raw
  `/agencies/{id}/members` endpoint, but the team grid uses `useRealtors` for
  richer profile data (name/photo/title) since the members payload is
  membership-only.

## Agent Log

<!-- newest entries on top -->

- 2026-06-03 — agent: created public agency profile route `agency/[slug]`; added `useAgencyBySlug` + `useAgencyMembers` hooks (Issue #978)
