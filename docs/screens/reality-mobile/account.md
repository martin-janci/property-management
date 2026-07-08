---
id: reality-mobile/account
name: Account (iOS SwiftUI)
product: reality-mobile
sitemapRefs: {}
implementations:
  ios-swiftui:
    component: AccountView
    route: Tab.account / Route.account
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: reality/account
    rel: web-counterpart
  - id: reality-mobile/auth-login
    rel: child
  - id: reality-mobile/saved-searches
    rel: child
sharedComponents: []
diagrams: []
useCases:
  - UC-47
epics:
  - Epic-82
designSources: []
owner: reality-frontend
---

## Functionality Checklist

- [x] [m] Tab root requiring auth (Tab.account.requiresAuth = true)
- [x] [m] Authenticated: avatar initials circle, user name, email
- [x] [m] Navigation links: Profile (→ Route.profile stub), Saved Searches (→ Route.savedSearches), Settings (→ Route.settings stub)
- [x] [m] Sign Out button → `authManager.logout()` + reset navigation
- [x] [m] Unauthenticated: sign-in CTA presenting LoginView sheet
- [x] [m] App version display (Configuration.shared.fullVersionString)
- [ ] [m] Profile editing screen (Route.profile destination is stub Text view)
- [ ] [m] Settings screen (Route.settings destination is stub Text view)
- [ ] [m] Avatar image upload (not implemented)

## States

- **Authenticated**: User info header + action list + sign-out button.
- **Unauthenticated**: Centered "sign_in_to_access_account" message + Sign In button.

## Notes

### Broader context

Account tab root. Auth state driven by `AuthManager.isAuthenticated` (`@Observable`, injected via `.environment`). Sign-out calls `AuthManager.logout()` which clears Keychain tokens and KMP SsoService session, then resets `NavigationCoordinator`.

### Specific (recent)

- Profile and Settings routes are registered in `Route.swift` and handled by `NavigationCoordinator` but their destination views are placeholder `Text` views in `MainTabView.destinationView()`.
- App version sourced from `Bundle.main.infoDictionary` — reads `CFBundleShortVersionString` and `CFBundleVersion`.

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/Account/AccountView.swift (epic-82 story 82.5). Profile/Settings stubs noted.
- 2026-05-28 — agent: fix(#581) added missing Agent Log entry that PR #554 omitted (CLAUDE.md screen-map Rule A.3); flagged dropped operationIds on home + saved-searches in Notes > Specific (recent) pending @ppt/sitemap extension.
