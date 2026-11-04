---
id: ppt/outage-new
name: Create Outage
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/outages/new"
    component: CreateOutagePage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens:
  - id: ppt/outages
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: pm-frontend
---

# Create Outage

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:466`.

## Agent Log
- 2026-11-04 — agent: create/update/start/resolve/cancel bodies and list filters are sent snake_case, and datetime-local values go out as RFC 3339 UTC (was 422 `missing field scheduled_start`) (R3 #9). Error toasts use translated copy (R3 #34). Status unchanged.
- 2026-05-18 — agent: created stub for unmapped route.
