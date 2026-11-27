---
id: reality/account-password
name: Account Password
product: reality
sitemapRefs: {}
implementations:
  reality-web:
    route: "/account/password"
    component: ChangePasswordPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens:
  - id: reality/account
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: reality-frontend
---

# Account Password

Authenticated change-password form (UC-47.4 variant) at `frontend/apps/reality-web/src/app/[locale]/account/password/page.tsx`. The default export is `ChangePasswordPage`; the inner form component is `ChangePasswordForm`. Client-side validation (required fields, 8-char minimum, new≠current, confirm match) is fully built and all copy is localized via `useTranslations('pages.accountPassword.*')` across sk/cs/de/en/pl/hu.

`apiStatus: stub` is accurate: the submit handler calls `changePassword` from `@/lib/auth-api`, but that helper is a **client-side stub** — it unconditionally throws `AuthApiError(code: 'NOT_IMPLEMENTED')` with a `TODO: wire when reality-server exposes /api/v1/users/me/password`. No backend password-change endpoint exists yet, so a real submission always surfaces the "not available yet" path.

## Notes

### Specific (recent)
- 2026-11-27 — PR #3793 ("route account error copy through the i18n catalog") stopped rendering the raw developer-facing English `AuthApiError.message` directly. The submit handler now routes the non-401 error through the new `useAuthErrorMessage` hook (`@/lib/auth-error`), which maps the error `code` onto the `authError.*` next-intl namespace — so the `NOT_IMPLEMENTED` "Changing your password … is not available yet. Please contact support." copy is now localized across all six locales instead of leaking English to cs/de/hu/pl/sk visitors. The 401 (wrong current password) branch still renders the pre-existing localized `t('currentIncorrect')` field error. Regression pinned in `page.i18n.test.tsx`. Docs/i18n + hook change only; no route, build-surface, or API-surface change — `apiStatus` stays `stub` (backend endpoint still absent).
- 2026-05-18 — audit: stub created from `frontend/apps/reality-web/src/app/[locale]/account/password/page.tsx`.

## Agent Log
<!-- newest entries on top -->
- 2026-11-27 — agent: screen-map drift reconcile for PR #3793 ("route account error copy through the i18n catalog"). Corrected `component` (`AccountPasswordPage` → actual default export `ChangePasswordPage`). Documented the new `useAuthErrorMessage`/`authError.*` i18n routing of the `NOT_IMPLEMENTED` error copy and clarified that `apiStatus: stub` is correct because `changePassword` is a client-side stub throwing `NOT_IMPLEMENTED` pending a reality-server `/api/v1/users/me/password` endpoint. Noted UC-47.4 (per the page's own doc comment) for future human frontmatter wiring. Docs-only change; `buildStatus: shipped` unchanged.
- 2026-05-18 — agent: created stub for unmapped route.
