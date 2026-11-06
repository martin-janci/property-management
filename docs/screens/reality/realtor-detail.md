---
id: reality/realtor-detail
name: Realtor Detail (Public)
product: reality
sitemapRefs: {}
implementations:
  reality-web:
    route: "/realtor/[id]"
    component: RealtorDetailPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: reality/realtor
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: reality-frontend
---

# Realtor Detail (Public)

Public-facing realtor profile page (`/realtor/[id]`, default export `AgentProfilePage`). The route fetches the realtor by id from reality-server via the hand-written `@/lib/realtor-api` helpers — `getRealtorPublicProfile` (`GET /api/v1/realtors/{id}/profile`) and `getRealtorReviews` (`GET /api/v1/realtors/{id}/reviews`) — and renders a not-found state (never fabricated data) when the profile 404s. Contact is an honest hand-off: a `tel:` link for phone-exposed realtors and a `mailto:` carrying the realtor's real `email_public` (with the typed message URL-encoded into the body), gated on `show_phone` / `show_email` / `accept_inquiries`; there is no fabricated POST.

`apiStatus: partial` (not `stub`): real wiring is in place, but through the thin hand-written helpers explicitly marked for replacement by the generated client, and the realtor profile/reviews endpoints are not yet tracked in `@ppt/sitemap` — so `endpoints:` is kept empty (listing them would fail `/screens validate`; populate once the sitemap tracks them).

Note: differs from `reality/agent-profile` (the realtor's own profile view) — this is a public-facing realtor profile page. The frontmatter `component: RealtorDetailPage` is the screen-map label; the on-disk default export is `AgentProfilePage`.

## Notes

### Specific (recent)
- 2026-11-06 — screen-map drift reconcile for PR #3313 (merged, "regression tests for realtor/[id] contact-channel honesty"). The earlier 2026-05-18 stub predates PR #3225, which replaced the hardcoded `MOCK_AGENT` + faked contact submit with real client-side fetching (`getRealtorPublicProfile` / `getRealtorReviews`) and an honest `mailto:`/`tel:` hand-off. PR #3313 added three regression tests to `realtor/[id]/page.test.tsx` locking the headline contact-channel gating: (1) the Kontaktovať button is disabled when no channel is exposed and `accept_inquiries` is off; (2) a phone-only realtor gets a `tel:` link and no email form; (3) an email realtor's "Odoslať e-mail" is a `mailto:email_public` with the typed message URL-encoded into the body (never a fabricated POST). Test-only, no runtime change. Flipped `apiStatus: stub → partial` to reflect the real-data wiring (hand-written `@/lib/realtor-api` helpers). `endpoints:` kept empty — realtor profile/reviews ops are absent from `@ppt/sitemap`, so listing them would break `/screens validate`. `buildStatus` unchanged (shipped).
- 2026-05-18 — audit: stub created from `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx`.

## Agent Log
- 2026-11-06 — agent: reconciled screen-map with merged PR #3313 (regression tests for realtor/[id] contact-channel honesty). Documented the real-data fetch + honest mailto/tel hand-off (originally landed in PR #3225) and the three new gating tests; flipped `apiStatus: stub → partial`; `endpoints:` kept empty (not in `@ppt/sitemap`). Docs-only; `/screens validate` clean.
- 2026-05-18 — agent: created stub for unmapped route.
