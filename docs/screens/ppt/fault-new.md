---
id: ppt/fault-new
name: Create Fault
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/faults/new"
    component: CreateFaultPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/faults-list
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-4
designSources: []
owner: pm-frontend
---

# Create Fault

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

Note: a `ppt/report-fault` screen-map already exists (mobile-oriented). This stub covers the web `/faults/new` route specifically.

## Notes

### Specific (recent)
- 2026-11-18 — **auth guard (PR #3606)**: `/faults/new` is now wrapped in `<ProtectedRoute>` (auth-only, not role-gated) in `routes/groups/faults.tsx` — previously a bare `<Route>` that rendered the form chrome for an anonymous visitor. Residents and managers can file faults, but both must be signed in; an anonymous visitor is now redirected to `/login` (return URL captured). `buildStatus`/`apiStatus` unchanged. Guard regression-covered by `routes/groups/core-route-groups-auth-guard.route.test.tsx`.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:498`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-06-02 — agent (gap-sweep): wired CreateFaultPageRoute to `useCreateFault` + `useBuildings` in App.tsx (was a no-op toast). Maps FaultFormData→CreateFaultRequest. apiStatus stub→partial — photo upload (File[]→useAddAttachment) still TODO.

## Agent Log

<!-- newest entries on top -->

- 2026-11-18 — agent: screen-map-drift-pr-3606-ppt — reconciled with merged PR #3606: the previously-bare `/faults/new` route is now wrapped in `<ProtectedRoute>` (auth-only) in `routes/groups/faults.tsx`; anonymous visitors redirect to `/login` (return URL captured). No route/component/status change — docs-only.

- 2026-11-09 — agent: gap-audit-r7 — photo picker re-enabled; photos are attached after create via the presigned flow, failures reported in a warning toast.

- 2026-11-05 — agent: screen-map-drift-pr-3244-ppt — reconcile i18n drift from PR #3244. `CreateFaultPage` moved hardcoded English labels/placeholders/validation copy into react-i18next (`faults.*`) across all 6 locales. i18n only — no route/component/endpoint/status change.
- 2026-11-04 — agent: unit dropdown filled from GET /buildings/{id}/units for the selected building; photo-first picker disabled (attachments endpoint needs an already-stored object, no fault presign flow) so photos are no longer silently dropped
