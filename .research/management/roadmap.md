# PPT Roadmap — upkeep 2026-10-31

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 3-day lag since 2026-10-28 run._

> **BUFFER-LOW BANNER (dispatcher payload this run)**
> `claimable=17/72 (coverage exhausted: 0 non-done stories, backlog-refill avail=0, retry-remint avail=0) — refill planner / author new coverage`.
> The default-mode ranker has **nothing to rank from coverage** (every story is `done`). The ranked plan below is driven entirely by role-returned next_actions, carried action-list items, and screen/orphan gaps. The highest-leverage task is **authoring new coverage** — until that lands, the dispatcher runs only on hotspot/churn candidates.

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on epic-10a (cursor idx 0 — negative; no OAuth-provider touches in the merge window).
- **Delta vs 2026-10-28 upkeep: 8 PRs merged** (#3185, #3186, #3188, #3189, #3190, #3191, #3197, #3198). #3188 closes #3187 (dev-CI blocker). None of the 8 PRs map to a coverage-status story flip.
- **New: planner-level blocker** — coverage exhaustion. Candidate-source quality will degrade until new coverage is authored.
- **Carried blockers unchanged**: #3168 cargo-deny, #3164 add_evidence authz, #3131 merge pipeline, #2652 KMP cloud, #2951 jest-expo.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 already queued elsewhere).
- Open PRs this window: 14 total (10 dependabot likely cargo-deny-gated; 4 non-dependabot drafts: #3171, #3075, #3108, #3192, #3193).

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done
- **Epic 7A — Basic Document Management:** 5/5 done
- **Epic 8A — Basic Notification Preferences:** 3/3 done
- **Epic 10A — OAuth Provider Foundation:** 3/3 done · **last_checked 2026-10-31 (cursor re-check)**
- **Epic 10B — Platform Administration:** 7/7 done
- **Epic 80 — Dispute Resolution:** 3/3 done
- **Epic 81 — Reports:** 2/2 done
- **Epic 82 — iOS Reality Portal (SwiftUI):** 5/5 done
- **Epic 83 — Portal Integrations:** 3/3 done
- **Epic 84 — Documents & e-signature:** 5/5 done
- **Epic 85 — Build/CI configuration:** 2/2 done
- **Epic 79 — API client integration:** 4/4 done
- **Epic 9 — 2FA/TOTP:** 1/1 done

## Ranked plan

### planner-refill (highest leverage this week)

- [high] **Author new coverage** — enumerate phase-2/3 epics beyond the current 13 (candidates: owner analytics, portfolio compliance, i18n hardening, offline-mobile, mediation UX) and lift them into `coverage.json` as `(epic, story-id, title, phase, status: not-started)` rows — owner: **pm-scrum-master + pm-tech-lead** — unblocks the dispatcher's claimable backlog from the degraded `17/72` state.

### regression / release-readiness (pm-qa this run)

- [high] **IG3 regression-test audit on the 8 merged PRs this window** (#3185-#3198) — confirm each has a failing-on-main test checked in; file follow-up issues for gaps — owner: **pm-qa**
- [high] **Cross-tenant IDOR regression harness** — extend PR #3183's `violations_mutation_cross_org_idor_tests` pattern to moderation.rs, aml_dsa.rs, `add_evidence` sub-routes, announcement/document attachment mutations — owner: **pm-qa + pm-backend**
- [medium] **Flaky-skip audit** across backend tests + vitest + jest-expo — produce silently-disabled-tests dashboard; coordinate with pm-devops on #2951 — owner: **pm-qa**
- [medium] **Release-readiness checklist for all-stories-done state** — explicit QA gate for the next release cut (regression green, no open P0 bugs, dependabot batch applied, cargo-deny clear) — owner: **pm-qa**
- [medium] **QA churn-hotspots → positive test-coverage expansions** (violations_role_authz_tests.rs, violations.rs, compliance_wave1b_tests.rs) — owner: **pm-qa**

### merge-queue unblock (carried, highest leverage, score 10)

- [high] **Resolve #3168 cargo-deny advisories** — gates every backend merge; the 10 open dependabot PRs are likely trapped behind this — owner: **pm-security**
- [high] Unblock mobile-native/KMP builds in cloud runner (#2652) — carried; 5+ KMP items structurally unclaimable — owner: **pm-devops**

### tier-1 security (carried, score 8-9)

- [high] **Gate violations add_evidence sub-resource (#3164)** with tenant-scoped authz check + cross-tenant integration test — owner: **pm-backend**
- [medium] **Sweep remaining violations xtenant 404 contract** across moderation.rs / aml_dsa.rs — carried; #3188 fixed `assign_violation` only, not the broader sweep — owner: **pm-backend / pm-scrum-master**

### draft-triage (medium)

- [medium] **Decide fate of 4 non-dependabot drafts** (#3171, #3075, #3108, #3192, #3193) — close, revive, or convert to a plan — owner: **pm-scrum-master**
- [medium] **Dependabot batch policy under cargo-deny #3168** — decide whether to batch-merge the 10 open dependabot PRs — owner: **pm-devops**

### Screen-map drift (low — carried)

- [low] **Screen-map drift PR #2994** (ppt-web route refactor) — reconcile frontmatter against current routes; `/screens validate` clean — owner: **pm-frontend / pm-qa**
- [low] **Link UC-33.1/33.2/33.3** to screen-maps (carried from previous rotation)

Buffer: 36/36 open · 0 candidates ranked but unqueued (coverage exhausted)
