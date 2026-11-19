# Project State — PPT (Property Management)

<sub>Generated: 2026-11-19T03:14:20Z — Scrum Master synthesis + pm-integration (rotation index 7 → 0). Coverage cursor advanced idx 5 → 6 (epic-80 re-checked, next epic-81).</sub>

## Executive summary

Sprint (Epic 6 / 7A / 8A / 10A) and all six tracked epics remain **done**; `coverage.json` holds 49/49 stories done (0 partial, 0 not-started). Delivery is entirely on the improvement track. 17 PRs (#3607–#3627) merged this window: reality-server listings/import fixes (property_type i18n, slug on search, import-history total), the lettre bump (#3619 closing #3618 and #3629), voice OAuth error-leak sanitization (#3621), ppt-web facilities isManager correction (#3620) and Toast/Announcer memoization (#3622), mobile Alert strings routed through i18n (#3626), screen-map drift syncs (#3624/#3625), and compliance pagination bounds (#3627). Four open PRs (#3612, #3617, #3628, #3630) — three touch the reality-server listings/import area and need sequencing. pm-integration (31 days since last run) flagged slug-client propagation and compliance `limit`-echo as the two most-exposed contract risks. Phase 1.5 rotating review of `reality-web` surfaced three new agency-branding findings (unhandled `useAgencyBranding` error path causing silent default-color clobber; hardcoded `Preview`/`Logo` alt text; hardcoded `€450,000` preview price). Carried digest-correctness items from the 2026-11-17 state remain the top backend priority (INNER-JOIN cohort gap, no `abandoned_at` terminal state, advisory-lock release leak).

## Sprint progress

- **Sprint:** Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth
- **Epics done:** 6 of 6 (100%)
- **Coverage (upkeep this run):** 49 of 49 stories done; rotating cursor re-checked **epic-80** (`last_checked` advanced for its stories; no status transitions).

## Shipped since last run (2026-11-18T03:15:25Z → 2026-11-19T03:14:20Z)

- #3627 — validate_pagination returns 400 for negative limit/offset in compliance routes (was opaque 500)
- #3626 — mobile RN Alert strings routed through i18n `t()` (WidgetSettings, DocumentDetail)
- #3625 — reconcile ppt screen-maps with ProtectedRoute wrap (follow-up to #3606)
- #3624 — reconcile reality auth screen-maps with locale-aware navigation (follow-up to #3595)
- #3623 — ppt-implement per-task PR-body scratchpad path (parallel-implementer clobber fix)
- #3622 — ppt-web Toast + Announcer provider values memoized
- #3621 — voice OAuth exchange/refresh error responses sanitized (no upstream leak)
- #3620 — ppt-web `FacilitiesPage` `isManager` derived from real role (was hardcoded true)
- #3619 — bump lettre to 0.11.24 (RUSTSEC-2026-0565); closes #3618 / #3629
- #3616 — reality-server `/listings` search `ListingSummary` carries SEO slug
- #3615 — i18n the trend-chart spike marker aria-label
- #3614 — reconcile faults-list screen-map with #3546 URL-param validation
- #3613 — reconcile reality login screen-map with #3559 open-redirect fix
- #3610 — agency import-history total is now a true COUNT (was capped page length)
- #3609 — reality-server property_type i18n labels data-driven
- #3608 — remediate cargo-deny RUSTSEC advisories (chrono/chrono-tz unmaintained, zip 3.0.0)
- #3607 — reality-web agency branding save now surfaces errors

## What's next (top 5)

1. **Sequence listings.rs PRs** — rebase #3617 (record_view existence check + per-IP throttle) and #3628 (suggestions region seed) on dev; merge #3617 first (security-flavored). *(pm-backend)*
2. **Decide #3612 (paginate agency import history)** — #3610 fixed total; confirm whether #3612 still adds page/offset or close as superseded. *(pm-backend)*
3. **Finish digest-correctness carry-over** — INNER JOIN excluding inactive-user cohort, add `abandoned_at` terminal state (migration + report_abandoned write), fix digest-sweep advisory-lock release leak. *(pm-backend)*
4. **Confirm slug propagation to generated clients** — #3616 added slug to reality-server ListingSummary; verify OpenAPI/TypeSpec regen and TS + KMP + iOS clients carry the field; document it as always-populated. *(pm-integration → pm-qa)*
5. **Land #3630 (mobile WidgetSettings i18n)** — mobile biome no-hardcoded-strings gate flips warn→error on 2026-12-01; burn down remaining mobile i18n backlog. *(pm-frontend)*

## Blockers

- **mobile-native/KMP cloud-build block (#2652)** — Gradle/KMP build cannot run in the cloud runner; 5+ KMP backlog items (create-listing stub, inquiries contract mismatch, no HttpTimeout) remain structurally unclaimable. *(pm-devops)*
- **Backend PR verification in cloud** — no Postgres/Docker in sandbox, so `#[sqlx::test]` suites cannot run locally; backend.yml CI is the only authoritative gate. *(pm-devops)*
- **Parallel suite_1 reliability** — `airbnb_install_routes_tests` leaks `AIRBNB_CLIENT_ID` env var without cleanup, flaking `airbnb_oauth_routes_tests::token_exchange_returns_503_when_not_configured` in parallel runs. *(pm-qa)*

## Decisions needed

- Close #3612 as superseded by #3610, or rescope it — **pm-backend**
- Make cargo-deny advisories a required or alerting check on `dev` (currently non-required) — **pm-devops**
- Role-gate `/financial` dashboard and `/financial/payments` to managers, or keep auth-only (#3625 follow-up) — **pm-security**
- Standard error envelope vs plain-text `(StatusCode, String)` for compliance validation errors — **pm-backend**
- Extend or hold the 2026-12-01 mobile i18n lint flip date — **pm-frontend**
- Whether `slug` becomes canonical public identifier for reality listings (UUID kept for back-compat) — **pm-product**
- RUSTSEC exception policy (max deadline, approver, CI gate surfacing) — **pm-security**

## Role focus today

- **Ran this run:** pm-scrum-master (always), pm-integration (rotation index 7 → 0 — next role: pm-tech-lead)
- **pm-integration summary:** Two contract themes active — (1) #3616 added `slug` to reality-server ListingSummary (additive but needs client-side regeneration verification + canonical-identifier decision); (2) compliance pagination fix (#3627) + voice OAuth sanitization (#3621) tightened error contracts but left the `limit` echo unclamped and the voice error body un-validated against RFC 6749 §5.2.
- **Carried backend focus:** digest subsystem (notification_digest.rs + granular_notification.rs are repeat churn hotspots); listings.rs is a new repeat hotspot (runs_seen=2 after this run) — ensure spec/contract follows contract-adjacent edits.
