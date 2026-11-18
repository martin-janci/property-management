---
id: ppt/fault-detail
name: Fault Detail
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/faults/:faultId"
    component: FaultDetailPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
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

# Fault Detail

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-11-18 — **auth guard (PR #3606)**: `/faults/:faultId` is now wrapped in `<ProtectedRoute>` (auth-only, not role-gated) in `routes/groups/faults.tsx` — previously a bare `<Route>` that rendered the detail chrome for an anonymous deep-link. The reporter/resident and the manager can read fault detail, but both must be signed in; an anonymous deep-link is now redirected to `/login` (return URL captured). `buildStatus`/`apiStatus` unchanged. Guard regression-covered by `routes/groups/core-route-groups-auth-guard.route.test.tsx`.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:499`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-06-03 — agent: confirmed FaultDetailPageRoute wired to useFault + triage/resolve/confirm/reopen/comment/attachment hooks (#970.1); apiStatus -> complete.
- 2026-08-18 — agent: screen-map-drift-pr-2647 — noted i18n update in PR #2647: `FaultDetailPageRoute` in `frontend/apps/ppt-web/src/routes/groups/faults.tsx` now renders its missing-param fallback via `t('errors.faultNotFound', 'Fault not found')` (was hardcoded English); `errors.faultNotFound` key added to all locale bundles. No route or component change.

## Agent Log

<!-- newest entries on top -->

- 2026-11-18 — agent: screen-map-drift-pr-3606-ppt — reconciled with merged PR #3606: the previously-bare `/faults/:faultId` detail route is now wrapped in `<ProtectedRoute>` (auth-only) in `routes/groups/faults.tsx`; anonymous deep-links redirect to `/login` (return URL captured). No route/component/status change — docs-only.

- 2026-11-09 — agent: gap-audit-r7 — photo upload enabled (presigned documents/upload-url → PUT → POST attachments; server now requires an org-scoped storage key and returns presigned read URLs); reporter can withdraw a `new` fault (POST /faults/{id}/withdraw); attachment contract fixed (client read file_url/file_name, server sends storage_url/original_filename).

- 2026-11-05 — agent: screen-map-drift-pr-3244-ppt — reconcile i18n drift from PR #3244. Detail-view components `TriageFaultDialog` and `FaultTimeline` moved hardcoded English into react-i18next (`faults.*`) across all 6 locales — triage dialog fields + AI-suggestion copy and timeline event labels — with fault enum labels resolved via the shared `features/faults/utils/faultLabels.ts` key maps. i18n only — no route/component/endpoint/status change (builds on the PR #2647 fallback i18n already logged).
- 2026-11-04 — agent: hid Add photo control — POST /faults/{id}/attachments takes JSON metadata (storage_url), the multipart client call could never succeed
