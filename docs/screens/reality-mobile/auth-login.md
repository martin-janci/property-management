---
id: reality-mobile/auth-login
name: Login (iOS SwiftUI)
product: reality
sitemapRefs:
  reality-web: reality-auth-login
implementations:
  ios-swiftui:
    component: LoginView
    route: Route.login (sheet presentation from MainTabView / AccountView)
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
endpoints:
  - POST /api/v1/auth/sso/validate (via KMP SsoService.validateAndLogin)
relatedScreens:
  - id: reality/auth-login
    rel: web-counterpart
  - id: reality-mobile/account
    rel: parent
  - id: reality-mobile/auth-register
    rel: sibling
sharedComponents: []
diagrams: []
useCases:
  - UC-47
epics:
  - "82"
designSources: []
owner: reality-frontend
---

## Functionality Checklist

- [x] [m] SSO login button — opens propertymanagement:// deep link URL scheme with callback
- [x] [m] Email/password fields with show/hide password toggle
- [x] [m] Form validation (email contains @, password non-empty) gating Login button
- [x] [m] ProgressView overlay while loading
- [x] [m] Error banner with warning icon on auth failure
- [x] [m] "Forgot password" button (placeholder, no action wired)
- [x] [m] Register link → Route.register
- [x] [m] Cancel toolbar button → dismiss sheet
- [x] [m] Pending destination consumed after successful login
- [ ] [m] Register screen (Route.register destination is stub Text view in MainTabView)
- [ ] [m] Forgot password flow (not implemented)
- [ ] [m] Email/password login always throws ssoRequired error (Reality Portal is SSO-only)

## States

- **Idle**: SSO button + divider + email/password form.
- **Loading**: ProgressView shown in Login button.
- **Error**: Error banner displayed below form.
- **Success**: Sheet dismissed; pending navigation consumed.

## Notes

### Broader context

Presented as a sheet from `MainTabView.handleTabChange()` when an unauthenticated user taps an auth-gated tab (Favorites, Inquiries, Account). Also navigable via `Route.login` pushed onto `accountPath`. Reality Portal is SSO-only — email/password `login()` always throws `AuthError.ssoRequired`. The email form is UI-complete for potential future direct login.

### Specific (recent)

- SSO initiates by opening `propertymanagement://sso?callback=realityportal://sso`. The Property Management app must be installed; no fallback if not installed.
- Token return flow: PM app redirects to `realityportal://sso?token=<token>` → handled by `RealityPortalApp.handleIncomingURL()` → `AuthManager.loginWithSsoToken()`.
- Localization strings use `String(localized:)` — keys: `sign_in`, `welcome_to_reality_portal`, `sign_in_description`, `sign_in_with_pm`, `sso_description`, `or`, `email`, `password`, `forgot_password`, `no_account_prompt`, `create_account`.

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/Auth/LoginView.swift (epic-82 story 82.5). SSO-only constraint and missing register/forgot-password flows noted.
