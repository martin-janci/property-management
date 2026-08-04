---
id: reality/agency-inquiries
name: Agency Inquiries
product: reality
sitemapRefs: {}
implementations:
  reality-web:
    route: "/agency/inquiries"
    component: AgencyInquiriesPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints:
  - inquiries_list
relatedScreens:
  - id: reality/agency-dashboard
    rel: parent
sharedComponents:
  - next-intl
diagrams: []
useCases:
  - UC-49.5
epics: []
designSources: []
owner: reality-frontend
---

# Agency Inquiries

Lists inquiries received for the signed-in principal's listings/agency (UC-49.5). `AgencyInquiriesPage` wraps `AgencyInquiriesContent` in `ProtectedRoute` + Header/Footer, with a status-filter chip strip (all / pending / responded / scheduled / completed / cancelled) and per-inquiry rows linking back to the listing.

## Notes

### Specific (recent)
- i18n (PR #2636): page fully localized via next-intl namespace `agencyInquiries` (title, subtitle, loading, loadError, empty, `status.*`) across all 6 reality-web locales (en, sk, cs, de, pl, hu). No hardcoded English remains. Status chip/badge labels resolve through dynamic `t('status.${value}')` keys.
- API: wired to the generic `inquiries_list` endpoint (`GET /api/v1/inquiries`) via the reused `useMyInquiries` hook — the API filters by the requesting principal. `apiStatus: partial` because a dedicated `/agencies/{id}/inquiries` endpoint/hook is still pending (see the page's own TODO comment); the current reuse works but is not agency-scoped server-side.
- 2026-05-18 — audit: stub created from `frontend/apps/reality-web/src/app/[locale]/agency/inquiries/page.tsx`.

## Agent Log
- 2026-08-03 — agent: synced to PR #2636 i18n rewrite. Documented `agencyInquiries` next-intl namespace (6 locales); added `inquiries_list` endpoint + `UC-49.5` + `next-intl` shared component; bumped apiStatus stub → partial (real `useMyInquiries` wiring, dedicated agency endpoint pending); replaced stub body with actual page description.
- 2026-05-18 — agent: created stub for unmapped route.
