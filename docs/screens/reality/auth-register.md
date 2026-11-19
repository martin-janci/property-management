---
id: reality/auth-register
name: Register (Reality Portal)
product: reality
sitemapRefs: {}
implementations:
  reality-web:
    route: "/auth/register"
    component: RegisterPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: reality/auth-login
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: reality-frontend
---

# Register (Reality Portal)

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- **Locale-aware auth navigation (PR #3595, merged 2026-11-18):** the register page now navigates with the locale-aware `Link`/`useRouter` from `@/i18n/routing` (next-intl `createNavigation`) instead of `next/link` + `next/navigation`. The plain versions strip the active `/[locale]/…` prefix, bouncing non-default locales (cs/de/en/hu/pl) back to the default. Covers the post-register `router.push('/auth/login')` redirect and the `/auth/login`, `/terms`, `/privacy` links. Matches the fix already applied to the login and SSO callback pages.
- 2026-05-18 — audit: stub created from `frontend/apps/reality-web/src/app/[locale]/auth/register/page.tsx`.

## Agent Log
- 2026-11-18 — agent: screen-map drift reconcile for PR #3595 ("preserve locale in reality-web auth navigation"; merged 2026-11-18). Register-page navigation switched from `next/link` + `next/navigation` to the locale-aware `Link`/`useRouter` from `@/i18n/routing`, so the post-register redirect and internal links keep the active `/[locale]/…` prefix. Documented under Notes > Specific. Frontmatter corrected while here: `component` `AuthRegisterPage` → `RegisterPage` (actual default export) and `apiStatus` `stub` → `partial` (route wires the real reality-server `POST /api/v1/users/register` via `auth-api.register()`); `buildStatus: shipped` unchanged — client-side navigation fix on a shipped screen, no UI/route/endpoint-surface change. Docs-only; `/screens validate` clean.
- 2026-05-18 — agent: created stub for unmapped route.
