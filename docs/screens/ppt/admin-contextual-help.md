---
id: ppt/admin-contextual-help
name: Admin Contextual Help Sidebar
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "* (AdminLayout topbar — global)"
    component: HelpSidebar, HelpTooltip, useContextualHelp
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: n/a
endpoints: []
relatedScreens:
  - id: ppt/admin-platform-health
    rel: sibling
  - id: ppt/admin-system-announcements
    rel: sibling
  - id: ppt/admin-oauth-clients
    rel: sibling
sharedComponents:
  - HelpSidebar
  - HelpTooltip
  - useContextualHelp
diagrams: []
useCases: []
epics:
  - Epic-10B-7
designSources: []
owner: pm-frontend
---

## Functionality Checklist

- [x] "?" button in AdminLayout topbar opens/closes the HelpSidebar
- [x] HelpSidebar shows the article matched to the current page route
- [x] Slide-over panel with backdrop; closes on Escape key and backdrop click
- [x] Article body rendered inline (bold, code, bullet lists, tables) — no external markdown parser
- [x] Footer link to external docs URL when article has one
- [x] HelpTooltip component for inline field-level help (hover/focus)
- [x] Tooltips added to Feature Flags and Platform Settings pages
- [x] Static articles for all 12 admin pages in `src/help/articles.ts`
- [x] i18n keys for EN / SK / CS

## States

- **Closed**: "?" button shows in topbar at all times; not active-styled
- **Open**: "?" button highlighted (blue tint); panel slides in from right with backdrop
- **No article**: Falls back to Dashboard article (safe default)

## Notes

### Broader context

Part of Epic 10B Story 10B.7. Pure frontend feature — no backend endpoint
required. Static markdown content lives in `frontend/apps/admin-web/src/help/articles.ts`.
The `getArticleForPath(pathname)` function resolves the longest-prefix match
so child routes (e.g. `/identity/memberships/123`) inherit their parent's article.

### Specific (recent)

- 2026-05-26 — agent: gap-10b-7-contextual-help-ui — initial implementation;
  HelpSidebar, HelpTooltip, useContextualHelp wired into AdminLayout topbar;
  13 static articles covering all admin sections; i18n EN/SK/CS; no backend dep.

## Agent Log

<!-- newest entries on top -->
- 2026-06-25 — agent: 10b-7-reconcile — verified feature shipped across backend
  (/api/v1/help + HelpRepository + migration 00034 + help_tests.rs via PR #844),
  admin-web (HelpSidebar/HelpTooltip/useContextualHelp), and mobile
  (HelpCenterScreen/ContextualHelp). Reconciled sprint-status + story to done.
- 2026-05-26 — agent: gap-10b-7-contextual-help-ui — initial implementation of
  contextual help sidebar and inline tooltip components for admin-web; wired
  globally via AdminLayout; static article content for all 12 admin pages.
