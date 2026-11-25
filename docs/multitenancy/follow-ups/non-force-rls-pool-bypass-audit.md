# Audit & Backlog — RLS tables not `FORCE`'d + raw-pool repository bypass

**Date:** 2026-11-25
**Owner:** `pm-security`
**Status:** audit complete — backlog proposed, remediation not yet scheduled
**Task:** `pm-security-audit-non-force-rls-pool-bypass-2026-11-25`
**Scope:** repo-wide, static analysis of `backend/crates/db` (migrations + repositories). No
schema or code change in this PR — this is a ranked audit plus a batched remediation backlog.

---

## 1. Why this matters (the threat model, confirmed in-tree)

PostgreSQL Row-Level Security has **two** switches per table:

- `ENABLE ROW LEVEL SECURITY` — makes policies apply to *non-owner, non-`BYPASSRLS`* roles.
- `FORCE ROW LEVEL SECURITY` — makes policies apply **to the table owner too**.

The production **api-server connects as the table owner** (`ppt`). This is not a hypothesis —
it is stated verbatim in the migrations that already fixed this class of bug:

> "`ENABLE` (without `FORCE`) is bypassed for the table owner and for superuser/`BYPASSRLS`
> roles. The production api-server connects as the table OWNER, so RLS was bypassed entirely
> → every tenant context saw EVERY org's rows: a cross-tenant IDOR / data-leak path."
> — `00179_fix_epic11_org_guc_owner_bypass.sql` (PAP-62); same pattern as `00172` (documents,
> #754), `00171` (messaging, #898), `00177` (Epic 8A notifications, #755).

So for a tenant-scoped table, **`ENABLE` without `FORCE` = full RLS bypass for every query the
app runs on a plain pool/owner connection** = silent cross-tenant data leak. This is strictly
worse than the `FORCE`-but-no-context failure mode, which collapses to *deny-all* (a loud,
non-leaking breakage) rather than a leak.

Two conditions must both hold for an **active** leak:

1. the table is tenant-scoped, `ENABLE`'d, **not** `FORCE`'d; and
2. some code path queries it on a **raw pool / owner** connection (no `set_request_context`).

This audit enumerates (1), enumerates (2), and ranks their intersection by data sensitivity.

### The CI blind spot that let this persist

The repo already ships a good detector — `backend/scripts/check-rls-enforcement.sh` (PAP-68
pool-field detector). But by construction it **only flags repositories whose SQL touches a table
in the `FORCE`-RLS set** (it intersects `touched ∩ force_final`). Tables that were never `FORCE`'d
are invisible to it, and so are the repositories that read them on a raw pool. That is exactly the
cohort this audit surfaces: **29 repositories touch non-`FORCE` RLS tables on a raw pool field and
appear in no existing baseline or allowlist** (see §4).

---

## 2. Method (reproducible)

Derived purely from `backend/crates/db`:

- **Non-`FORCE` tables:** parse every `ALTER TABLE … ENABLE ROW LEVEL SECURITY` and every
  `… FORCE ROW LEVEL SECURITY` across `migrations/*.sql`; a table is "enabled but not forced"
  if it appears in the first set and never in the second. (Each FORCE in-tree is an explicit
  per-table `ALTER`; there is no dynamic `DO`/loop FORCE that this would miss — verified.)
- **Policy presence / GUC:** parse `CREATE POLICY … ON <table>` and the GUC names each policy
  references, to separate "has a policy but owner bypasses it" from "no policy at all".
- **Raw-pool repositories:** a repository unit (single `*.rs` file, or a split-repo `*/` module
  dir analysed as one unit) that (a) holds a `pool`/`db: PgPool|DbPool` field, (b) uses
  `&self.pool`/`&self.db`, (c) never calls `set_request_context`, and (d) whose SQL
  (`FROM|JOIN|INTO|UPDATE <table>`) touches a non-`FORCE` table. This mirrors the four criteria
  the existing detector uses — only the table set is widened from `FORCE` to `ENABLE-not-FORCE`.

### Headline counts

| Metric | Count |
|---|---:|
| Tables `ENABLE`'d (distinct) | 463 |
| Tables `FORCE`'d (distinct) | 291 |
| **Tables `ENABLE`'d but never `FORCE`'d** | **172** |
| … of those, with **no `CREATE POLICY` at all** | 17 |
| … of those, touched by at least one raw-pool repository | 149 |
| Raw-pool repositories touching a non-`FORCE` table | 42 |
| … **not** tracked by any existing RLS baseline/allowlist (new blind spot) | **29** |

Sensitivity tiers below: **S** = credentials / auth / financial / PII-screening / lease
contracts / audit trails; **A** = core tenant business data (buildings, units, residents, faults,
votes, violations, meters, packages, investor, government-portal); **B** = analytics / derived /
reference / config.

---

## 3. Finding 1 — 172 RLS tables not `FORCE`'d, ranked by sensitivity

Columns: table · migration that enabled RLS · has a `CREATE POLICY`? · # raw-pool repos touching
it · which repos. A **no policy** row means RLS is `ENABLE`'d with no policy defined: non-owner
roles get deny-all, but the owner (the app) bypasses it and sees **all** rows — so if the table is
tenant-scoped this is both a present leak *and* a future deny-all the moment it is `FORCE`'d
without first adding a policy.

### Tier S — critical (44 tables)

| Table | Mig | Policy | #repos | Raw-pool repos |
|---|---|---|---:|---|
| `audit_logs` | 00025 | yes | 5 | `audit_log.rs`, `data_export.rs`, `dispute.rs`, `membership.rs`, `platform_admin.rs` |
| `invoices` | 00040 | yes | 2 | `financial.rs`, `owner_analytics.rs` |
| `leases` | 00052 | yes | 2 | `lease.rs`, `multi_currency.rs` |
| `account_transactions` | 00039 | yes | 1 | `financial.rs` |
| `cross_border_leases` | 00101 | **no policy** | 1 | `multi_currency.rs` |
| `currency_conversion_audit` | 00101 | **no policy** | 1 | `multi_currency.rs` |
| `delegation_audit_log` | 00010 | yes | 1 | `delegation.rs` |
| `e_signature_nonces` | 00170 | yes | 1 | `e_signature_nonce.rs` |
| `fee_schedules` | 00039 | yes | 1 | `financial.rs` |
| `financial_accounts` | 00039 | yes | 1 | `financial.rs` |
| `fine_payments` | 00098 | yes | 1 | `violations.rs` |
| `investor_profiles` | 00096 | yes | 1 | `investor_portal.rs` |
| `invoice_items` | 00040 | yes | 1 | `financial.rs` |
| `late_fee_configs` | 00040 | yes | 1 | `financial.rs` |
| `lease_amendments` | 00052 | yes | 1 | `lease.rs` |
| `lease_documents` | 00089 | yes | 1 | `lease_abstraction.rs` |
| `lease_extractions` | 00089 | yes | 1 | `lease_abstraction.rs` |
| `lease_imports` | 00089 | yes | 1 | `lease_abstraction.rs` |
| `lease_payments` | 00052 | yes | 1 | `lease.rs` |
| `lease_reminders` | 00052 | yes | 1 | `lease.rs` |
| `lease_templates` | 00052 | yes | 1 | `lease.rs` |
| `multi_currency_transactions` | 00101 | **no policy** | 1 | `multi_currency.rs` |
| `online_payment_sessions` | 00040 | yes | 1 | `financial.rs` |
| `payment_allocations` | 00040 | yes | 1 | `financial.rs` |
| `payment_webhook_dead_letters` | 00240 | yes | 1 | `financial.rs` |
| `payments` | 00040 | yes | 1 | `financial.rs` |
| `regulatory_submission_audit` | 00062 | yes | 1 | `government_portal.rs` |
| `tenant_applications` | 00052 | yes | 1 | `lease.rs` |
| `tenant_screenings` | 00052 | yes | 1 | `lease.rs` |
| `unit_credit_balances` | 00040 | yes | 1 | `financial.rs` |
| `unit_fees` | 00039 | yes | 1 | `financial.rs` |
| `user_2fa` | 00024 | yes | 1 | `two_factor_auth.rs` |
| `valuation_audit_logs` | 00095 | yes | 1 | `property_valuation.rs` |
| `vote_audit_log` | 00014 | yes | 1 | `vote.rs` |
| `capability_grants` | 00138 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `impersonation_tokens` | 00138 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `mfa_recovery_codes` | 00149 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `reserve_fund_transactions` | 00057 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `screening_consent_audit_log` | 00070 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `screening_consent_requests` | 00070 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `screening_consents` | 00070 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `screening_data_access_requests` | 00070 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `screening_data_processing_records` | 00070 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `two_factor_auth_verifications` | 00138 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |

### Tier A — high / core tenant data (64 tables)

| Table | Mig | Policy | #repos | Raw-pool repos |
|---|---|---|---:|---|
| `buildings` | 00007 | yes | 22 | `announcement.rs`, `building.rs`, `community.rs`, `compliance.rs`, `data_export.rs`, `energy.rs` +16 |
| `units` | 00008 | yes | 19 | `announcement.rs`, `building.rs`, `data_export.rs`, `energy.rs`, `fault.rs`, `financial.rs` +13 |
| `unit_residents` | 00009 | yes | 8 | `announcement.rs`, `building.rs`, `data_export.rs`, `person_month.rs`, `registry.rs`, `unit.rs` +2 |
| `unit_owners` | 00008 | yes | 4 | `building.rs`, `owner_analytics.rs`, `registry.rs`, `unit.rs` |
| `announcements` | 00015 | yes | 3 | `announcement.rs`, `compliance.rs`, `data_export.rs` |
| `faults` | 00013 | yes | 3 | `data_export.rs`, `fault.rs`, `platform_admin.rs` |
| `meter_readings` | 00041 | yes | 3 | `energy.rs`, `financial.rs`, `meter.rs` |
| `meters` | 00041 | yes | 3 | `energy.rs`, `financial.rs`, `meter.rs` |
| `delegations` | 00010 | yes | 2 | `delegation.rs`, `vote.rs` |
| `person_months` | 00011 | yes | 2 | `meter.rs`, `person_month.rs` |
| `vote_questions` | 00014 | yes | 2 | `data_export.rs`, `vote.rs` |
| `vote_responses` | 00014 | yes | 2 | `data_export.rs`, `vote.rs` |
| `votes` | 00014 | yes | 2 | `data_export.rs`, `vote.rs` |
| `announcement_attachments` | 00015 | yes | 1 | `announcement.rs` |
| `announcement_comments` | 00016 | yes | 1 | `announcement.rs` |
| `announcement_reads` | 00015 | yes | 1 | `announcement.rs` |
| `building_package_settings` | 00068 | yes | 1 | `package_visitor.rs` |
| `building_registry_rules` | 00067 | yes | 1 | `registry.rs` |
| `building_visitor_settings` | 00068 | yes | 1 | `package_visitor.rs` |
| `capital_calls` | 00096 | yes | 1 | `investor_portal.rs` |
| `community_rules` | 00098 | yes | 1 | `violations.rs` |
| `consumption_aggregates` | 00041 | yes | 1 | `meter.rs` |
| `data_export_requests` | 00026 | yes | 1 | `data_export.rs` |
| `document_templates` | 00036 | yes | 1 | `document_template.rs` |
| `enforcement_actions` | 00098 | yes | 1 | `violations.rs` |
| `facilities` | 00012 | yes | 1 | `facility.rs` |
| `facility_bookings` | 00012 | yes | 1 | `facility.rs` |
| `fault_attachments` | 00013 | yes | 1 | `fault.rs` |
| `fault_timeline` | 00013 | yes | 1 | `fault.rs` |
| `government_portal_connections` | 00062 | yes | 1 | `government_portal.rs` |
| `investment_portfolios` | 00096 | yes | 1 | `investor_portal.rs` |
| `investor_dashboard_metrics` | 00096 | yes | 1 | `investor_portal.rs` |
| `investor_distributions` | 00096 | yes | 1 | `investor_portal.rs` |
| `investor_reports` | 00096 | yes | 1 | `investor_portal.rs` |
| `missing_reading_alerts` | 00041 | yes | 1 | `meter.rs` |
| `ocr_meter_corrections` | 00196 | yes | 1 | `meter.rs` |
| `packages` | 00068 | yes | 1 | `package_visitor.rs` |
| `parking_spots` | 00067 | yes | 1 | `registry.rs` |
| `person_month_submissions` | 00250 | yes | 1 | `person_month.rs` |
| `pet_registrations` | 00067 | yes | 1 | `registry.rs` |
| `portfolio_properties` | 00096 | yes | 1 | `investor_portal.rs` |
| `reading_submission_windows` | 00041 | yes | 1 | `meter.rs` |
| `reading_validation_rules` | 00041 | yes | 1 | `meter.rs` |
| `regulatory_report_templates` | 00062 | yes | 1 | `government_portal.rs` |
| `regulatory_submission_attachments` | 00062 | yes | 1 | `government_portal.rs` |
| `regulatory_submission_schedules` | 00062 | yes | 1 | `government_portal.rs` |
| `regulatory_submissions` | 00062 | yes | 1 | `government_portal.rs` |
| `roi_calculations` | 00096 | yes | 1 | `investor_portal.rs` |
| `smart_meter_providers` | 00041 | yes | 1 | `meter.rs` |
| `smart_meter_reading_logs` | 00041 | yes | 1 | `meter.rs` |
| `utility_bill_distributions` | 00041 | yes | 1 | `meter.rs` |
| `utility_bills` | 00041 | yes | 1 | `meter.rs` |
| `vehicle_registrations` | 00067 | yes | 1 | `registry.rs` |
| `violation_appeals` | 00098 | yes | 1 | `violations.rs` |
| `violation_comments` | 00098 | yes | 1 | `violations.rs` |
| `violation_evidence` | 00098 | yes | 1 | `violations.rs` |
| `violation_notifications` | 00098 | yes | 1 | `violations.rs` |
| `violation_statistics` | 00098 | yes | 1 | `violations.rs` |
| `violations` | 00098 | yes | 1 | `violations.rs` |
| `visitors` | 00068 | yes | 1 | `package_visitor.rs` |
| `vote_comments` | 00014 | yes | 1 | `vote.rs` |
| `package_notifications` | 00068 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `signature_requests` | 00037 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `visitor_access_logs` | 00068 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |

### Tier B — medium / analytics, derived, reference, config (64 tables)

> **Triage caveat:** several Tier B rows are plausibly **global reference data**, not
> tenant-scoped — e.g. `exchange_rates`, `exchange_rate_fetch_log`, `subscription_plans`,
> `subscription_coupons`, `webhook_event_types`, and the `market_*` reference tables. For a
> genuinely global table, `ENABLE`-without-`FORCE` (or no tenant policy) can be intentional. These
> are flagged for a **tenant-vs-global decision**, not automatic forcing. Confirm each against its
> creating migration before acting.

| Table | Mig | Policy | #repos | Raw-pool repos |
|---|---|---|---:|---|
| `background_jobs` | 00072 | yes | 3 | `background_jobs.rs`, `infrastructure.rs`, `operations.rs` |
| `article_comments` | 00069 | yes | 1 | `news_article.rs` |
| `article_media` | 00069 | yes | 1 | `news_article.rs` |
| `article_reactions` | 00069 | yes | 1 | `news_article.rs` |
| `article_views` | 00069 | yes | 1 | `news_article.rs` |
| `avm_property_valuations` | 00095 | yes | 1 | `property_valuation.rs` |
| `avm_property_value_history` | 00095 | yes | 1 | `property_valuation.rs` |
| `background_job_executions` | 00125 | yes | 1 | `background_jobs.rs` |
| `cma_property_comparisons` | 00088 | yes | 1 | `market_pricing.rs` |
| `comparable_adjustments` | 00095 | yes | 1 | `property_valuation.rs` |
| `comparative_market_analyses` | 00088 | yes | 1 | `market_pricing.rs` |
| `currency_exposure_analysis` | 00101 | **no policy** | 1 | `multi_currency.rs` |
| `digest_notifications` | 00061 | yes | 1 | `granular_notification.rs` |
| `distributed_spans` | 00079 | yes | 1 | `infrastructure.rs` |
| `distributed_traces` | 00079 | yes | 1 | `infrastructure.rs` |
| `event_notification_preferences` | 00038 | yes | 1 | `granular_notification.rs` |
| `exchange_rate_fetch_log` | 00101 | **no policy** | 1 | `multi_currency.rs` |
| `exchange_rates` | 00101 | **no policy** | 1 | `multi_currency.rs` |
| `extraction_corrections` | 00089 | yes | 1 | `lease_abstraction.rs` |
| `favorite_providers` | 00103 | yes | 1 | `marketplace.rs` |
| `grouped_notifications` | 00061 | yes | 1 | `granular_notification.rs` |
| `health_alert_rules` | 00080 | yes | 1 | `infrastructure.rs` |
| `held_notifications` | 00038 | yes | 1 | `granular_notification.rs` |
| `market_data_points` | 00088 | yes | 1 | `market_pricing.rs` |
| `market_regions` | 00088 | yes | 1 | `market_pricing.rs` |
| `market_statistics` | 00088 | yes | 1 | `market_pricing.rs` |
| `multi_currency_report_config` | 00101 | **no policy** | 1 | `multi_currency.rs` |
| `multi_currency_report_snapshots` | 00101 | **no policy** | 1 | `multi_currency.rs` |
| `news_articles` | 00069 | yes | 1 | `news_article.rs` |
| `notification_digests` | 00061 | yes | 1 | `granular_notification.rs` |
| `notification_groups` | 00061 | yes | 1 | `granular_notification.rs` |
| `notification_schedule` | 00038 | yes | 1 | `granular_notification.rs` |
| `organization_currency_config` | 00101 | **no policy** | 1 | `multi_currency.rs` |
| `performance_portfolios` | 00100 | yes | 1 | `portfolio_performance.rs` |
| `portfolio_aggregated_metrics` | 00091 | **no policy** | 1 | `portfolio_analytics.rs` |
| `portfolio_alert_rules` | 00091 | **no policy** | 1 | `portfolio_analytics.rs` |
| `portfolio_alerts` | 00091 | **no policy** | 1 | `portfolio_analytics.rs` |
| `portfolio_benchmarks` | 00091 | **no policy** | 1 | `portfolio_analytics.rs` |
| `portfolio_trends` | 00091 | **no policy** | 1 | `portfolio_analytics.rs` |
| `pricing_recommendations` | 00088 | yes | 1 | `market_pricing.rs` |
| `property_comparisons` | 00091 | **no policy** | 1 | `portfolio_analytics.rs` |
| `property_currency_config` | 00101 | **no policy** | 1 | `multi_currency.rs` |
| `property_performance_metrics` | 00091 | **no policy** | 1 | `portfolio_analytics.rs` |
| `property_valuation_features` | 00095 | yes | 1 | `property_valuation.rs` |
| `property_valuation_models` | 00095 | yes | 1 | `property_valuation.rs` |
| `provider_reviews` | 00103 | yes | 1 | `marketplace.rs` |
| `reminder_schedules` | 00040 | yes | 1 | `financial.rs` |
| `rfqs` | 00103 | yes | 1 | `marketplace.rs` |
| `role_notification_defaults` | 00038 | yes | 1 | `granular_notification.rs` |
| `unit_pricing_history` | 00088 | yes | 1 | `market_pricing.rs` |
| `valuation_comparables` | 00095 | yes | 1 | `property_valuation.rs` |
| `valuation_market_data` | 00095 | yes | 1 | `property_valuation.rs` |
| `valuation_reports` | 00095 | yes | 1 | `property_valuation.rs` |
| `valuation_requests` | 00095 | yes | 1 | `property_valuation.rs` |
| `article_comment_reactions` | 00069 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `organization_feature_preferences` | 00085 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `outage_notifications` | 00086 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `outages` | 00086 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `reminder_logs` | 00040 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `saved_searches` | 00046 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `subscription_coupons` | 00059 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `subscription_plans` | 00059 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `tenant_settings` | 00137 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |
| `webhook_event_types` | 00102 | yes | 0 | _(no raw-pool repo; RlsConnection/admin path)_ |

---

## 4. Finding 2 — raw-pool repositories (the active exposure surface)

42 repository units query a non-`FORCE` table on a raw pool field with zero `set_request_context`.
`Worst` = highest sensitivity tier among the tables each touches; `S/A/B` = per-tier table counts.

| Repository | Worst | #non-FORCE tables | S | A | B |
|---|:---:|---:|---:|---:|---:|
| `financial.rs` | S | 17 | 12 | 4 | 1 |
| `multi_currency.rs` | S | 12 | 4 | 1 | 7 |
| `data_export.rs` | S | 10 | 1 | 9 | 0 |
| `property_valuation.rs` | S | 10 | 1 | 0 | 9 |
| `lease.rs` | S | 9 | 7 | 2 | 0 |
| `violations.rs` | S | 9 | 1 | 8 | 0 |
| `vote.rs` | S | 9 | 1 | 8 | 0 |
| `investor_portal.rs` | S | 8 | 1 | 7 | 0 |
| `government_portal.rs` | S | 6 | 1 | 5 | 0 |
| `lease_abstraction.rs` | S | 4 | 3 | 0 | 1 |
| `owner_analytics.rs` | S | 4 | 1 | 3 | 0 |
| `platform_admin.rs` | S | 4 | 1 | 3 | 0 |
| `delegation.rs` | S | 2 | 1 | 1 | 0 |
| `audit_log.rs` | S | 1 | 1 | 0 | 0 |
| `dispute.rs` | S | 1 | 1 | 0 | 0 |
| `e_signature_nonce.rs` | S | 1 | 1 | 0 | 0 |
| `membership.rs` | S | 1 | 1 | 0 | 0 |
| `two_factor_auth.rs` | S | 1 | 1 | 0 | 0 |
| `meter.rs` | A | 14 | 0 | 14 | 0 |
| `market_pricing.rs` | A | 9 | 0 | 2 | 7 |
| `portfolio_analytics.rs` | A | 9 | 0 | 2 | 7 |
| `granular_notification.rs` | B | 8 | 0 | 0 | 8 |
| `registry.rs` | A | 8 | 0 | 8 | 0 |
| `announcement.rs` | A | 7 | 0 | 7 | 0 |
| `package_visitor.rs` | A | 6 | 0 | 6 | 0 |
| `fault.rs` | A | 5 | 0 | 5 | 0 |
| `news_article.rs` | B | 5 | 0 | 0 | 5 |
| `person_month.rs` | A | 5 | 0 | 5 | 0 |
| `building.rs` | A | 4 | 0 | 4 | 0 |
| `energy.rs` | A | 4 | 0 | 4 | 0 |
| `infrastructure.rs` | B | 4 | 0 | 0 | 4 |
| `unit.rs` | A | 4 | 0 | 4 | 0 |
| `marketplace.rs` | B | 3 | 0 | 0 | 3 |
| `unit_resident.rs` | A | 3 | 0 | 3 | 0 |
| `background_jobs.rs` | B | 2 | 0 | 0 | 2 |
| `compliance.rs` | A | 2 | 0 | 2 | 0 |
| `facility.rs` | A | 2 | 0 | 2 | 0 |
| `rental/mod.rs` | A | 2 | 0 | 2 | 0 |
| `community.rs` | A | 1 | 0 | 1 | 0 |
| `document_template.rs` | A | 1 | 0 | 1 | 0 |
| `operations.rs` | B | 1 | 0 | 0 | 1 |
| `portfolio_performance.rs` | B | 1 | 0 | 0 | 1 |

**Blind-spot cohort (29 repos not in any existing baseline/allowlist):**
`audit_log.rs`, `background_jobs.rs`, `delegation.rs`, `document_template.rs`,
`e_signature_nonce.rs`, `facility.rs`, `fault.rs`, **`financial.rs`**, `government_portal.rs`,
`granular_notification.rs`, `infrastructure.rs`, `investor_portal.rs`, **`lease.rs`**,
`lease_abstraction.rs`, `market_pricing.rs`, `meter.rs`, **`multi_currency.rs`**, `news_article.rs`,
`operations.rs`, `package_visitor.rs`, `person_month.rs`, `portfolio_analytics.rs`,
`property_valuation.rs`, `registry.rs`, **`two_factor_auth.rs`**, `unit.rs`, `unit_resident.rs`,
`violations.rs`, **`vote.rs`**.

The bolded repos are the most alarming: `financial.rs` (12 Tier-S tables — invoices, payments,
accounts, fees), `lease.rs` (leases, amendments, tenant screenings/applications — contract + PII),
`multi_currency.rs` (currency transactions + 5 no-policy currency tables), `two_factor_auth.rs`
(`user_2fa`), `vote.rs` (`vote_audit_log` + ballots). None are caught by CI today.

The remaining 13 repos (`announcement.rs`, `building.rs`, `community.rs`, `compliance.rs`,
`data_export.rs`, `dispute.rs`, `energy.rs`, `marketplace.rs`, `membership.rs`,
`owner_analytics.rs`, `platform_admin.rs`, `portfolio_performance.rs`, `rental/mod.rs`) **are**
already in `rls-pool-field-baseline.txt` (they also touch `FORCE`'d tables), so they are tracked —
but the baseline entry only reflects their `FORCE`-table exposure, not these additional non-`FORCE`
reads.

> **Caveat on repo-level counting.** The detector (and this audit) classify at the *unit* level:
> a repo that mixes raw-pool methods with `RlsConnection`-executor methods will not show
> `set_request_context` in the pool-field methods but may still be partly safe. Per-method
> confirmation is required during conversion (§6) before asserting a concrete leak for any one
> endpoint. This audit scopes and ranks; it does not prove exploitability per method.

---

## 5. Finding 3 — 17 tables with RLS `ENABLE`'d but **no policy**

These enable RLS yet define no `CREATE POLICY`. Under the owner connection they currently return
all rows (bypass); the day they are `FORCE`'d without a policy they flip to deny-all. They must be
triaged **policy-first**:

- Multi-currency (mig `00101`): `cross_border_leases` (Tier S), `currency_conversion_audit` (S),
  `multi_currency_transactions` (S), `currency_exposure_analysis`, `exchange_rate_fetch_log`,
  `exchange_rates`, `multi_currency_report_config`, `multi_currency_report_snapshots`,
  `organization_currency_config`, `property_currency_config`.
- Portfolio analytics (mig `00091`): `portfolio_aggregated_metrics`, `portfolio_alert_rules`,
  `portfolio_alerts`, `portfolio_benchmarks`, `portfolio_trends`, `property_comparisons`,
  `property_performance_metrics`.

`exchange_rates` / `exchange_rate_fetch_log` are likely global reference (see §3 caveat); the
`cross_border_leases` / `*_audit` / `*_transactions` members are tenant financial data and should
be policy+FORCE'd. The portfolio-analytics set is tenant-derived analytics (Tier B) — add a
tenant policy then FORCE, mirroring the owner-analytics precedent.

---

## 6. Backlog — proposed remediation (ranked, batched)

The proven pattern is already in-tree: a FORCE migration mirroring `00179`/`00172`
(rewrite the policy to the canonical `get_current_org_id()` GUC where the policy used a
mismatched/stale GUC, then `ALTER TABLE … FORCE ROW LEVEL SECURITY`), paired with converting the
offending repository to the **executor pattern** (take `impl Executor` / `&mut PgConnection` from
the `RlsConnection` extractor, like `document.rs` / `board_meetings.rs`), plus an IG3-style
regression test that fails cross-tenant before the fix and passes after. Do **migration + repo
conversion together per cluster** so a FORCE never lands while a raw-pool reader of that table is
still live (that would turn a leak into a deny-all outage).

### P0 — Tier S, actively exposed (do first)

| # | Cluster (migrations) | Tables | Repo(s) to convert |
|---|---|---|---|
| S1 | Invoices & payments `00040`/`00039`/`00240` | invoices, invoice_items, payments, payment_allocations, online_payment_sessions, payment_webhook_dead_letters, unit_credit_balances, account_transactions, financial_accounts, fee_schedules, unit_fees, late_fee_configs | `financial.rs` |
| S2 | Lease management & abstraction `00052`/`00089` | leases, lease_amendments, lease_payments, lease_reminders, lease_templates, tenant_applications, tenant_screenings, lease_documents, lease_extractions, lease_imports | `lease.rs`, `lease_abstraction.rs` |
| S3 | Auth / credentials `00024`/`00138`/`00149` | user_2fa, two_factor_auth_verifications, mfa_recovery_codes, capability_grants, impersonation_tokens | `two_factor_auth.rs` (+ confirm the RlsConnection/admin paths for the 0-repo tables) |
| S4 | Multi-currency `00101` (policy-first — see §5) | cross_border_leases, currency_conversion_audit, multi_currency_transactions (+ reference-table triage) | `multi_currency.rs` |
| S5 | Audit trails `00025`/`00010`/`00014`/`00062`/`00095` | audit_logs, delegation_audit_log, vote_audit_log, regulatory_submission_audit, valuation_audit_logs | `audit_log.rs`, `delegation.rs`, `vote.rs`, `government_portal.rs`, `property_valuation.rs`, `data_export.rs`, `dispute.rs`, `membership.rs`, `platform_admin.rs` |
| S6 | GDPR screening `00070` + investor PII `00096` | screening_consents, screening_consent_requests, screening_consent_audit_log, screening_data_access_requests, screening_data_processing_records, investor_profiles, reserve_fund_transactions, fine_payments | `investor_portal.rs`, `violations.rs` (+ confirm RlsConnection paths for screening tables) |

### P1 — Tier A, core tenant data

| # | Cluster (migrations) | Tables (count) | Repo(s) |
|---|---|---|---|
| A1 | Buildings/units/residents `00007`–`00011` | buildings, units, unit_owners, unit_residents, person_months, person_month_submissions | `building.rs`, `unit.rs`, `unit_resident.rs`, `person_month.rs` (+ the many fan-in repos) |
| A2 | Faults `00013` | faults, fault_attachments, fault_timeline | `fault.rs` |
| A3 | Voting & delegation `00014`/`00010` | votes, vote_questions, vote_responses, vote_comments, delegations | `vote.rs` |
| A4 | Announcements `00015`/`00016` | announcements (+4) | `announcement.rs` |
| A5 | Violations `00098` | violations (+6) | `violations.rs` |
| A6 | Meters & utilities `00041` | meters, meter_readings, utility_bills (+9) | `meter.rs`, `energy.rs` |
| A7 | Registries `00067` | parking_spots, pet_registrations, vehicle_registrations (+2) | `registry.rs` |
| A8 | Packages & visitors `00068` | packages, visitors (+5) | `package_visitor.rs` |
| A9 | Facilities `00012` | facilities, facility_bookings | `facility.rs` |
| A10 | Government portal `00062` | government_portal_connections, regulatory_submissions (+4) | `government_portal.rs` |
| A11 | Investor portal `00096` | capital_calls, investor_distributions, investor_reports, roi_calculations, investment_portfolios, portfolio_properties, investor_dashboard_metrics | `investor_portal.rs` |

### P2 — Tier B, analytics / reference / config

| # | Cluster (migrations) | Notes | Repo(s) |
|---|---|---|---|
| B1 | Portfolio analytics `00091` (no-policy) | add tenant policy then FORCE | `portfolio_analytics.rs` |
| B2 | Market pricing & valuation `00088`/`00095` | tenant-derived; triage `market_*` reference vs tenant | `market_pricing.rs`, `property_valuation.rs` |
| B3 | Notification grouping/prefs `00061`/`00038` | FORCE + executor conversion | `granular_notification.rs` |
| B4 | News/media `00069` | FORCE + executor conversion | `news_article.rs` |
| B5 | Infra/jobs/tracing `00072`/`00079`/`00080`/`00125` | likely service-role — classify before forcing | `infrastructure.rs`, `background_jobs.rs`, `operations.rs` |
| B6 | Marketplace `00103` | FORCE + executor conversion | `marketplace.rs` |
| B7 | Global-reference triage | decide tenant-vs-global for `exchange_rates`, `subscription_plans/coupons`, `webhook_event_types`, `market_*` | — |

### P3 — cross-cutting: close the detector blind spot (do alongside P0)

Extend `backend/scripts/check-rls-enforcement.sh` so the pool-field detector **also** flags
repositories whose SQL touches an `ENABLE`-but-not-`FORCE` table — not only the `FORCE` set.
Concretely: derive a second table set `enable_not_force_final` (the ENABLE set minus the FORCE
set minus an explicit *global/reference* exemption list) and run the same four-criteria classifier
against it, seeding a `rls-nonforce-pool-field-baseline.txt` with the 29-repo cohort so the ratchet
only tightens from here. Without this, every repo converted in P0–P2 could silently regress.

---

## 7. Appendix — regenerate this audit

Static, deterministic, no DB needed. Analysis scripts used for this pass are reproducible from the
two primitives the repo already standardises on:

```bash
# (1) existing enforcement detector — FORCE set only (today's CI view)
cd backend && bash scripts/check-rls-enforcement.sh        # --strict to fail

# (2) ENABLE-vs-FORCE gap, per table (the widened view this audit adds):
#  - ENABLE set:  ALTER TABLE … ENABLE ROW LEVEL SECURITY   across migrations/*.sql
#  - FORCE  set:  ALTER TABLE … FORCE  ROW LEVEL SECURITY   across migrations/*.sql
#  - gap = ENABLE − FORCE  (172 tables as of 2026-11-25)
#  - raw-pool repos: pool/db field + &self.pool|db + no set_request_context,
#                    SQL FROM|JOIN|INTO|UPDATE ∩ gap
```

Counts are a 2026-11-25 snapshot of `backend/crates/db`. Re-run after any migration that enables
RLS or converts a repository, and fold new numbers back into §2.
