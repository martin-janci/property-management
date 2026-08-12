# PPT Project State

_Generated: 2026-08-12 by daily research routine (Phase 1.6, pm-qa rotation, buffer-low refill run)_

## Executive summary

35 PRs landed on `dev` in the 5 days since the last routine run (2026-08-07 → 2026-08-12), covering reality-server hardening (agency-members IDOR, sso-session-invalidate, db-error-leak, password-reset), api-core resilience (workflow SSRF-TOCTOU, memory-DoS caps, fail-closed on parse errors, quiet-hours drain), reports CSV formula-injection sanitization (partial), integrations layout dedupe, i18n gaps closed on ppt-web/mobile-rn/reality-web, and auto-fix churn-hotspot regression tests. **The landable backlog pool is now drained** — dispatcher signalled `claimable=4/72`, all high-signal work either has an open PR or is mobile-native/KMP (unlandable in cloud). This run refills 12 new landable backlog rows + 12 action-list items and promotes 2 plans (CSV-injection audit, quiet-hours drain integration test) to reopen the landable lane.

## Sprint progress

- **Sprint:** Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth
- **Epics done:** 2 / 6

## Shipped since last run (2026-08-07 → 2026-08-12, 35 PRs)

- **Reality-server hardening:** db-error-leak, sso-session-invalidate, password-reset transport, agency-members unauth IDOR
- **API-core resilience:** workflow SSRF-TOCTOU (#2710), workflow api_call memory DoS (#2707), condition-parse fail-closed (attempts #2684 still quarantined), quiet-hours drain fix (#2729), voice-actions honest failure (#2728)
- **Security:** reports CSV formula-injection (vote titles only, #2731)
- **Refactors:** layout tenant/admin/auth handler dedupe (#2711/#2713/#2715), reports helpers extraction (#2720)
- **i18n gaps closed:** ppt-web offline-indicator/protectedroute, mobile-rn meterdetail/date-locale, reality-web listing-form
- **Auto-fix hotspots:** useOfflineSupport remap-on-persist, rentals mappers regression tests, ai/llm partial-batch fail-closed

## What's next (top 5)

1. **[high]** Run refill planner: mine coverage.json partial stories + churn hotspots (webhook.rs, scheduler/mod.rs, financial.rs, buildings.rs, lease.rs) for landable candidates — **owner: dispatcher / pm-tech-lead** (this run added 12 landable backlog rows + 12 action-list items + 2 promoted plans; verify buffer >18 open after next dispatcher tick)
2. **[high]** Merge two low-risk mobile-rn PRs already through review (#2739 meterdetail-i18n, #2740 date-locale) — **owner: reviewer**
3. **[high]** Decide fate of quarantined #2684 (workflow cond-parse fail-open): scope down to minimal deny_unknown_fields fix or hand off to manual rust-backend review — **owner: pm-tech-lead**
4. **[high]** CSV formula-injection audit repowide (GDPR export, reports, market_pricing) — **owner: pm-qa / pm-security** (plan promoted: `plans/code-review-security-csv-injection-audit-repowide.md`)
5. **[high]** Add DB-integration test for QuietHoursDrainWorker.drain_due() — **owner: pm-qa** (plan promoted: `plans/code-review-api-core-quiet-hours-drain-untested.md`)

## Blockers

- **Landable backlog pool** — claimable=4/72; non-KMP backlog drained to 5 items, one quarantined (#2684). Refill in progress via this run's 12 new backlog rows and 2 promoted plans. (owner: dispatcher / pm-tech-lead)
- **PR #2684 (workflow cond-parse-failopen)** — fix_rounds=3 exhausted, CI red on test-shard 1-4 at commit 926afe8, quarantined. Needs manual scope-down decision. (owner: rust-backend specialist / pm-tech-lead)
- **mobile-native/KMP backlog (4 items)** — Kotlin can't compile in cloud sandbox — no execution lane for these fixes. Policy decision pending. (owner: pm-tech-lead)

## Role focus today

pm-scrum-master (always-on), pm-qa (rotation slot 3 → advanced to 4 next)

## Per-role summary

- **pm-scrum-master** — 35 PRs landed; landable backlog drained; refill priority. Top blockers: buffer-low, PR #2684 quarantined, mobile-native-KMP unlandable lane.
- **pm-qa** — high-value gaps: QuietHoursDrainWorker DB-integration untested (drain_due invariant not fenced); CSV-injection fix landed for vote titles only, sibling exports (GDPR/reports/market_pricing) likely share the pattern; AUTHED_QUERY_KEY_ROOTS structural guard missing after 3rd miss.

## Coverage upkeep

- Coverage cursor advanced 5→6 (epic-80 → epic-81); epic-80 fully done, no changes.
- Coverage stories: 47 done / 2 partial (84-1 S3 direct-upload frontend wiring, 84-2 signer document-sign page) — both promoted as backlog rows this run and refilled into action-list (`gap-84-1-s3-direct-upload-frontend-wire`, `gap-84-2-document-sign-page-ppt-web`).
