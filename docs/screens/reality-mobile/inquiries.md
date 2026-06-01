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
- [ ] [m] InquiryDetail full screen (Route.inquiryDetail destination is stub Text view in MainTabView)
- [ ] [m] Real-time updates / WebSocket for new replies (not implemented)
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

- `Route.inquiryDetail` and `Route.newInquiry` are handled in `NavigationCoordinator.navigate()` and appended to `inquiriesPath`, but `MainTabView.destinationView()` renders them as placeholder `Text` views.
- `InquiryStatus` Swift typealias maps from KMP `shared.InquiryStatus` enum (pending/responded/closed).

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/Inquiries/InquiriesView.swift (epic-82 story 82.5). Detail and newInquiry stubs noted.
- 2026-05-28 — agent: fix(#581) added missing Agent Log entry that PR #554 omitted (CLAUDE.md screen-map Rule A.3); flagged dropped operationIds on home + saved-searches in Notes > Specific (recent) pending @ppt/sitemap extension.
