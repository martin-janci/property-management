# PPT Project State

_Generated: 2026-11-06 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (73-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a re-checked — 3/3 stories still `done`, no PRs touched OAuth 10a-* in the window). pm-scrum-master always-on._

## Delta since last routine run (2026-11-05 03:17 UTC)

- **15 PRs merged** in the last ~21h — a dense security/hardening burst:
  - **11 `gh-issue-*` follow-ups** (#3318–#3328) each closing one of the open merged-review issues #3304–#3315 (all closed). Topics: reality-web messages key order (#3309→#3318), formatMoney Decimal precision (#3310→#3319), consolidate `require_manager_rls` (#3306→#3320), RecentActivity faults scoped to reporter (#3315→#3321), RLS-aware org-scoped vote sub-resource reads (#3308→#3322), resolution-term `responsiblePartyId` via party-id map (#3312→#3323), lease payments via the honest route (#3311→#3324), manager-gate automation execution list/stats (#3314→#3325), reuse `TenantRole::from_role_type` in escalation (#3304→#3326), fault update/triage/status org isolation in SQL (#3307→#3327), IoT org-scope folded into reading-ingest insert (#3305→#3328).
  - **4 "round-5" api-server PRs** — #3330 functional fixes (energy schema, alert rules, seed `--force`, dashboard layout, outages types, automation i18n), #3331 authz A (API keys, package/visitor settings, attachments, rentals, news media, Booking.com push), #3332 authz B (accounting, buildings, certifications, registry, maintenance, platform checks), #3338 scope pet/vehicle registration reads to caller's units. Together they added ~2100 lines of new authz test coverage.
- **No new blockers surfaced; no new open issues created.**
- **Draft in flight**: 5 dispatcher screen-map-drift reconciliations (#3333–#3337) plus #3339 churn-hotspot suspend_cascade test coverage — all in draft awaiting CI / review.
- **PR #3301** (reality-server inquiry-dead-fk-classify) still open, non-draft, awaiting reviewer.
- **Carried blockers unchanged**: #3168 (cargo-deny RUSTSEC advisories), #2652 (KMP cloud builds), #2951 (RN jest-expo), #3131 (merge pipeline deadlock); #3187 cross-org-assign-404 resolved by PR #3327.

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** Sprint tracking still flat — the sprint named in `sprint-status.yaml` has every epic `done` and nothing `in-progress`; today's shipped work is all post-sprint hardening streamed through the dispatcher.
- **15 PRs merged in 24h — all security/hardening**. The authz-sweep (A + B + pet/vehicle) tightened cross-tenant + role gates across accounting, buildings, certifications, registry, maintenance, platform, API keys, settings, attachments, rentals, news media, and the Booking.com push path. The follow-up stream from merged-review closed every open follow-up issue in that batch. This is the strongest security-signal day since the initial IDOR sweep.
- **pm-qa rotation came up 73 days stale** — last run 2026-08-25. 5 new pm-qa next_actions queued (three high: CI execution-proof for round-5 suites; required-status `/screens validate` gate; assign implementers to the two ready KMP plans. Two medium: churn-hotspot test seed-isolation review for #3339; authz-fix mutation test gate. One low: QA gate on negative tests for authz PRs). Three new risks queued (mobile-coverage blockers, screen-map check advisory-vs-required, round-5 suite happy-path-only false confidence).
- **New blockers on dev CI**: none. The 2026-10-28 blocker #3187 (cross-tenant assign 404) is resolved by PR #3327 this run.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 6/6** at tracked level (sprint-status reconciled 2026-10-26; nothing moved today). Coverage holds 13 epics all at `done` (49/49 stories).

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — (re-checked today; no change) |
| 10B — Platform Administration | done | 7/7 done | — |
| Follow-up stream (post-merge review) | n/a (not a sprint epic) | n/a | #3318–#3328 |
| Round-5 authz sweep | n/a (not a sprint epic) | n/a | #3330, #3331, #3332, #3338 |

## Shipped since last run

15 PRs — see the Delta block above for the full list. No regressions reverted; no stuck-in-draft blockers.

## What's next (top 5)

1. **Review + merge PR #3301** (reality-server inquiry-dead-fk-classify) — the only non-draft, non-dependabot open PR. Owner: pm-tech-lead.
2. **Confirm round-5 authz suites actually exercise cross-tenant/wrong-role negatives in CI** (not just happy-path 200s). Spot-check `round5_sec_b_authz_tests.rs`, `r5_sec_a_authz_tests.rs`, `registry_read_scope_tests.rs`. Owner: pm-qa + pm-backend.
3. **Assign implementers to the two ready mobile-native-kmp plans** (`inquiries-response-contract`, `create-listing-not-wired`) — both have been ready > 2 weeks with no owner. Owner: pm-frontend / pm-scrum-master.
4. **Hold draft screen-map auto-fix PRs #3333–#3337** until `/screens validate` is green and the check is required-status on `dev`. Owner: pm-qa + pm-devops.
5. **Decide sprint rotation** — `sprint-status.yaml` is stale; either define a new sprint scope or formally move to hardening/maintenance cadence. Owner: pm-product / pm-scrum-master.

## Blockers

- #2652 — KMP cloud builds blocked (carried from 2026-08-31; now 67 days)
- #2951 — RN jest-expo (carried from 2026-08-25; now 73 days)
- #3168 — cargo-deny RUSTSEC advisories (carried)
- #3131 — merge pipeline deadlock (carried)

## Role focus today

pm-scrum-master (always on) + **pm-qa** (daily rotation).

Per-role one-liner summary this run:
- **pm-scrum-master:** Sprint's finished (49/49 done). Recommend reviewing/merging #3301, triaging 9 open dependabot PRs, reviewing draft #3339, picking up the 5 screen-map-drift drafts, and either defining the next sprint or formally moving to maintenance cadence; also flagged the authz "rounds" pattern suggests we keep finding unaudited routes — a route-level authz lint is probably overdue.
- **pm-qa:** Round-5 suites are wired (not compile-only), but need CI execution proof + negative-assertion spot-check; screen-map auto-fix PRs need a required-status gate; mobile plans need implementers; `jest-expo`/KMP-cloud-build blockers need a release-gate decision.
