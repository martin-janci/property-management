# Epic / Story / FR Delivery Status vs Codebase

> **Authoritative delivery-status reconstruction.** Built by auditing the documented BMAD
> catalog (`_bmad-output/epics.md`) against actual code under `backend/`, `frontend/`,
> `mobile-native/` at repo HEAD `a8a65b0fe` (branch `dev`). Evidence is file paths, route
> modules, migrations and PRs — **not** the existing status docs (those are reconciled in §6).
>
> Deliverable for **PAP-17** ("Initialize epics documentation into project") via **PAP-18**.
> Generated **2026-06-08** by CTO. Static read-only analysis; no feature code was written.

---

## 1. Executive Summary

**Overall: ~71% of documented epics are delivered end-to-end; 100% are delivered on the
backend.** The product is far *larger* than its catalog — the catalog describes 24 epics /
101 FRs, but the codebase ships **177 migrations, 71 mounted api-server route groups, and
46 ppt-web feature dirs.** The dominant risk is not missing backend; it is **(a) a handful
of MVP/Phase-2 features whose UI was never wired into the web router, and (b) a very large
body of undocumented "feature-factory" code** that no epic, story, or FR describes.

| Metric | Result |
|--------|--------|
| Epics fully delivered (backend + all relevant UI) | **17 / 24 (~71%)** |
| Epics partial (backend done, UI gap) | **7 / 24 (~29%)** |
| Epics fully missing | **0** |
| Backend coverage of documented epics | **24 / 24 (100%)** — every epic has migration + routes + substantive repo |
| FR coverage (API/schema level) | **101 / 101 reachable via API** — claim holds at the backend; 4 FR groups lack a web UI (see §5) |
| Undocumented backend route modules (code-no-doc) | **~40+** (marketplace, disputes, insurance, legal, esg, investor_portal, gov-portal, registry, forms, operations, …) |
| Undocumented ppt-web feature dirs | **~23 of 46** unmapped to any epic; ~16 are unreachable dead code |
| Genuine backend stubs (501 Not Implemented) | **3** — `vendor_portal.rs`, `public_api.rs`, `competitive.rs` |

### Biggest gaps (ranked)

1. **Epic 5 — Building Voting has NO manager-web UI.** Backend is the richest MVP module
   (`voting.rs` 1557 L, `vote.rs` repo 40 queries) and mobile has full screens, but there is
   **no `features/voting/` dir and no voting route in ppt-web**. A core MVP governance feature
   is unusable from the manager web app. *(Verified: `AppRoutes.tsx` has no voting group;
   `grep` finds zero `/voting` paths in ppt-web.)*
2. **Epic 12 (Meters) & Epic 19 (Leases) web UIs are built but unrouted.** Full feature dirs
   (`features/meters/` incl. OCR, `features/leases/`) exist but are **not mounted in
   `AppRoutes.tsx`** — dead from a user's perspective. Both work on mobile via raw fetch.
3. **Epic 14 (IoT & Smart Building) has zero UI** on any platform. Backend (`iot.rs`,
   `00048_create_iot_sensors`) is done; no dashboard was ever built.
4. **~16 fully-written ppt-web feature dirs are never mounted** (insurance, registry,
   government-portal, subscription, delegation, person-months, data-residency, multi-currency,
   portfolio-performance, competitive, api-ecosystem, developer, integrations, compliance,
   migration). Heavy code with no route and no epic.
5. **Generated API client lags the backend.** `@ppt/api-client` has no `voting`, `meters`, or
   `leases` module, forcing mobile to call raw endpoints and blocking clean web wiring.

### Top risks

- **Governance risk:** the flagship MVP voting workflow can't be administered from web.
- **Stale-doc risk:** `docs/FEATURE_COMPLETENESS_STATUS.md` is self-deprecated and
  `docs/DOCUMENTATION_DEEP_DIVE.md` claims "8 mounted routers" (reality: 71, ~9× stale).
  Trusting them would badly mis-state status. Screen-map frontmatter is the better signal.
- **Scope-sprawl / maintenance risk:** ~40 undocumented backend modules + ~16 dead-code web
  dirs represent un-catalogued, possibly unowned surface area (incl. an AML/EDD module
  `aml_dsa.rs` at 1963 L and `competitive.rs` with no schema at all).
- **Two unreconciled FR taxonomies:** `epics.md` = "101 FRs"; `docs/functional-requirements.md`
  = 51 categories / 256 sub-FRs / 508 UCs. Neither maps cleanly onto the other.

---

## 2. Per-Epic Status Rollup

Status = end-to-end (backend + the UI surfaces the epic implies). **BE**=backend, **W**=web,
**M**=mobile. ✅ done · 🟡 partial · ❌ missing · — not applicable.

### Phase 1 — MVP (12 epics)

| Epic | Name | FRs | BE | W | M | Rollup | One-line evidence |
|------|------|-----|----|----|----|--------|-------------------|
| 1 | User Auth & Sessions | FR1-3,5,6 | ✅ | ✅ | ✅ | **DONE** | `auth.rs` 2648 L, `user.rs` 28 q; ppt-web+admin+reality+mobile `auth/`; `/settings/sessions` (#966) |
| 2A | Organizations & Tenant Isolation | FR8-11,13,14 | ✅ | ✅ | — | **DONE** | `organizations/*`, RLS `00006`; admin-web `MembershipsPage`/`TenantLifecyclePage` |
| 2B | Notification Infra & Offline | FR26-28 | ✅ | 🟡 | ✅ | **DONE** | `ws_notifications.rs`, `push_tokens.rs`; mobile `usePushNotifications` + offline queue |
| 3 | Property & Building Mgmt | FR15-21 | ✅ | ✅ | ✅ | **DONE** | `buildings.rs` 2283 L; `features/buildings/`; mobile `screens/buildings` |
| 4 | Fault Reporting & Resolution | FR30-36 | ✅ | ✅ | ✅ | **DONE** | `faults.rs` 1508 L, `fault.rs` 34 q; `groups/faults.tsx` 454 L; mobile photo capture |
| **5** | **Building Voting & Decisions** | FR37-44 | ✅ | ❌ | ✅ | **🟡 PARTIAL** | **No ppt-web voting dir/route**; `voting.rs` 1557 L + mobile `screens/voting/` only |
| 6 | Announcements & Communication | FR22-25,29 | ✅ | ✅ | ✅ | **DONE** | `announcements/*`, `messaging.rs` 1222 L; `groups/announcements.tsx` 602 L |
| 7A | Basic Document Management | FR45-47,50 | ✅ | ✅ | ✅ | **DONE** | `documents/core.rs` 1663 L, `document.rs` 66 q; `groups/documents.tsx`; mobile 11 doc screens |
| 8A | Basic Notification Preferences | FR27 (basic) | ✅ | ✅ | — | **DONE** | `notification_preferences.rs`; ppt-web `/settings/*` |
| 9 | Privacy, Security & GDPR | FR4,90-95 | ✅ | ✅ | — | **DONE** | `gdpr.rs` 657 L, `tenant-ops` export/purge; `TwoFactorAuthPage`, `PrivacySettingsPage` + `gdprClient.ts` |
| 10A | OAuth Provider Foundation | FR7 (partial) | ✅ | ✅ | — | **DONE** | `oauth.rs` 1192 L, `oauth.rs` repo 34 q; admin-web `OAuthClientsPage`/`OAuthConsentPage` (#1171); ppt-web `oauth-grants` |
| 10B | Platform Administration | FR12,96-101 | ✅ | ✅ | — | **DONE** | `platform_admin/*`; admin-web Dashboard/PlatformHealth/feature-flags/SystemAnnouncements/Onboarding |

### Phase 2 — Financial & Enhanced (4 epics)

| Epic | Name | FRs | BE | W | M | Rollup | One-line evidence |
|------|------|-----|----|----|----|--------|-------------------|
| 11 | Financial Mgmt & Payments | FR52-57 | ✅ | ✅ | — | **DONE** | `financial.rs`, `budgets.rs` 1767 L; `groups/financial.tsx` (`/financial`,`/budgets`,`/invoices`,`/payments`) |
| 12 | Meter Readings & Utilities | FR58-63 | ✅ | 🟡 | ✅ | **🟡 PARTIAL** | `meters.rs` 1169 L; **ppt-web `features/meters/` built but NOT routed**; mobile `MeterReadingScreen` raw fetch |
| 7B | Advanced Document Features | FR48,49,51 | ✅ | 🟡 | — | **🟡 PARTIAL** | `signatures.rs` 1478 L, `documents/versions.rs`; web has `DocumentSearch` only, OCR/version UI thin |
| 8B | Granular Notification Prefs | FR27 (adv.) | ✅ | 🟡 | — | **🟡 PARTIAL** | `granular_notifications.rs` 880 L; `@ppt/api-client` modules present, dedicated web page thin |
| (2B-Complete) | WebSocket & Mobile Notif Infra | — | ✅ | — | ✅ | **DONE** | `ws_notifications.rs` + `push_fanout`; mobile push registration |

### Phase 3 — Modern Technology (2 epics)

| Epic | Name | FRs | BE | W | M | Rollup | One-line evidence |
|------|------|-----|----|----|----|--------|-------------------|
| 13 | AI Assistant & Automation | FR64-70 | ✅ | ✅ | — | **DONE** | `ai/*`, `automation.rs`; `features/ai-chat/` (`/ai-assistant`) + `workflow-automation/` (`/automations/*`) routed in `core.tsx` |
| 14 | IoT & Smart Building | FR71-75 | ✅ | ❌ | ❌ | **🟡 PARTIAL** | `iot.rs` 708 L, `00048_create_iot_sensors`; **no dashboard UI on any platform** |

### Phase 4 — Reality Portal & Rental (6 epics)

| Epic | Name | FRs | BE | W | M | Rollup | One-line evidence |
|------|------|-----|----|----|----|--------|-------------------|
| 10A-SSO | Cross-Platform SSO Consumer | FR7 (complete) | ✅ | ✅ | ✅ | **DONE** | reality-server `sso.rs` (callback + open-redirect guard); reality-web `auth/callback`; KMP `SsoService.kt` 348 L |
| 15 | Property Listings & Multi-Portal Sync | FR76-78 | ✅ | ✅ | ✅ | **DONE** | `listings.rs` 1298 L, `integrations/sync.rs` 2040 L; reality-web `listings/`, `realtor/listings/new`; KMP listing repo |
| 16 | Portal Search & Discovery | FR79-81 | ✅ | 🟡 | ✅ | **🟡 PARTIAL** | `saved_searches.rs`, `compare.rs`, `market_pricing.rs` 1833 L; **reality-web `price-map` page uses `MOCK_DISTRICTS`** |
| 17 | Agency & Realtor Management | FR82-83 | ✅ | ✅ | ✅ | **DONE** | `agencies.rs`, `agency_branding`; reality-web `agency/*`,`realtor/*`; KMP Android realtor/agency screens |
| 18 | Short-Term Rental Integration | FR84-86 | ✅ | ✅ | — | **DONE** | `rentals.rs` 1716 L, `integrations/booking_channel.rs`; `groups/rentals.tsx` 570 L (bookings/calendar/guests) |
| 19 | Lease Mgmt & Tenant Screening | FR87-89 | ✅ | 🟡 | ✅ | **🟡 PARTIAL** | `leases.rs` 1290 L, `enhanced_tenant_screening.rs`; **ppt-web `features/leases/` built but NOT routed**; mobile `LeaseSignatureScreen` |

> **Epic count discrepancy:** `epics.md` frontmatter declares `total_epics: 25` but the
> Epic List + Summary table enumerate **24** (12+4+2+6). The extra epics under
> `_bmad-output/epics/` — **epic-111 Multi-Language Support** and **epic-120 Docker
> Infrastructure** — are not in the 24-row catalog; one of them likely accounts for the "25".
> Both are effectively delivered (i18n: ppt-web/reality-web/mobile locale bundles incl.
> sk/cs/de/en + pl/hu; Docker: `docker-build.yml`/`docker-frontend.yml` + manifests). Treat
> the "24 vs 25" as a catalog bookkeeping nit, not a delivery gap.

---

## 3. Per-Story Status — MVP (Phase 1, all 74 stories)

Backend is **DONE for every MVP story** (each maps to a real migration + repo + route).
The `W`/`M` columns flag where the user-facing surface is missing. Unless flagged, the story
is delivered on the platforms its epic targets.

| Story | Status | Evidence / note |
|-------|--------|-----------------|
| 1.1 Registration + email verify | ✅ | `auth.rs`; ppt-web register; localized email templates |
| 1.2 Email/Password Login | ✅ | `auth.rs`; all 4 web/mobile apps |
| 1.3 JWT Token Refresh | ✅ | `00002_refresh_tokens`; refresh-rotation in `auth.rs` |
| 1.4 Password Reset Flow | ✅ | `00003_password_reset_tokens`; ppt-web forgot/reset |
| 1.5 Active Session Management | ✅ | ppt-web `/settings/sessions` (#966) |
| 1.6 User Lifecycle (Admin) | ✅ | admin-web users; `00130 invites` |
| 1.7 Localized Email Templates | ✅ | sk/cs/de/en templates |
| 2A.1 Organization Creation | ✅ | `organizations/*`, `00004` |
| 2A.2 RLS Enforcement | ✅ | `00006_enable_rls_policies` + per-table RLS migrations |
| 2A.3 RLS Penetration Test Framework | ✅ | RLS test scaffolding in `db`/tests |
| 2A.4 Org Settings & Branding | ✅ | `organizations/settings`; `00142 org_auth_policies` |
| 2A.5 Org Member Management | ✅ | admin-web `MembershipsPage`; `00128 memberships` |
| 2A.6 RBAC | ✅ | role/membership repos; `00138 capability_grants` |
| 2A.7 Org Data Export | ✅ | `tenant-ops` export; `00026 data_export_requests` |
| 2B.1 Event Bus Foundation | ✅ | Redis pub/sub; notification pipeline service |
| 2B.2 Push Service (FCM/APNs) | ✅ | `push_tokens.rs`, `00157 device_push_tokens`; mobile registration |
| 2B.3 Email Notification Service | ✅ | email service in `common`/integrations |
| 2B.4 In-App Notification Center | 🟡 W | backend + critical-notifications UI present; full center thin on web |
| 2B.5 Privacy-Aware Notification Design | ✅ | `00018 privacy_settings` respected in fanout |
| 2B.6 Offline Sync Queue Foundation | ✅ | mobile `useOfflineSupport` AsyncStorage queue |
| 2B.7 Idempotency Pattern | ✅ | idempotency middleware in `api-core` |
| 3.1 Building Creation | ✅ | `buildings.rs`; `features/buildings/` |
| 3.2 Unit Definition | ✅ | `00008 units`, `unit.rs` 26 q |
| 3.3 Resident Association | ✅ | `unit_residents.rs`, `00009` |
| 3.4 Ownership Delegation | ✅ | `delegations.rs`, `00010` (note: ppt-web `delegation/` dir is **unrouted**) |
| 3.5 Person-Month Tracking | ✅ BE / 🟡 W | `person_months.rs`, `00011`; ppt-web `person-months/` dir **unrouted** (mobile screen exists) |
| 3.6 Unit Detail View (Residents) | ✅ | `/buildings/:id` |
| 3.7 Common Areas & Facilities | ✅ | `facilities.rs`, `00012`; `/facilities/bookings/my` |
| 4.1 Fault Creation w/ Photos | ✅ | `groups/faults.tsx`; mobile `ReportFaultScreen` (camera, max 5) |
| 4.2 AI Category Suggestion | ✅ | `ai_category`/`confidence` columns; AI suggestion badge in web |
| 4.3 Fault Triage by Manager | ✅ | `useTriageFault` |
| 4.4 Fault Status Workflow | ✅ | lifecycle in `fault.rs` 34 q |
| 4.5 Fault Status Tracking (Residents) | ✅ | resident timeline |
| 4.6 Fault Resolution & Rating | ✅ | `useResolveFault`/`useConfirmFault` |
| 4.7 Fault Reports & Analytics | ✅ | `reports.rs` fault analytics |
| **5.1 Vote Creation w/ Question Types** | 🟡 **W❌** | backend `voting.rs`; **no ppt-web UI**; mobile create only |
| **5.2 Quorum Configuration** | 🟡 **W❌** | backend only on web |
| **5.3 Vote Casting (Online Only)** | 🟡 **W❌ / M✅** | mobile `VoteDetailScreen`; no web |
| **5.4 Delegated Voting** | 🟡 **W❌** | `00010 delegations` + backend; no web vote UI |
| **5.5 Vote Discussion Threads** | 🟡 **W❌** | backend only on web |
| **5.6 Vote Results & Quorum Calc** | 🟡 **W❌ / M✅** | mobile results; no web |
| **5.7 Immutable Voting Audit Trail** | ✅ BE | immutable audit log present (backend) |
| **5.8 Voting Compliance Reports** | 🟡 **W❌** | `reports.rs` backend; no web surface |
| 6.1 Announcement Creation & Targeting | ✅ | `groups/announcements.tsx`; `00158/00176 targeting` |
| 6.2 Announcement Viewing & Ack | ✅ | web + mobile |
| 6.3 Announcement Comments | ✅ | `00016 announcement_comments` |
| 6.4 Pinned Announcements | ✅ | `00158 pinned` |
| 6.5 Direct Messaging | ✅ | `messaging.rs` 1222 L; `groups/messaging.tsx` |
| 6.6 Neighbor Information (Privacy-Aware) | ✅ | `neighbors.rs`; `groups/neighbors.tsx` routed |
| 7A.1 Document Upload + Metadata | ✅ | `documents/core.rs`; web + mobile upload |
| 7A.2 Folder Organization | ✅ | `documents/folders.rs` |
| 7A.3 Permission-Based Access | ✅ | `00172 documents RLS` |
| 7A.4 Download & Preview | ✅ | web PDF preview; mobile preview |
| 7A.5 Document Sharing | ✅ | `documents/shares.rs` (2 RLS-context TODOs flagged §4.C) |
| 8A.1 Channel-Level Toggles | ✅ | `notification_preferences.rs`; `/settings/*` |
| 8A.2 Critical Notification Override | ✅ | `critical_notifications.rs`, `00022` |
| 8A.3 Preference Sync | ✅ | `00023/00177` trigger/RLS fixes |
| 9.1 TOTP 2FA Setup | ✅ | `mfa.rs` 1239 L; `TwoFactorAuthPage` `/settings/two-factor` |
| 9.2 2FA Recovery & Mgmt | ✅ | recovery codes in `mfa.rs` |
| 9.3 GDPR Data Export | ✅ | `gdpr.rs` + `00026`; `/settings/privacy` export |
| 9.4 GDPR Data Deletion | ✅ | `00027 scheduled_deletion`; deletion request UI |
| 9.5 Privacy Settings Config | ✅ | `PrivacySettingsPage` + `gdprClient.ts` |
| 9.6 Compliance Audit Logs | ✅ | `00025 audit_logs`, `00155 immutable`; admin `audit.rs` |
| 9.7 Compliance Reports Generation | ✅ | `reports.rs` |
| 10A.1 OAuth 2.0 Authorization Server | ✅ | `oauth.rs` 1192 L (authorize/token/consent, PKCE) |
| 10A.2 OAuth Client Registration | ✅ | admin-web `OAuthClientsPage` |
| 10A.3 OAuth Token Management | ✅ | `00150/00156/00173 oauth audit` |
| 10B.1 Org Management Dashboard | ✅ | admin-web Dashboard |
| 10B.2 Feature Flag Management | ✅ | `00030 feature_flags`; admin feature-flags page |
| 10B.3 Platform Health Monitoring | ✅ | `00031`; admin PlatformHealth |
| 10B.4 System Announcements | ✅ | `00032`; admin SystemAnnouncements |
| 10B.5 Support Data Access | ✅ | admin support-data page; `00163/00165 support_tooling` |
| 10B.6 User Onboarding Tour | ✅ | `00033 user_onboarding`; admin OnboardingTours + ppt-web `onboarding/` |
| 10B.7 Contextual Help & Documentation | ✅ | `00034 contextual_help`; `help.rs`, ppt-web `features/help` |

**MVP story tally:** 74 stories — **66 fully delivered**, **8 partial** (the entire Epic 5
voting set on web: 5.1–5.6, 5.8; plus 2B.4 / 3.5 web surface). **0 missing on backend.**

---

## 4. Gap Analysis

### 4.A — Documented stories/FRs with NO (or unreachable) implementation

| Gap | Where | Detail |
|-----|-------|--------|
| Epic 5 voting — **web UI** | ppt-web | No `features/voting/`, no route. FR37-44 unreachable from manager web. |
| Epic 12 meters — **web route** | ppt-web | `features/meters/` (incl. OCR) exists but **not in `AppRoutes.tsx`** → unreachable. |
| Epic 19 leases — **web route** | ppt-web | `features/leases/` exists but **not routed** → unreachable. |
| Epic 14 IoT — **all UI** | — | No dashboard on web or mobile; backend-only. |
| Epic 16 price-map — **real data** | reality-web | `price-map/page.tsx` renders `MOCK_DISTRICTS`/`MOCK_INSIGHTS`, not reality-server. |
| Epic 7B/8B advanced UI | ppt-web | OCR/version-history & granular-prefs pages thin/absent though backend done. |
| Story 3.5 person-months — web | ppt-web | `person-months/` dir unrouted (mobile screen exists). |
| `@ppt/api-client` gaps | shared | No `voting`/`meters`/`leases` client module → blocks clean web wiring; mobile uses raw fetch. |

> **No documented FR is entirely un-built on the backend.** Every gap above is a *UI/wiring*
> gap on top of a working API + schema.

### 4.B — Implemented features with NO epic/story/FR doc ("code with no doc")

**Backend (~40+ route modules + ~50 migrations) with no catalog entry**, grouped:

- **Facilities/Ops/Maintenance:** `work_orders.rs` (935 L), `vendors.rs` (989 L),
  `operations.rs` (1443 L), `package_visitor.rs` (1175 L), `outages.rs` (932 L).
- **Insurance/Legal/Compliance/Disputes:** `insurance.rs`, `legal.rs` (1134 L),
  `compliance.rs`, `regional_compliance.rs`, `data_residency.rs`, `disputes.rs` (1581 L),
  `violations.rs`, **`aml_dsa.rs` (1963 L — AML/EDD, large & unowned)**, `emergency.rs`
  (1634 L), `government_portal.rs` (987 L).
- **Financial/Investor/Portfolio (beyond Epic 11):** `subscriptions.rs` (1681 L),
  `investor_portal.rs`, `owner_analytics.rs`, `portfolio_analytics.rs`,
  `portfolio_performance.rs`, `property_valuation.rs`, `market_pricing.rs` (1833 L),
  `board_meetings.rs`.
- **ESG/Certifications:** `esg_reporting.rs`, `building_certifications.rs`.
- **Marketplace/News/Community/Registry/Forms:** `marketplace.rs` (1933 L),
  `news_articles.rs` (1060 L), `community.rs`, `registry.rs` (1093 L), `forms.rs` (1873 L).
- **Platform/Infra (beyond Epic 10B):** `infrastructure.rs` (2122 L), `feature_packages.rs`,
  `reports.rs` (2271 L), `migration.rs` (1421 L), `tenant_config.rs`, `api_ecosystem.rs`
  (1913 L), `multi_currency.rs`.

**Frontend ppt-web feature dirs with no epic** (≈23 of 46):

- **Routed-but-undocumented** (reachable, just no epic): `disputes/` (482 L group),
  `outages/` (373 L group), `neighbors/`, `community/` (`/community/marketplace|events|groups`),
  `news/`, `emergency/`, `facilities/`, `command-palette/` (global widget).
- **Dead code (no route, no epic, ~16 dirs):** `insurance/`, `registry/`,
  `government-portal/`, `subscription/`, `delegation/`, `person-months/`, `data-residency/`,
  `multi-currency/`, `packages/`, `portfolio-performance/`, `competitive/`, `api-ecosystem/`,
  `developer/`, `integrations/`, `compliance/`, `migration/`. Several import the generated
  client heavily (e.g. `integrations` 12 imports) yet are never mounted.

**Mobile undocumented screens:** RN `forms/`, `person-months/`, `outages/`, `neighbors/`,
`news/`; plus RN scaffolds **missing native deps** (`nfc/`, `qrcode/`, `voice/`) and screens
built-but-unwired in `App.tsx` (`WidgetSettingsScreen`, Help/Feedback/onboarding).

### 4.C — Stale / contradictory docs and stubs

**Genuine backend stubs (every handler returns 501):**

- `routes/public_api.rs` — 12 handlers `not_implemented` (external-developer API facade).
- `routes/vendor_portal.rs` — 15 handlers `not_implemented` (vendor-facing portal; the
  separate `vendors.rs` repo IS real).
- `routes/competitive.rs` — 20 handlers `not_implemented`, **no migration backs it** — pure
  scaffold (matches the dead `competitive/` web dir).

**Minor backend TODOs (not stubs):** `documents/core.rs:625`, `documents/shares.rs:415,546`
(public-share paths run without RLS `TenantContext` — a security surface worth closing);
`budgets.rs:1506` (`TODO(epic-24)` — references an epic beyond the catalog).

**Stale status docs** (full reconciliation in §6): `FEATURE_COMPLETENESS_STATUS.md`
self-deprecated; `DOCUMENTATION_DEEP_DIVE.md` claims "8 mounted routers" (reality 71).

---

## 5. FR Coverage Check — validating the "101/101 FRs" claim

**Verdict: the "101/101 FRs covered" claim holds at the API/schema level, but 4 FR groups
are not reachable from the web UI.** Every FR maps to an epic whose backend is DONE, so each
FR is exercised by a real route + migration. The qualifier the frontmatter omits is *surface*.

| FR group | FRs | Backend | UI reachability | Note |
|----------|-----|---------|-----------------|------|
| Identity & Access (CA-01) | FR1-7 | ✅ | ✅ | FR7 SSO complete (Epic 10A + 10A-SSO) |
| Org & Multi-Tenancy (CA-02) | FR8-14 | ✅ | ✅ | RLS enforced |
| Property & Resident (CA-03) | FR15-21 | ✅ | ✅ | |
| Communication (CA-04) | FR22-29 | ✅ | ✅ | |
| **Issue & Fault (CA-05)** | FR30-36 | ✅ | ✅ | |
| **Voting (CA-06)** | **FR37-44** | ✅ | **❌ web / ✅ mobile** | **Web UI missing — biggest FR-coverage caveat** |
| Document (CA-07) | FR45-51 | ✅ | 🟡 | FR48/49/51 (OCR/search/versions) UI thin |
| Financial (CA-08) | FR52-57 | ✅ | ✅ | |
| **Meter/Utilities (CA-09)** | **FR58-63** | ✅ | **🟡 web unrouted / ✅ mobile** | Web built but not mounted |
| AI & Automation (CA-10) | FR64-70 | ✅ | ✅ | |
| **IoT & Smart Building (CA-11)** | **FR71-75** | ✅ | **❌ no UI** | Backend-only |
| Real Estate & Listings (CA-12) | FR76-81 | ✅ | 🟡 | FR79-81 search OK; price-map mocked |
| **Rental Mgmt (CA-13)** | FR82-86 | ✅ | ✅ | Agency + STR done; **FR87-89 leases web unrouted** |
| Lease/Screening (CA-13 cont.) | FR87-89 | ✅ | 🟡 | Web unrouted / mobile done |
| Compliance & Privacy (CA-14) | FR90-95 | ✅ | ✅ | |
| Platform Operations (CA-15) | FR96-101 | ✅ | ✅ | |

**FR coverage: 101/101 on backend; ~88/101 fully reachable in a shipped UI.** The
~13 with a UI gap are FR37-44 (voting web), FR48/49/51 (advanced docs), FR58-63 (meters web),
FR71-75 (IoT), FR87-89 (leases web). Note also the **two unreconciled FR taxonomies** (§6):
`epics.md` 101 FRs ≠ `docs/functional-requirements.md` 51 categories / 256 sub-FRs / 508 UCs.

---

## 6. Reconciliation — existing status docs & duplicate BMAD copies

| Doc | Claim | Verdict |
|-----|-------|---------|
| `docs/FEATURE_COMPLETENESS_STATUS.md` | 10 UCs driven to "100%" across 3 waves | **STALE / self-deprecated.** Banner (lines 3-18) marks it DEPRECATED (last refresh 2026-01-06), flagged internally contradictory by the 2026-05-23 review (header 100% vs body ❌). **Do not cite.** Points to screen-map frontmatter as truth. |
| `docs/DOCUMENTATION_DEEP_DIVE.md` | "api-server mounts only 8 routers" (`/auth,/organizations,/buildings,/faults,/voting,/rentals,/listings,/integrations`) | **STALE ~9×.** Reality: **71 mounted `/api/v1/*` groups.** Also internally contradicts itself on UC counts (479/493/407 vs the later 508 resolution). FaultStatus/TypeSpec-as-source guidance still valid. |
| `docs/functional-requirements.md` | "decomposes all 508 use cases" | **Structurally complete, plausibly current** (197 KB). But it is **FR-XX.Y / 508-UC taxonomy** (51 categories, 256 sub-FRs) — **NOT** the "101 FRs" of `epics.md`. The two numbering schemes are unreconciled. |
| `docs/use-cases.md` | 508 UC catalog | Consistent with `docs/CLAUDE.md` and the FR doc's 508 figure. |

**Duplicate `_bmad-output/` copies:** Five copies of `epics.md` exist (root + 4 git
worktrees under `.worktrees/`: code-reviews, dispatchers, issues-fixes, walk-rewiew). All
**byte-identical** (`md5 d3f161cab92f5ceea282bd4388989322`). These are `git worktree`
checkouts, not stray copies. **No divergence — no reconciliation needed.**

---

## 7. Recommended follow-up issues (for CEO/Atlas sign-off — NOT yet created)

Build gaps surfaced by this analysis, ranked. *Per PAP-18, listed only — not created.*

1. **[MVP-blocker] Build ppt-web Voting UI (Epic 5 / FR37-44).** Manager web cannot create or
   administer votes. Needs `features/voting/` + route group + `@ppt/api-client` voting module.
2. **[Quick win] Wire `features/meters/` and `features/leases/` into `AppRoutes.tsx`
   (Epic 12 / 19).** UIs already exist; mostly add route groups + client modules.
3. **[Phase 3 gap] Build IoT/Smart-Building dashboard UI (Epic 14 / FR71-75).** Backend done,
   no front door.
4. **[Polish] Replace reality-web `price-map` mock with reality-server data (Epic 16).**
5. **[Hygiene] Triage the ~16 dead-code ppt-web feature dirs and 3 backend 501-stubs**
   (`vendor_portal`, `public_api`, `competitive`) — wire, document, or delete.
6. **[Security] Close the document public-share RLS-context TODOs** (`documents/shares.rs:415,546`).
7. **[Docs hygiene] Retire `FEATURE_COMPLETENESS_STATUS.md`, fix the "8 routers" line in
   `DOCUMENTATION_DEEP_DIVE.md`, and reconcile the two FR taxonomies** (101-FR vs 256-sub-FR).
8. **[Catalog] Decide policy for the ~40 undocumented backend modules** — backfill epics/FRs
   for the keepers (disputes, marketplace, forms, gov-portal, ESG…) or formally mark out-of-scope.

---

*Evidence basis: 3 parallel stack audits (backend / frontend / mobile+docs) cross-checked by
direct `grep`/`find`/`read` on HEAD `a8a65b0fe`. Verification spot-checks recorded inline
(e.g. `AppRoutes.tsx` group wiring, absence of ppt-web voting dir, `MOCK_DISTRICTS` in
price-map). No code was modified.*

---

## 8. Un-catalogued surface triage (PAP-24)

> Resolves [PAP-18](/PAP/issues/PAP-18) §7 items **#5** (dead-code dirs + 501 stubs) and **#8**
> (undocumented backend modules). A wire / document / delete decision is recorded for **every**
> item. Verification was re-run against HEAD `dev` directly (not trusting §4) — and it surfaced
> **4 more unmounted ppt-web dirs** than §4.B listed (`marketplace`, `forms`, `onboarding`,
> `critical-notifications`) and **corrected the framing**: most "dead" dirs are *built UIs with a
> live mounted backend* (an unwired-feature/product call), **not** dead code. Only `competitive`
> is genuinely dead (501 stub + no migration + no product backing).

### 8.0 Stub & uncatalogued-surface policy (NEW — enforced going forward)

A backend `501 not_implemented` handler, or an unmounted frontend feature dir, is **permitted only if**:

1. it is tracked by an **open epic or issue**, AND
2. it carries a top-of-module marker `// ROADMAP(PAP-NN): <one-line reason>` (TS: `// ROADMAP(PAP-NN)`).

Anything uncatalogued **and** unannotated is dead code and is **deleted on sight**. New stubs
added without a tracking issue **fail code review**. Mounted endpoints that only return 501 should
be **unmounted** (so external callers get `404`, not a false "exists but broken" `501`) until real.

### 8.1 Backend 501-stubs (3) — verified mounted, all handlers `not_implemented`

| Module | Mounted at | 501 handlers | Migration? | Decision | Rationale |
|--------|-----------|--------------|-----------|----------|-----------|
| `routes/competitive.rs` | `/api/v1/competitive` | 20 | **none** | **DELETE** | Pure scaffold: no schema, no product doc, mirrors dead `competitive/` web dir. Remove module + `mod` decl + nest + web dir. |
| `routes/public_api.rs` | `/api/v1/developer` | 12 | n/a (facade) | **DOCUMENT as roadmap** | External-developer API facade — a deliberate public contract; keep but add `ROADMAP` marker + tracking issue, and **unmount** until implemented. Drives the `developer/` web dir decision (8.2). |
| `routes/vendor_portal.rs` | `/api/v1/vendor-portal` | 16 | n/a (facade) | **DOCUMENT as roadmap** | Vendor-facing portal facade. The implemented vendor surface is the separate `vendors.rs` (989 L, real). Keep stub + `ROADMAP` marker + tracking issue; unmount until implemented. |

### 8.2 ppt-web feature dirs — 46 total: 24 reachable, 22 unmounted (triaged below)

Reachable (24, no action): `ai-chat announcements auth buildings community dashboard disputes
documents emergency errors facilities faults financial messaging neighbors news oauth-grants
outages privacy rentals reports settings workflow-automation` + `command-palette` (global widget
mounted in `App.tsx`).

| Unmounted dir | LOC | Backend status | Decision | Owner / note |
|---------------|----:|----------------|----------|--------------|
| `competitive/` | 2766 | 501 stub, no migration | **DELETE** | Bundled with `competitive.rs` deletion (8.1). |
| `developer/` | 5953 | backend is `public_api` 501 stub | **DELETE (or hold)** | Non-functional until `public_api` is built; delete unless roadmapped with the stub. |
| `meters/` | 4368 | `/api/v1/meters` live | **WIRE** | Owned by [PAP-20](/PAP/issues/PAP-20). |
| `leases/` | 5362 | `/api/v1/leases` live | **WIRE** | Owned by [PAP-20](/PAP/issues/PAP-20). |
| `insurance/` | 3824 | `/api/v1/insurance` live | **WIRE (deferred)** | Built UI, live backend — product-roadmap call. |
| `marketplace/` | 5674 | `/api/v1/marketplace` live | **WIRE (deferred)** | **Newly found.** Separate from mounted `community/MarketplacePage`. |
| `forms/` | 2444 | `/api/v1/forms` live | **WIRE (deferred)** | **Newly found.** |
| `onboarding/` | 3406 | n/a (client flow) | **WIRE / INVESTIGATE** | **Newly found** — core flow unmounted; likely a regression, treat as keeper. |
| `critical-notifications/` | 932 | live | **WIRE (deferred)** | **Newly found.** |
| `migration/` | 4850 | `migration.rs` live | **WIRE (deferred)** | Admin/data-migration tool. |
| `subscription/` | 3189 | `/api/v1/subscriptions` live | **WIRE (deferred)** | |
| `government-portal/` | 2800 | `government_portal` live | **WIRE (deferred)** | |
| `integrations/` | 2864 | `/api/v1/integrations` live | **WIRE (deferred)** | Heavy client usage; backend live. |
| `compliance/` | 2403 | `compliance.rs` live | **WIRE (deferred)** | |
| `registry/` | 2398 | `registry.rs` live | **WIRE (deferred)** | |
| `multi-currency/` | 2628 | `multi_currency.rs` live | **WIRE (deferred)** | |
| `portfolio-performance/` | 2275 | `portfolio_performance.rs` live | **WIRE (deferred)** | |
| `api-ecosystem/` | 1813 | `api_ecosystem.rs` live | **WIRE (deferred)** | Platform feature. |
| `delegation/` | 1748 | `/api/v1/delegations` live | **WIRE (deferred)** | |
| `person-months/` | 1592 | live | **WIRE (deferred)** | Story 3.5; mobile screen exists. |
| `data-residency/` | 1301 | `data_residency.rs` live | **WIRE (deferred)** | Compliance niche. |
| `packages/` | 941 | `package_visitor.rs` live | **WIRE (deferred)** | |

**Net:** 2 DELETE (`competitive`, `developer`), 2 already-owned WIRE (PAP-20), 18 WIRE-deferred
(built UI + live backend → roadmap decision, see 8.4). **Zero are "dead code with no backing"
except `competitive`/`developer`.**

### 8.3 ~40 undocumented backend modules — all verified **mounted & live** → DOCUMENT (none deleted)

Decision for the whole class: **DOCUMENT** (backfill an epic/FR catalog stub). These are
production routes (substantial, 700–2300 L each) carrying real schema + migrations; deletion is
off the table. Grouped per §4.B — each group becomes a catalog backfill unit:

- **Facilities/Ops:** `work_orders` `vendors` `operations` `package_visitor` `outages`
- **Insurance/Legal/Compliance/Disputes:** `insurance` `legal` `compliance` `regional_compliance` `data_residency` `disputes` `violations` `aml_dsa` `emergency` `government_portal`
- **Financial/Investor/Portfolio:** `subscriptions` `investor_portal` `owner_analytics` `portfolio_analytics` `portfolio_performance` `property_valuation` `market_pricing` `board_meetings`
- **ESG/Certifications:** `esg_reporting` `building_certifications`
- **Marketplace/News/Community/Registry/Forms:** `marketplace` `news_articles` `community` `registry` `forms`
- **Platform/Infra:** `infrastructure` `feature_packages` `reports` `migration` `tenant_config` `api_ecosystem` `multi_currency`

The largest/highest-risk unowned module is **`aml_dsa.rs` (1963 L, AML/EDD)** — prioritize its
catalog entry (regulatory surface).

### 8.4 Disposition & follow-ups

- **DELETE (dead):** `competitive` (backend+web) and `developer` web dir → child issue, build-verified.
- **DOCUMENT (roadmap stubs):** `public_api`, `vendor_portal` → add `ROADMAP` markers + tracking,
  unmount from live router. Folded into the deletion child (same backend touch).
- **DOCUMENT (catalog backfill):** ~40 mounted backend modules → child issue.
- **WIRE-deferred (18 built UIs w/ live backend):** **product-roadmap decision** — not unilaterally
  deletable (discards working features) nor auto-wireable (each needs a route group + `@ppt/api-client`
  module + UX sign-off). Escalated to Atlas/UX for a keep-and-schedule vs retire call; tracked, not
  executed here.
- **Policy:** §8.0 stub/uncatalogued-surface policy adopted as the standing review rule.
