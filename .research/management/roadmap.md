# PPT Roadmap — upkeep 2026-11-07

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 10-day lag since 2026-10-28 run._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on the 3 epic-10a OAuth stories (cursor re-check — negative).
- Delta vs 2026-10-28 upkeep: **2 PRs merged** — both frontend code-review fixes (reality-web shared-comparison validation; ppt-web registry i18n). Neither flips a coverage story (extended UI surfaces).
- **Two action-list items resolved → done**: `code-review-reality-web-share-cmp-unchecked-json` (PR #3350), `code-review-ppt-web-ui-registry-no-i18n` (PR #3351).
- **No new blockers; six carried.** Cross-tenant assign-404 (#3187), cargo-deny advisories (#3168), violations add_evidence authz (#3164), merge pipeline deadlock (#3131), KMP cloud builds (#2652), RN jest-expo (#2951).
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 already tracked) · 1 carried screen-map-drift item (#3293 reality-web realtor/[id]).
- Open PRs this window: 0 touched.

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done.
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done. **Re-checked this run (cursor idx 0).** No merged PR in the 2026-10-29..2026-11-07 window touched OAuth surfaces; `last_checked = 2026-11-07` stamped on all 3 stories.
- **Epic 10B — Platform Administration:** 7/7 done.
- **Epic 80 — Dispute Resolution:** 3/3 done.
- **Epic 81 — Reports:** 2/2 done.
- **Epic 82 — iOS Reality Portal (SwiftUI):** 5/5 done.
- **Epic 83 — Portal Integrations:** 3/3 done.
- **Epic 84 — Documents & e-signature:** 5/5 done.
- **Epic 85 — Build/CI configuration:** 2/2 done.
- **Epic 79 — API client integration:** 4/4 done.
- **Epic 9 — 2FA/TOTP:** 1/1 done.

## Ranked plan

### merge-queue unblock (highest leverage, score 10)

- [high] **Resolve #3168 cargo-deny advisories** (3 new RUSTSEC advisories on dev) — bump the offending transitive crate(s) or add bounded cargo-deny exceptions with a deadline — gates every backend merge today — owner: **pm-security** (carried)
- [high] **Fix #3187 cross-tenant assign-404 contract** — either `assign_violation` returns 404 on cross-tenant, or the test's role shape is adjusted — gates backend CI on dev — owner: **pm-security** (carried)
- [high] Unblock mobile-native/KMP builds in cloud runner (**#2652**) — 5+ KMP items structurally unclaimable — owner: **pm-devops** (carried)

### tier-1 security (score 8-9)

- [high] **Gate violations add_evidence sub-resource (#3164)** with a tenant-scoped authz check; add cross-tenant integration test on the `community_cross_tenant_idor_tests.rs` pattern — owner: **pm-backend** (carried)

### IG3 / test-discipline (score 7-8) — new this run

- [medium] **Adopt CI diff-guard for IG3 TDD gate on `auto-impl/*` branches** — mechanize the two-commit TDD contract that PR #3350 modelled (test commit must fail on base_ref; fix commit must pass) — before the pattern decays — owner: **pm-qa + pm-tech-lead**
- [medium] **Extend reality-web parseListingSummary coverage** — three surviving negative cases (null address, array body, HTML error-page 200) atop #3350's TDD seed; same sweep for parseListingDetail — owner: **pm-qa**
- [medium] **Adopt eslint-plugin-react-hooks + i18next/no-literal-string** on `frontend/apps/mobile` (blocking, warn-only for one sprint) — closes the chronic mobile-rn defect class behind the carried `mobile-rn lint-gap` risk — owner: **pm-frontend** (carried, 74d unresolved)

### frontend "fake-submit" cluster (score 6-7) — carried from pm-frontend 2026-10-28

- [high] Wire `ReportFaultScreen.tsx` handleSubmit to real `/api/v1/faults` POST (currently `setTimeout(1500)` fake-submit — fault reports silently discarded) — owner: **pm-frontend**
- [high] Wire reality-web `/sell` wizard Publish to reality-server POST — every seller submission silently discarded today — owner: **pm-frontend**
- [high] Audit ppt-web logout AUTHED_QUERY_KEY_ROOTS allow-list — TanStack caches leak across users on a shared workstation — owner: **pm-frontend**

### backend reliability (score 6-7)

- [medium] Diagnose + resolve #3131 merge-pipeline deadlock — long-stale; likely sqlx transaction scope or advisory-lock contention — owner: **pm-backend** (carried)
- [medium] **Violations xtenant-404-contract sweep** across moderation.rs / aml_dsa.rs — PR #3183 fixed most, #3187 revealed assign still returns 403 — owner: **pm-scrum-master → pm-security** (carried)

### refactor-with-security-overtones (score 5-6) — carried `gh-issue-33xx` follow-ups

- [medium] #3304 — fold `common::TenantRole::from_role_type` instead of role parsing (PR #3291)
- [medium] #3305 — fold IoT org-scope check into `add_reading` insert (PR #3289)
- [medium] #3306 — consolidate per-file `require_manager(rls)` helpers onto shared `rls_authz::require_manager_rls` (PR #3287)
- [medium] #3307 — fault update/triage/status: use `find_by_id_for_org` instead of manual guard (PR #3286)
- [medium] #3308 — RLS-aware repo variants for vote sub-resource reads (PR #3281)
- [medium] #3309 — enforce stable key order for `messages/*.json` (PR #3260 reordering)
- [medium] #3310 — community feed shows every author as "Neighbor"; formatMoney Decimal coercion (PR #3269)

### quality (score 4-5, carried)

- [medium] Resolve mobile RN jest-expo version rot (**#2951**) — owner: **pm-devops** (carried)
- [medium] #3006 — unify triplicated auth-header + MFA-retry in `@ppt/api-client` fetch primitives — owner: **pm-tech-lead** (carried)

### KMP code-review backlog (score 2-3, all blocked on #2652) — carried

- CreateListingScreen onSubmit stub
- InquiriesResponse page_size vs reality-server `limit` mismatch
- Ktor HttpClient has no HttpTimeout
- getPortfolioAnalytics() 100-cap + unbounded fan-out
- KMP SSO error-body unguarded parse

### Screen-map drift

- [low] **Sweep #3304-#3310 for screen-map drift** — none reference `docs/screens/`; dispatcher pre-merge gate recommended (new this run) — owner: **pm-qa**
- Carried: `screen-map-drift-pr-3293-reality` (reality-web realtor/[id] i18n without screen-map update) · 3 missing UC links (UC-33.1/33.2/33.3)

### ppt-web i18n parity lock (score 2-3) — new this run

- [low] **Add registry sub-tree parity assertion** across en/sk/cs after PR #3351 — de/pl/hu fall back to en by design — owner: **pm-qa**

Buffer: **15/36 open** · candidates ranked but unqueued: ~0 (coverage is all `done` — refill depends on the dispatcher's external backlog scan, not this upkeep run). ⚠ Buffer below half — consider running scan to refresh coverage; dispatcher-side backlog scan is overdue to replenish.
