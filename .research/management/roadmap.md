# PPT Roadmap — upkeep 2026-11-06

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 24h lag since 2026-11-05 run._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on all 3 epic-10a stories (cursor idx 0 → 1 — negative).
- Delta vs 2026-11-05 upkeep: **21 PRs merged** — the biggest tenant-isolation + follow-up remediation wave to date. Round-5 authz A/B (#3331, #3332), Round-5 functional (#3330, migrations 00241/00242/00243), pet/vehicle tenant scoping (#3338), 10× post-merge-review follow-ups (gh-issue-3304..3315), suspend_cascade coverage (#3339), screen-map-drift reconciliations (#3334–#3337, #3340).
- Screen coverage: 0 orphan screens · 0 validation errors · drift carried over (3 known missing UC links already queued elsewhere).
- **Three 'Still open' items from #3332 body** captured as new action-list rows this run: accounting-server by-id RLS retirement, webhook headers auth-smuggling hardening, onboarding ResolvedTenant application.
- Open PRs this window: 18 (6 non-dependabot: #3333 screen-map-drift-pr-3293-reality; #3301 inquiry FK; #3294 draft ws-jwt; #2559, #2555 stalled feats from July).

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done.
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done. **Re-checked this run (cursor idx 0).** No merged PR in the 2026-11-05..2026-11-06 window touched OAuth surfaces; `last_checked = 2026-11-06` stamped on all 3 stories.
- **Epic 10B — Platform Administration:** 7/7 done. Round-5 authz A/B + 10 post-merge follow-ups hardened tenant isolation across this epic's surface.
- **Epic 80 — Dispute Resolution:** 3/3 done.
- **Epic 81 — Reports:** 2/2 done.
- **Epic 82 — iOS Reality Portal (SwiftUI):** 5/5 done.
- **Epic 83 — Portal Integrations:** 3/3 done.
- **Epic 84 — Documents & e-signature:** 5/5 done.
- **Epic 85 — Build/CI configuration:** 2/2 done.
- **Epic 79 — API client integration:** 4/4 done.
- **Epic 9 — 2FA/TOTP:** 1/1 done.

## Ranked plan

### Phase (post-MVP hardening — all 4 named sprint epics complete)

- [high] Retire the accounting-server RLS-only by-id methods (invoice/contact/match) — move to tenant-id-predicated variants · owner: pm-scrum-master (dispatches to rust-backend) · why: closes #3332 'Still open' item 1; parity with the rest of api-server; silent cross-tenant read is the easiest IDOR surface left.
- [high] Harden EnhancedWebhookSubscription.headers against auth-header smuggling — reject or strip Authorization/Cookie/X-API-Key on write; omit from read-response schema · owner: pm-scrum-master · why: closes #3332 'Still open' item 2; outbound-webhook exfil vector.
- [high] Apply the ResolvedTenant rule to onboarding — role selection must not consult the X-Tenant header · owner: pm-scrum-master · why: closes #3332 'Still open' item 3; the only remaining role-pick surface not on the ResolvedTenant contract.
- [high] Add regression tests for accounting-server by-id endpoints — cross-tenant id → 404 · owner: pm-qa · depends_on: accounting-server by-id retirement · why: pm-qa DoD policy — fix PR requires failing-on-main test.
- [high] Add webhook-headers auth-smuggling regression test — create/update rejects or strips auth header names; schema omits them · owner: pm-qa · depends_on: webhook headers fix · why: pm-qa DoD policy.
- [high] Add onboarding ResolvedTenant regression test — POST /onboarding with foreign X-Tenant → 403 · owner: pm-qa · depends_on: onboarding ResolvedTenant application · why: pm-qa DoD policy.
- [medium] Design-decide audit_logs ON DELETE SET NULL FK + append-only trigger conflict — ADR covering CASCADE vs NO ACTION vs extend-ACCESS-EXCLUSIVE-escape-hatch · owner: pm-scrum-master → pm-tech-lead · why: #3330 flagged landmine; seed-force handled, normal-op delete path not.
- [medium] Automate Round-5 regression suite at CI level — mark check-rls-enforcement.sh --strict + round5_sec_b_authz_tests + suite_7 property_cluster_org_scope as required on dev · owner: pm-scrum-master → pm-devops · why: lock in the hardening work; prevent regressions.
- [medium] Add i18n regression for mobile RN Alert.alert hardcoded strings — DocumentDetailScreen/WidgetSettingsScreen/FeedbackScreen · owner: pm-qa · depends_on: pm-frontend · why: 4-locale app; 73-day-old lint-gate risk re-raised.

### Screen-map drift

- (no new drift this run; the ppt-web/reality-web route changes in merged PRs were paired with doc updates.)

Buffer: 29/36 open · 10 new candidates queued this run.
