---
id: reality/auth-reset-password
name: Reset Password (Reality Portal)
product: reality
sitemapRefs: {}
implementations:
  reality-web:
    route: "/auth/reset-password"
    component: ResetPasswordPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens:
  - id: reality/auth-forgot-password
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: reality-frontend
---

# Reset Password (Reality Portal)

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- **Locale-aware auth navigation (PR #3595, merged 2026-11-18):** the reset-password page now navigates with the locale-aware `Link`/`useRouter` from `@/i18n/routing` (next-intl `createNavigation`) instead of `next/link` + `next/navigation`. The plain versions strip the active `/[locale]/…` prefix, bouncing non-default locales (cs/de/en/hu/pl) back to the default. Covers the post-reset `router.replace('/auth/login')` redirect and the `/auth/forgot-password` link. `useSearchParams` (reads the `token`) stays on `next/navigation` — next-intl does not re-export it. Matches the fix already applied to the login and SSO callback pages.
- 2026-05-18 — audit: stub created from `frontend/apps/reality-web/src/app/[locale]/auth/reset-password/page.tsx`.

## Agent Log
- 2026-11-18 — agent: screen-map drift reconcile for PR #3595 ("preserve locale in reality-web auth navigation"; merged 2026-11-18). Reset-password-page navigation switched from `next/link` + `next/navigation` to the locale-aware `Link`/`useRouter` from `@/i18n/routing`, so the post-reset redirect and the forgot-password link keep the active `/[locale]/…` prefix (`useSearchParams` stays on `next/navigation`). Documented under Notes > Specific. Frontmatter corrected while here: `component` `AuthResetPasswordPage` → `ResetPasswordPage` (actual default export). `apiStatus: stub` unchanged — `auth-api.confirmPasswordReset()` still throws 501 NOT_IMPLEMENTED pending the reality-server `/api/v1/users/password-reset/confirm` endpoint; `buildStatus: shipped` unchanged (client-side navigation fix, no UI/route/endpoint-surface change). Docs-only; `/screens validate` clean.
- 2026-05-18 — agent: created stub for unmapped route.
