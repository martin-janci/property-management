---
id: reality-mobile/inquiries
name: Inquiries (iOS SwiftUI)
product: reality-mobile
sitemapRefs:
  reality-web: reality-inquiries
implementations:
  ios-swiftui:
    component: InquiriesView
    route: Tab.inquiries / Route.inquiries
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
endpoints:
  - inquiries_list
  - inquiries_contact
relatedScreens:
  - id: reality/inquiries
    rel: web-counterpart
  - id: reality-mobile/listing-detail
    rel: sibling
  - id: reality-mobile/account
    rel: sibling
sharedComponents: []
diagrams: []
useCases:
  - UC-46
epics:
  - "82"
designSources: []
owner: reality-frontend
---

## Functionality Checklist

- [x] [m] Tab root screen requiring auth (Tab.inquiries.requiresAuth = true)
- [x] [m] Load inquiries via KMP `inquiryRepository.getInquiries()`
- [x] [m] Inquiry list rows: listing title, last message, status badge, date
- [x] [m] Status badges: Pending (warning), Replied (success), Closed (neutral)
- [x] [m] Tap row → Route.inquiryDetail(id:) — navigates to stub detail
- [x] [m] Loading state
- [x] [m] Empty state (envelope icon + "no_inquiries" + Browse button)
- [x] [m] Error state
- [x] [m] Badge count on tab bar driven by NavigationCoordinator.inquiriesBadgeCount
- [x] [m] InquiryDetail full screen: InquiryDetailView.swift — conversation thread (original msg + realtor replies as MessageBubble), reply composer, listing-context card, closed-inquiry guard
- [ ] [m] Real-time updates / WebSocket for new replies — explicitly out-of-scope for epic-82; fetch-on-load + reload-after-reply used instead
- [ ] [m] New Inquiry creation form (Route.newInquiry destination is stub Text view)

## States

- **Loading**: ProgressView centered.
- **Error**: Warning icon + message + Retry.
- **Empty**: Envelope SF symbol + "no_inquiries" + "Browse properties" CTA.
- **Success**: List of InquiryRowCard items.

## Notes

### Broader context

Auth-gated tab. Maps KMP `Inquiry` → Swift `InquiryPreview` via `KMPBridge.toInquiryPreview()`. Badge count (`inquiriesBadgeCount` on `NavigationCoordinator`) is currently not auto-updated; would require background polling or WebSocket (UC-19 real-time feature).

### Specific (recent)

- `Route.inquiryDetail` destination is now `InquiryDetailView(inquiryId:)` — the stub Text placeholder noted in the gap-82-1 audit has been replaced. Loads via `GET /api/v1/inquiries/{id}`, replies via `POST /api/v1/inquiries/{id}/replies`.
- `Route.newInquiry` destination is still a stub Text view in `MainTabView.destinationView()`.
- `InquiryStatus` Swift typealias maps from KMP `shared.InquiryStatus` enum (pending/responded/closed); Swift UI uses `.replied` for KMP `.responded` (name difference is intentional and documented in `InquiryDetailView.swiftInquiryStatus(from:)`).
- No streaming/WebSocket: conversation thread is loaded once on `.task {}` and reloaded after each `sendReply()`. `ApiConfig.wsUrl` exists in KMP but is not wired to the inquiry flow.
- **Android (KMP/Compose) parity gap.** The shared `InquiryRepository` now exposes the full inquiry-reply + scheduling surface — `replyToInquiry()` (POST `/api/v1/inquiries/{id}/replies`), `scheduleViewing()` (POST `/api/v1/viewings`), plus `getViewings()`/`cancelViewing()` — all with `commonTest` contract coverage (`InquiryModelsContractTest`). The earlier deferral note ("inquiry-reply API + scheduling backend not yet present") is therefore stale and has been corrected. What remains for Android is UI-only: `androidApp/.../ui/inquiries/InquiriesScreen.kt` still renders only the messages/viewings **list** — the inline `KmpInquiryThreadScreen` thread view + reply composer + scheduling calendar are not wired (iOS ships them via `InquiryDetailView.swift`). Frontmatter stays `ios-swiftui`-only until `@ppt/sitemap` gains a reality-mobile Android target (same convention as `reality-mobile/search`).

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/Inquiries/InquiriesView.swift (epic-82 story 82.5). Detail and newInquiry stubs noted.
- 2026-05-28 — agent: fix(#581) added missing Agent Log entry that PR #554 omitted (CLAUDE.md screen-map Rule A.3); flagged dropped operationIds on home + saved-searches in Notes > Specific (recent) pending @ppt/sitemap extension.
- 2026-06-10 — agent: verify task 82-5; confirmed InquiryDetailView.swift fully implemented (conversation thread + reply composer); no streaming/WebSocket (fetch-on-demand, not in epic-82 scope); PushNotificationManager + tests confirmed; coverage.json updated to done/high.
- 2026-07-01 — agent: verify task 82-5 (coverage phase4). Re-audited the KMP + iOS slices. iOS covered (InquiryDetailView.swift thread+composer, SendInquiryView.swift). The KMP agent-log deferral ("inquiry-reply API + scheduling backend not present") is now RESOLVED at the shared layer — `InquiryRepository.replyToInquiry`/`scheduleViewing`/`getViewings`/`cancelViewing` all ship with `InquiryModelsContractTest` coverage. Corrected the stale KDoc in `androidApp/.../ui/inquiries/InquiriesScreen.kt` and recorded the remaining gap in Notes>Specific. Remaining Android gap is UI-only (inline Compose thread + calendar) — genuine net-new feature work, not verify-to-done, and not compile-verifiable offline (AGP plugin unresolvable in the Linux sandbox). Docs+comment-only change; brace-balanced, scope-check clean. Left buildStatus in-progress (Android thread UI still open); iOS remains covered.
