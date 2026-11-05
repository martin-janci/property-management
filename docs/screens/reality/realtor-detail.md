---
id: reality/realtor-detail
name: Realtor Detail (Public)
product: reality
sitemapRefs: {}
implementations:
  reality-web:
    route: "/realtor/[id]"
    component: AgentProfilePage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: reality/realtor
    rel: parent
  - id: reality/agent-profile
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: reality-frontend
---

# Realtor Detail (Public)

Public-facing realtor profile page at `/realtor/[id]`. The route is served by the
`AgentProfilePage` component (`frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx`),
which fetches the realtor's public profile + reviews client-side via TanStack Query
against the real reality-server endpoints and degrades missing data to empty / not-found
states rather than fabricating it.

**Overlaps with `reality/agent-profile`.** Both screen-maps describe this same
`/realtor/[id]` route and `AgentProfilePage` component — `realtor-detail` is the older
audit stub (2026-05-18); `agent-profile` carries the reconciled checklist/states/notes
after PR #3225 wired the real data. The page's own header comment maps itself to
`docs/screens/reality/agent-profile.md`. Consolidating the two (or demoting one to a
redirect/alias) is open drift — see Agent Log.

## Notes

### Specific (recent)

- 2026-11-05 — i18n'd via next-intl (PR #3293 / issue #3283). All user-visible chrome —
  loading / not-found states, stat labels, section headings, contact details, and the
  contact modal — now routes through `useTranslations('pages.agentProfile')` + `useLocale`,
  with real translations in all six supported locales (sk, cs, de, en, pl, hu). Previously
  the page rendered hardcoded Slovak literals, so cs/de/en/hu/pl visitors saw Slovak copy.
  Interpolated keys (`experience {count}`, `license {number}`, `memberSince {date}`,
  `modalTitle {name}`) carry their placeholders in every locale. A per-locale coverage
  guard was added (`src/i18n/agent-profile-messages.test.ts`).
- Wired to real data (PR #3225): `GET /api/v1/realtors/{id}/profile` and
  `GET /api/v1/realtors/{id}/reviews` via TanStack Query (API helpers in
  `src/lib/realtor-api.ts`). apiStatus is `partial` (not `complete`): there is no
  realtor-scoped public listings endpoint and no realtor-direct inquiry endpoint, so the
  listings grid is omitted and the contact modal hands off to the realtor's real channels
  (mailto / tel) instead of a server submit.
- `endpoints:` is kept empty because the two realtor endpoints above are not registered in
  `frontend/packages/sitemap/src/data/reality-server/index.ts`, and `/screens validate`
  rejects unknown endpoint ids. Registering them is a separate sitemap-owner change (same
  follow-up noted on `reality/agent-profile`).
- 2026-05-18 — audit: stub created from `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx`.

## Agent Log
- 2026-11-05 — agent: reflected PR #3293 — page i18n'd via next-intl across six locales (sk/cs/de/en/pl/hu); apiStatus stub→partial, component RealtorDetailPage→AgentProfilePage (the real exported component), added agent-profile sibling link; flagged the realtor-detail / agent-profile overlap (same route+component) as open drift.
- 2026-05-18 — agent: created stub for unmapped route.
