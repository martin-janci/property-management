# Reality Portal — mobile-native Analytics Parity Audit

> Scope: event-tracking parity between the **web** Reality Portal
> (`frontend/apps/reality-web`) and the **native** Reality Portal
> (`mobile-native/` — Kotlin Multiplatform: Android Compose + iOS SwiftUI).
> Focus funnels: **listing view**, **search**, **contact-inquiry**.
> Audience: data / analytics engineers and the mobile-native owners (pm-mobile).
> Grounded in code as of branch
> `auto-impl/data-mobile-native-analytics-parity-2026-345a201a` (from `dev`).

This is an **audit + target-contract document**. It records what each platform
emits today, marks every missing hook **[gap]**, and points at the exact code
that would carry it — so the schema is a target the mobile-native code can be
wired against, not a claim that any field is already on the wire from native.

## 1. Headline finding

`mobile-native` emits **zero** analytics/telemetry events, on **both** Android
and iOS. There is no event bus, no `trackEvent`-equivalent, no dataLayer, and no
analytics SDK anywhere under `mobile-native/`. The three target funnels
(listing view, search, contact-inquiry) are therefore **entirely uninstrumented
on native**, so every native listing view, search, and inquiry is invisible to
the conversion metrics the web portal already feeds.

A secondary finding: on the **web** side, only the **listing-view** funnel is
instrumented today. **Search** and **contact-inquiry** are *also* uninstrumented
on reality-web — so for those two funnels the parity target does not yet exist
and must be defined here rather than merely mirrored (see §4.2, §4.3).

## 2. The parity reference (what reality-web emits)

reality-web ships a lightweight, dependency-free event bus and exactly one
instrumented funnel event:

| Piece | Code |
|-------|------|
| Transport-agnostic event bus (`trackEvent`, sinks, GTM `dataLayer` push) | `frontend/apps/reality-web/src/lib/analytics.ts` |
| Anonymous per-tab session context (`sessionId` + `referrer`) | `frontend/apps/reality-web/src/lib/analytics-session.ts` |
| Canonical `listing.viewed` event + `deriveListingViewContext` (view-source / filter-state) | `frontend/apps/reality-web/src/components/listings/listingAnalytics.ts` |
| Emit site — one event per real listing view, deduped by listing id, suppressed in layout-preview | `frontend/apps/reality-web/src/components/listings/ListingDetailContent.tsx` |

`admin-web` carries a byte-for-byte sibling of the same bus
(`frontend/apps/admin-web/src/lib/analytics.ts`) plus signup/onboarding events —
confirming the intended house pattern: a tiny `trackEvent(name, props)` bus with
dot-namespaced event names and `snake_case` properties, feeding a deploy-time
tag manager via `window.dataLayer`.

### 2.1 `listing.viewed` schema (the one real reference event)

Emitted from `trackListingViewed(...)`:

| Property | Type | Source |
|----------|------|--------|
| `listing_id` | string | listing id |
| `slug` | string | listing slug |
| `transaction_type` | string? | sale / rent |
| `view_source` | enum | `search \| listing \| home \| favorites \| realtor \| internal \| external \| direct \| unknown` — derived from referrer / validated `?source=` |
| `filter_state` | object | active search filters (`q`, `transaction_type`, `property_type`, `price_min/max`, `area_min/max`, `rooms_min`, `city`, `district`, `sort_by`, `sort_order`) carried through from the originating search |
| `session_id` | string | anonymous per-tab id |
| `referrer` | string | referrer captured at emit time |

Note the two hardening rules the native port must preserve: `view_source` is
validated against a **bounded** enum (untrusted `?source=` collapses to
`unknown`) to cap dimension cardinality, and preview/non-real traffic is
excluded from the funnel.

## 3. Current-state matrix

| Funnel | reality-web | mobile-native Android | mobile-native iOS |
|--------|:-----------:|:---------------------:|:-----------------:|
| Listing view | ✅ `listing.viewed` | ❌ **[gap]** | ❌ **[gap]** |
| Search | ❌ **[gap]** | ❌ **[gap]** | ❌ **[gap]** |
| Contact-inquiry | ❌ **[gap]** | ❌ **[gap]** | ❌ **[gap]** |

"✅" = a canonical event is emitted on the wire today. All other cells are
uninstrumented. (The realtor-facing *ListingAnalyticsScreen*,
`mobile-native/androidApp/.../ui/realtor/ListingAnalyticsScreen.kt`, **displays**
analytics for a realtor's own listings — UC-51.10 — and is not client-side event
emission; it is out of scope for this audit.)

## 4. Per-funnel gaps + proposed native wire points

Because most of the native listing/search/inquiry logic already lives in the
shared KMP module (`commonMain`), a single emit point per funnel there covers
**both** Android and iOS. Prefer wiring in `commonMain`.

### 4.1 Listing view — `listing.viewed`  **[gap: Android + iOS]**

- **Reference:** `listing.viewed` (§2.1).
- **Native wire point:** `ListingDetailViewModel.loadListing()` on the
  `onSuccess` branch, in
  `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/listing/ListingDetailViewModel.kt`.
  Emit once per successful detail load (guard with the existing `started` flag so
  a retry after error does not double-count; there is no preview traffic on
  native, so the web preview-suppression rule is N/A).
- **Property mapping:** `listing_id`, `slug`, `transaction_type` come straight
  from the loaded `ListingDetail`. `view_source` should be passed into the
  view-model from the navigation origin (search results → `search`, related
  listing → `listing`, favorites → `favorites`, deep link → `external`,
  otherwise `direct`) — `DeepLinkRouter` /
  `mobile-native/shared/.../navigation/` already knows the entry channel.
  `filter_state` is available from the active `SearchState` when the user came
  from search. `session_id` / `referrer` require a native session-context helper
  (see §5).

### 4.2 Search — `search.performed`  **[gap: web + Android + iOS]**

No search event exists on **any** platform today (reality-web's
`src/app/[locale]/listings/page.tsx` emits nothing; the native search screens
`mobile-native/androidApp/.../ui/search/SearchScreen.kt` and the iOS `SearchView`
emit nothing). Proposed canonical event so web and native land on one schema:

- **Event:** `search.performed`
- **Native wire point:** the debounced-query → network-search path built by
  `SearchState.buildSearchRequest(...)` in
  `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/listing/SearchState.kt`
  (fire on the settled/dispatched search, i.e. the same generation that
  `debouncedQueryFlow` lets through — **not** on every keystroke).
- **Proposed properties:** `q` (string?), `filter_state` (same filter object as
  §2.1), `result_count` (int, from the returned page), `page` (int),
  `source` (`typed \| filter_change \| near_me \| initial`), `session_id`.

### 4.3 Contact-inquiry — `inquiry.submitted`  **[gap: web + Android + iOS]**

No inquiry event exists on any platform (reality-web `ContactForm.tsx` has no
tracking; native `ListingDetailViewModel.onSubmitInquiry` and the inquiry screens
emit nothing). Proposed canonical event:

- **Event:** `inquiry.submitted`
- **Native wire point:** `ListingDetailViewModel.onSubmitInquiry(...)` on the
  `onSuccess` branch (same file as §4.1), alongside the existing
  `ListingDetailEvent.InquirySubmitted` emission. A second call site is any
  standalone inquiry surface (`InquiryRepository.createInquiry`).
- **Proposed properties:** `listing_id`, `transaction_type`, `has_message`
  (bool), `contact_channel` (`in_app_form`), `authenticated` (bool),
  `session_id`. **Do not** log the message body, name, email, or phone — the
  portal's analytics operate under a no-PII contract (mirrors
  `analytics-session.ts`).

## 5. Recommended native implementation shape

Mirror the reality-web bus so producers and dashboards share one schema:

1. **`commonMain` event bus** — a pure-Kotlin `Analytics` object with
   `track(event: String, properties: Map<String, Any?>)`, a registerable
   `AnalyticsSink` fun-interface, and swallow-all error isolation (an event sink
   must never break the UI). No Ktor / Android / iOS types, so iOS reuses it
   verbatim — the same rationale that keeps `ListingDetailViewModel` and
   `SearchState` in `commonMain`. This is the direct analogue of
   reality-web `lib/analytics.ts`.
2. **`commonMain` session context** — an anonymous, per-launch `session_id`
   (mint-once, in-memory or a lightweight persisted store like the existing
   `SsoStateStore` actual/expect split) — the analogue of
   `analytics-session.ts`. No PII.
3. **Canonical event constants + derivation** — a `ListingAnalytics`-equivalent
   holding the event names (`listing.viewed`, `search.performed`,
   `inquiry.submitted`) and the bounded `view_source` enum, kept in sync with
   this document. Cover it with `commonTest` (host-JVM) unit tests, matching the
   existing `listingAnalytics.test.ts`.
4. **Platform sink wiring** — register the real transport (Firebase/GA4, or the
   app's chosen SDK) in `androidMain` / `iosMain`; keep `commonMain`
   transport-agnostic. Respect the consent state before forwarding (see §6).

Because there is currently no Koin container in the app (DI is constructor
injection — see the `ListingDetailViewModel` header note re #2079), the bus
should be reachable as a process-global object (like the web module-level
`trackEvent`) rather than injected, until DI lands.

## 6. Cross-cutting gaps

- **Consent gating.** reality-web has a `CookieConsentBanner`; native has no
  analytics consent surface. Any native analytics must gate emission on a
  consent decision to stay GDPR-aligned.
- **Session stitching.** Without a native `session_id`, native views/searches/
  inquiries can't be stitched into one journey the way web views are.
- **Web-side follow-ups.** §4.2 and §4.3 require *web* work too — this audit
  defines `search.performed` / `inquiry.submitted` as the shared target so the
  web and native implementations don't diverge.

## 7. Recommended follow-up actions

| # | Action | Suggested owner |
|---|--------|-----------------|
| 1 | Add the `commonMain` `Analytics` bus + session context + canonical event constants (§5) with `commonTest` coverage | pm-mobile |
| 2 | Wire `listing.viewed` at `ListingDetailViewModel.loadListing` success (§4.1) | pm-mobile |
| 3 | Wire `search.performed` at the `SearchState` dispatched-search path (§4.2) | pm-mobile |
| 4 | Wire `inquiry.submitted` at `ListingDetailViewModel.onSubmitInquiry` success (§4.3) | pm-mobile |
| 5 | Add native analytics-consent gating (§6) | pm-mobile |
| 6 | Instrument `search.performed` + `inquiry.submitted` on reality-web to the same schema (§4.2/§4.3) | pm-frontend |

## 8. Verification note (environment gap)

The code-level fixes (items 1–5) are **pm-mobile-owned** (`mobile-native/**`) and
could not be implemented-and-verified in this audit run: the mobile-native Gradle
build cannot configure in the sandbox because the Android Gradle Plugin
(`com.android.application:9.3.0`) is not resolvable through the environment proxy
(Google Maven is blocked), so `./gradlew` fails at root-project configuration
before any `:shared` task runs. This audit is therefore delivered as a
`docs/data/**` definitions document (verifiable without the mobile toolchain);
the wiring is specified precisely (§4–§5) so a pm-mobile run with a working
Android toolchain can implement and verify it directly.
