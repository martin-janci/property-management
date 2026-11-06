# PPT Project State

_Generated: 2026-11-06 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (73-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a OAuth Provider Foundation re-checked, 3/3 still done, no PRs touched OAuth this window). pm-scrum-master always-on._

## Delta since last routine run (2026-11-05)

- **21 PRs merged** (#3318–#3340) — the biggest tenant-isolation hardening wave to date.
  - **Round 5 authz A (#3331)** — API keys, package/visitor settings, attachments, rentals, news media, Booking.com push.
  - **Round 5 authz B (#3332, 12 items)** — accounting (invoice/contact/match by-id → 404 cross-tenant), buildings (`is_manager()` incl. technical_manager), building certifications (child credits/documents/milestones org-scoped), registry (non-manager pet/vehicle on own unit only), predictive maintenance (`add_maintenance_photo` manager-gated), equipment (`fault_id` + `performed_by` org-scoped on maintenance), facilities/person-months RLS connection released, subscriptions require_super_admin via DB platform principal, features.rs respects ResolvedTenant, onboarding roles from current-org membership, api_ecosystem auth_config/key_hash never serialized, data residency reads open/writes admin.
  - **Round 5 functional (#3330)** — migrations 00241 (energy/sustainability tables + policies, SUM floor_area → units.size_sqm, Decimal /0 guard), 00242 (portfolio_alert_rules/portfolio_alerts reconciliation), 00243 (automation templates slug column + 6-locale i18n); ppt-seed --force uses ACCESS EXCLUSIVE lock to toggle trg_audit_logs_no_update during cleanup; /layout/resolved/ppt/dashboard baseline layout published in seed; outages type mirroring; notification-settings/digest-prefs i18n gap-37 closure; person-months resident→403 + manager→200 assertions.
  - **Pet/vehicle tenant scoping (#3338)** — registration reads scoped to the caller's units.
  - **suspend_cascade (#3339)** — Enumerate + Display arms of SuspendCascadeError now covered.
  - **10 gh-issue-3304..3315 follow-ups** — post-merge-reviewer remediations: RLS-aware vote sub-resource reads (#3322), fault org isolation via SQL (#3327), IoT fold-insert (#3328), manager-gate automation stats (#3325), formatMoney Decimal precision + personLabel (#3319), honest /leases/{leaseId}/payments route (#3324), resolution-term responsiblePartyId mapping (#3323), member-role escalation via TenantRole::from_role_type (#3326), require_manager_rls consolidation (#3320), stable key order for reality-web messages/*.json (#3318), RecentActivity reporter-scope (#3321).
  - **Screen-map drift reconciliations** — #3334–#3337, #3340 reconciled 25+ screen-maps across ppt + reality after prior i18n and regression-test PRs.
- **3 'Still open' items from #3332 body** — accounting-server still uses RLS-only by-id methods; EnhancedWebhookSubscription.headers can carry auth headers; onboarding doesn't apply the ResolvedTenant rule. All three now have high-priority action-list rows and pm-qa test-first partners.
- **Carried blockers:** none net-new on CI; 2026-10-28's #3187 cross-tenant-assign-404 blocker was cleared in a prior window.

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-10a (idx 0, OAuth Provider Foundation — stories 10a-1..10a-3 still done; no PRs touched OAuth surfaces).
- **21 PRs merged** — the single biggest RLS/authz tightening wave in project history; Round-5 added regression suites for 11+ authz items, property_cluster_org_scope_tests, round5_sec_b_authz_tests, suite_7 person_months, and Decimal/money precision.
- **pm-qa rotation came up 73 days stale** — last run 2026-08-25. Six pm-qa next_actions queued (three high: accounting-by-id tests, webhook-headers smuggling tests, onboarding ResolvedTenant regression; three medium: audit_logs normal-op delete, Round-5 required-checks CI glue, mobile RN Alert.alert i18n regression).
- **One new code-review finding this run** — mobile RN Alert.alert hardcoded-English pattern (4 call sites, 73-day-old lint-gate risk re-raised).
- **New risks (3):** 'Still open' post-merge convention has no signal type; audit_logs ON DELETE SET NULL landmine; post-merge-review follow-up volume (11/21 PRs) indicates first-pass review gap.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level. Coverage holds 13 epics all at done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done · **last_checked 2026-11-06 (cursor re-check)** | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | #3326, #3327, #3328, #3322, #3318, #3320, #3319, #3324, #3323, #3325, #3321 (10× post-merge follow-ups), #3331/#3332 (Round-5 A/B authz) |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |
| 9 — 2FA/TOTP | done (1/1) | 1/1 done | — |
| 79/82/83/84/85 (extended) | — | all done | #3338 (pet/vehicle tenant scoping), #3330 (Round-5 functional incl. energy/alert_rules/seed/outages/automation i18n/person-months), #3339 (suspend_cascade cover) |

## What's next (top 5 from the roadmap)

1. **[high] Retire accounting-server RLS-only by-id methods** — move accounting reads onto the tenant-id-predicated variants to match #3332's shape · owner: pm-scrum-master (eventually dispatches to rust-backend) · dep: none · pairs with pm-qa's accounting-by-id regression tests.
2. **[high] Harden EnhancedWebhookSubscription.headers** — reject/strip Authorization/Cookie/X-API-Key etc. on write; omit from the read response schema · owner: pm-scrum-master · dep: none · pairs with pm-qa's webhook-headers smuggling test.
3. **[high] Apply ResolvedTenant rule to onboarding** — role selection consults ResolvedTenant only; 403 on foreign org header · owner: pm-scrum-master · dep: none · pairs with pm-qa's onboarding ResolvedTenant regression.
4. **[medium] Design-decide audit_logs ON DELETE SET NULL FK conflict** — ADR for the trigger/escape-hatch/CASCADE/NO ACTION decision · owner: pm-scrum-master → pm-tech-lead · dep: pm-tech-lead ADR.
5. **[medium] Automate Round-5 regression as required-checks on dev** — check-rls-enforcement.sh --strict + round5_sec_b_authz_tests + suite_7 property_cluster_org_scope · owner: pm-scrum-master → pm-devops.

## Blockers

- none this run.

## Role focus today

- **pm-scrum-master** (always on) — delivery synthesis + capturing the three 'Still open' items from #3332.
- **pm-qa** (rotation idx 3 of 8, 73 days stale) — test-shadow debt on #3332 'Still open', post-merge-review volume analysis, mobile RN Alert.alert i18n lint gate.

## Buffer

- action-list: 29/36 open (added 10, closed 6 this run — under the 36 target; not below half). Plan: pm-devops / pm-tech-lead rotations should keep trickling gap tasks in.
- risks: 29 total open (added 3 this run).
