# pm-frontend — 2026-08-01

_Rotating role for the 2026-08-01 routine run. Prior run: 2026-06-10._

## Summary

This run's frontend churn is mostly platform hygiene (admin-web onto `@ppt/ui-kit`,
openapi-ts drift-gate hardening) rather than active-sprint UI stories; the one
substantive gap is that reality-web's listing-detail screen-map hasn't yet been
reconciled with the #2600 inline-script XSS hardening, and PR #2618 doing that
reconciliation is still a draft.

## Next actions

1. **[high]** Land PR #2618 to reconcile `docs/screens/reality/listing-detail.md`
   (and any other reality-web screen touching inline `<script>` / JSON-LD) with
   PR #2600's XSS hardening, adding an Agent Log entry per the reality-web
   screen-map protocol.
2. **[high]** Confirm `buildListingJsonLd`
   (`frontend/apps/reality-web/src/app/[locale]/listings/[slug]/jsonLd.ts`) output
   is escaped/sanitized before being placed in a
   `<script type="application/ld+json">` tag, not just type-guarded.
3. **[medium]** Wire `TenantLifecyclePage.tsx` off the hardcoded `DEMO_TENANT`
   onto the `:id` route param now that it's been refactored onto ui-kit primitives.
4. **[medium]** Close the mobile parity gap for story 6-3 (announcement comment
   threads: mobile shows count only, no read/post UI).
5. **[medium]** Ship frontend permission-authoring UI
   (AccessScopeSelector / RoleSelector / UserSelector) for story 7a-3; backend
   enforcement is done but DocumentUpload still only carries `buildingId`.

## Risks

- **[high · medium]** Screen-map for `reality/listing-detail` is stale relative to
  shipped security fix (#2600), so future agents editing that screen won't see
  accurate XSS-hardening context. — mitigation: merge #2618 promptly.
- **[medium · high]** JSON-LD builder concatenates listing.description and address
  fields directly into an object later serialized into an inline script tag — if
  the hardening in #2600 lives elsewhere (e.g. layout component) and not in
  `jsonLd.ts` itself, this path could remain an XSS vector.
- **[medium · medium]** `TenantLifecyclePage` ships to admin-web with a hardcoded
  demo tenant despite gated capability checks looking production-ready, risking
  a misleading/no-op admin action.
- **[medium · low]** Mobile comment/permission-UI gaps (6-3, 7a-3) persist across
  multiple sprints without a tracked follow-up story.

## Open questions

- What exact diff did PR #2600 make — which file(s)/component(s) implement the
  inline-script XSS fix (`jsonLd.ts`, `layout.tsx`, or a shared sanitizer)?
- Does #2615's openapi-ts drift-gate hardening cover both `@ppt/api-client` and
  `@ppt/reality-api-client` generation in CI, or only one? (yes — both, per PR body)
- Is there a tracked story/issue for `TenantLifecyclePage`'s `:id`-param iteration,
  or is `DEMO_TENANT` intentionally long-lived?
- Are there other reality-web screens beyond listing-detail that render inline
  scripts (e.g. layout preview `postMessage` bridge) needing the same #2618-style
  reconciliation?

## Decisions needed

- Should PR #2618 (screen-map reconciliation) be promoted from draft and merged
  before other reality-web screen work proceeds — owner: pm-frontend /
  reality-web maintainer
