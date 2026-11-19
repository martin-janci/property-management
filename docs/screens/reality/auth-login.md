---
id: reality/auth-login
name: Login (Reality Portal)
product: reality
sitemapRefs: {}
implementations:
  reality-web:
    route: "/auth/login"
    component: LoginPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: reality/auth-register
    rel: sibling
  - id: reality/auth-forgot-password
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: reality-frontend
---

# Login (Reality Portal)

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- **Locale-aware post-login redirect (PR #3595, merged 2026-11-18):** the post-login `router.replace(redirectTo)` now uses the locale-aware `useRouter` from `@/i18n/routing` (next-intl `createNavigation`) instead of `next/navigation`'s router, which strips the active `/[locale]/…` prefix and bounces non-default locales (cs/de/en/hu/pl) back to the default. The page's `Link`s were already locale-aware; `useSearchParams` (reads `?redirect=`) stays on `next/navigation` — next-intl does not re-export it. The redirect target is still sanitised through `sanitizeRedirectPath()` (see below) before being handed to the locale-aware router. Matches the SSO callback page, which fixed the same locale-drop earlier.
- **Post-login redirect open-redirect hardening (PR #3559, 2026-11-16):** the post-login `router.replace` target comes from the `?redirect=` query param. The old inline guard — `redirectTo.startsWith('/') && !redirectTo.startsWith('//') ? redirectTo : '/'` — blocked protocol-relative `//evil.com` but **not** a backslash bypass like `/\evil.com` (or `/\/\evil.com`), which browsers normalise to a protocol-relative URL → open redirect. The param is now sanitised through the shared `sanitizeRedirectPath()` in `frontend/apps/reality-web/src/lib/sanitize-url.ts` (co-located with `sanitizeHref`): it strips C0 control chars + leading space the URL parser ignores, requires a single leading `/`, rejects `//`, resolves the candidate against a fixed sentinel origin with the WHATWG `URL` parser (normalising `\`→`/` for special schemes) and rejects anything that resolves off-origin, then re-emits path-only (`pathname + search + hash`) — covering protocol-relative, backslash, absolute-URL and dangerous-scheme vectors while preserving a genuine same-origin path's query + fragment. The SSO callback page (`reality/oauth-callback`) was refactored to the same single-sourced helper, dropping its ad-hoc `/\` check. Regression suite: `sanitize-url.test.ts`. Note `sanitizeRedirectPath` intentionally differs from `sanitizeHref` (which allows `//host`) because for a redirect target `//host` *is* the vector.
- 2026-05-18 — audit: stub created from `frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx`.

## Agent Log
- 2026-11-18 — agent: screen-map drift reconcile for PR #3595 ("preserve locale in reality-web auth navigation"; merged 2026-11-18). The post-login `router.replace(redirectTo)` now goes through the locale-aware `useRouter` from `@/i18n/routing` instead of `next/navigation`, keeping the active `/[locale]/…` prefix on the redirect (the page's `Link`s were already locale-aware; `useSearchParams` stays on `next/navigation`). Redirect sanitisation via `sanitizeRedirectPath()` is unchanged. Documented under Notes > Specific. No frontmatter change — `component: LoginPage` and `apiStatus: partial` already correct; `buildStatus: shipped` unchanged (client-side navigation fix). Docs-only; `/screens validate` clean.
- 2026-11-18 — agent: screen-map drift reconcile for PR #3559 ("block backslash open-redirect in reality-web post-login redirect"; merged 2026-11-16). The login page now routes the `?redirect=` param through the shared `sanitizeRedirectPath()` helper (`src/lib/sanitize-url.ts`) instead of the inline `startsWith('/') && !startsWith('//')` guard, closing the `/\evil.com` backslash open-redirect; the SSO callback page was unified onto the same helper. Documented under Notes > Specific. Frontmatter corrected while here: `component` `AuthLoginPage` → `LoginPage` (actual default export) and `apiStatus` `stub` → `partial` (route wires the real reality-server `/api/v1/users/login` call via `auth-api.login()`); `buildStatus: shipped` unchanged — security hardening of a shipped screen, no UI/route/endpoint-surface change. Docs-only; `/screens validate` clean.
- 2026-05-18 — agent: created stub for unmapped route.
