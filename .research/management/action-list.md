# Action list

_Generated: 2026-09-07T04:35:01Z — regenerated from `action-list.json`._

| ID | Priority | Owner | Action |
|---|---|---|---|
| `code-review-mobile-native-kmp-portfolio-analytics-caps-100` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on large portfolios |
| `code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit — up to 100 parallel GETs from a mobile dev |
| `code-review-mobile-native-kmp-cancellation-swallowed` | low | pm-backend | mobile-native-kmp: shared repositories swallow CancellationException in catch(e: Exception), breaking coroutine cancellation and showing spurious errors |
| `code-review-mobile-native-kmp-ssoservice-untested` | low | pm-qa | mobile-native-kmp: SsoService (deep-link token exchange, login, password reset, session restore) has zero direct tests |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries  |
| `code-review-mobile-native-kmp-httpclient-no-timeout` | low | pm-backend | mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — every suspend API call can hang indefinitely on Android + iOS |
| `code-review-mobile-native-kmp-create-listing-not-wired` | low | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `pm-devops-unblock-mobile-native-cloud-builds` | high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — currently 7/8 open backlog items are structurally unclaimable in cloud, forcing Tier-1d gen |
| `screen-map-drift-pr-2894-reality` | low | pm-qa | screen-map-drift: PR #2894 touched reality-web routes without updating docs/scre |
| `code-review-mobile-rn-dashboard-layout-first-visit-default` | low | pm-frontend | useDashboardLayout.ts:37-58 -- the Phase-2 'background fetch' does `const fresh = await apiRequest<ResolvedScreen>('/api/v1/layout/resolved/${screen}?platform=m |
| `code-review-mobile-rn-reportfault-perm-4xx-false-saved` | low | pm-frontend | ReportFaultScreen.tsx:312-337 -- the online submit path wraps `apiRequest<CreateFaultResult>('/api/v1/faults', ...)` in try/catch and, in the catch (:327-334),  |
| `code-review-ppt-web-core-authed-query-roots-drift` | low | pm-frontend | logout() (AuthContext.tsx:573-596) purges user-scoped TanStack Query cache by iterating a hand-maintained string allowlist: `for (const root of AUTHED_QUERY_KEY |
| `code-review-ppt-web-core-financial-dash-false-zeros` | low | pm-frontend | frontend/apps/ppt-web/src/routes/groups/financial.tsx:70-136 (FinancialDashboardPageRoute): three useQuery calls (getARAgingReport, getOverdueInvoices, listInvo |
| `code-review-ppt-web-core-locale-bundles-incomplete` | low | pm-frontend | Systemic translation-coverage gap: every non-English ppt-web locale bundle is roughly HALF-populated relative to en.json. Measured by flattening each messages/* |
| `code-review-ppt-web-core-mfa-verify-hardcoded-path` | low | pm-frontend | frontend/apps/ppt-web/src/App.tsx:72 — MfaWrapper.verify() calls raw `fetch('/api/v1/auth/mfa/verify', ...)` with a HARDCODED root-relative path, bypassing the  |
| `code-review-ppt-web-core-rentals-noop-action-handlers` | low | pm-frontend | frontend/apps/ppt-web/src/routes/groups/rentals.tsx:517-521 (GuestRegistrationPageRoute): the route fetches real guest data (rentalsApiListGuests, line 483) and |
| `code-review-ppt-web-core-route-groups-no-protectedroute` | medium | pm-security | ppt-web has NO global auth gate: App.tsx:272-275 renders `<RouteErrorBoundary><Suspense><AppRoutes/></Suspense></RouteErrorBoundary>` inside the always-rendered |
| `code-review-ppt-web-ui-document-hooks-hardcoded-slovak-toasts` | low | pm-frontend | features/documents/hooks/useDocumentDownload.ts:46-49 and features/documents/hooks/useMoveDocumentWithToast.ts:25-38 hardcode Slovak-only toast strings ('Stiahn |
| `code-review-ppt-web-ui-facilities-ismanager-stub-true` | low | pm-frontend | frontend/apps/ppt-web/src/features/facilities/pages/FacilitiesPage.tsx:36-41 — `useIsManager()` is a hardcoded stub: `// Placeholder: Returns true during develo |
| `code-review-ppt-web-ui-portfolio-dashboard-deadnav` | low | pm-frontend | PortfolioDashboardPage.tsx:266-268 -- every PropertyCard rendered in the portfolio dashboard grid is given an onClick handler whose entire body is a comment: `o |
| `code-review-ppt-web-ui-registry-api-client-no-auth` | low | pm-frontend | RegistryPage.tsx:17-21 and RegistryRulesPage.tsx:16-20 both instantiate `const registryApi = createRegistryApi({ baseUrl: API_BASE_URL, // accessToken and tenan |
| `code-review-ppt-web-ui-untranslated-feature-pages` | low | pm-frontend | Systemic i18n completeness gap: 67 of 196 *Page.tsx / *List.tsx files under frontend/apps/ppt-web/src/features have NO useTranslation import at all — entire fea |
| `code-review-reality-web-agency-branding-save-no-error` | low | pm-frontend | AgencyBranding.tsx handleSave (L79-95) does `await updateBranding.mutateAsync({ agencyId, data: { logo, coverImage, primaryColor, ... } })` with NO try/catch, t |
| `code-review-reality-web-cmp-ls-unchecked` | low | pm-frontend | frontend/apps/reality-web/src/lib/comparison-context.tsx:40-41 — `const parsed = JSON.parse(stored); setListings(parsed);` on mount trusts the localStorage payl |
| `code-review-reality-web-comparison-unvalidated-listing-cast` | low | pm-frontend | Client-side shared-comparison loader trusts an unvalidated network body across the API boundary, inconsistent with the app's own SSR normalizer discipline. Comp |
| `code-review-reality-web-edit-price-zero` | low | pm-frontend | frontend/apps/reality-web/src/app/[locale]/account/listings/[id]/edit/page.tsx:198 — handleSave does `price: typeof form.price === 'number' ? form.price : Numbe |
| `code-review-reality-web-login-open-redirect-backslash` | medium | pm-security | frontend/apps/reality-web/src/app/[locale]/auth/login/page.tsx:33,72-73 — the login form reads its post-login destination from a user-controlled query param (`c |
| `code-review-reality-web-profile-page-mock-data` | low | pm-frontend | frontend/apps/reality-web/src/app/[locale]/profile/page.tsx:17,65,268,346,384 — the signed-in user's own profile route renders MOCK_PROFILE / MOCK_LISTINGS / MO |
| `code-review-reality-web-realtor-detail-modal-no-error` | low | pm-frontend | RealtorManagement.tsx RealtorDetailModal has three destructive/state-changing handlers that await mutateAsync with NO try/catch: handleSave (updateRealtor.mutat |
| `code-review-reality-web-realtor-profile-hardcoded-slovak` | low | pm-frontend | frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx hardcodes Slovak-only UI strings — ':114 "✓ Overený maklér" (verified-realtor badge), :227 "Akt |
| `code-review-reality-web-realtor-profile-mock-unwired` | low | pm-frontend | frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx:16-20 — the public agent-profile route renders MOCK_AGENT / MOCK_AGENT_LISTINGS / MOCK_AGENT_RE |
