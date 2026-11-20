---
id: reality/agency-realtors
name: Agency Realtors
product: reality
sitemapRefs: {}
implementations:
  reality-web:
    route: "/agency/realtors"
    component: AgencyRealtorsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens:
  - id: reality/agency-dashboard
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: reality-frontend
---

# Agency Realtors

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/reality-web/src/app/[locale]/agency/realtors/page.tsx`.

## Agent Log
- 2026-11-20 — agent: member management live: `PUT/DELETE /agencies/{id}/members/{memberId}` (owner/manager only, last active owner protected) and `GET .../members/{memberId}/stats`; `useUpdateRealtor`/`useRemoveRealtor`/`useRealtorStats` wired. Resend invitation still not available (no invitation list in UI).
- 2026-11-20 — agent: reality-api-client: realtors list read from `/agencies/{id}/members`, invite posts `/agencies/{id}/invitations`; edit/remove/resend reject with NotAvailableError.
- 2026-05-18 — agent: created stub for unmapped route.
