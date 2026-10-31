# PPT Project State

_Generated: 2026-10-31 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (67-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a re-checked; all 3 OAuth stories still done). pm-scrum-master always-on._

## Delta since last routine run (2026-10-28)

- **6 PRs merged** on 2026-10-30 20:35–20:51 UTC — a 16-minute batch, all carrying `needs-human-review`:
  - **#3185** code-review-ppt-web-core-neighbors-stale-token-usememo — ppt-web `useNeighbors` + `usePrivacySettings` capture current auth token (empty-dep `useMemo` dropped).
  - **#3186** code-review-ppt-web-core-iot-ws-hasgivenup-dropped — IoT dashboard consumes `hasGivenUp` + offline banner (touches `routes/groups/iot.tsx` — **triggers `screen-map-drift-pr-3186-ppt`**).
  - **#3188** gh-issue-3187 fix cross-org IDOR in `assign_violation` (403 → 404). Unblocks dev CI (`cross_org_assign_violation_is_not_found` red test now passes).
  - **#3189** code-review-mobile-rn-vote-detail-ranked-no-order — mobile RN VoteDetailScreen renders ranked questions as an ordered ballot.
  - **#3190** pm-frontend-ppt-web-authed-roots-session-leak-2026-10-28 — queryKeys.test.ts audit enforcing logout-purge coverage for `personMonthKeys`.
  - **#3191** code-review-ppt-web-ui-invoice-qty-price-nan — ppt-web AccountingInvoiceForm rejects NaN qty/unitPrice.
- **1 issue closed** (#3187 cross-tenant assign-404 contract gap) — resolved by #3188.
- **15 open PRs touched**: `#3171` iterating (COMMENTED), `#3192/#3193/#3197/#3198` dispatcher drafts, `#3075`/`#3108`/`#3119` still stalled, 7 dependabot PRs rebased by weekly sweep.
- **Action-list changes:** `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` → done (closed by #3188). **12 new actions queued** (6 pm-scrum-master + 6 pm-qa) — buffer up to 40 open / 36 target (above floor).

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-10a (idx 0, OAuth Provider Foundation — stories 10a-1/10a-2/10a-3 all still done, nothing in the window touched OAuth surfaces).
- **6 PRs merged** — a batch flush of research-driven fixes. 5 of 6 shipped regression tests; #3188 is covered by #3183's earlier regression suite. Review debt real: **all 6 PRs carry `needs-human-review`** at merge.
- **pm-qa rotation came up 67 days stale** — last run 2026-08-25. Six pm-qa next_actions queued (two high — mobile-native-kmp runway + inquiries contract test, two medium — #3119 triage + draft gating, one each on authz matrix + test-gap scheduling); one new high-severity risk (`pm-qa-mobile-native-kmp-has-high-confidence-ready-plans-...`).
- **Churn hotspots widening**: `violations.rs` (5 touches/14d, repeated), `reality-web/.../agency/[slug]/page.tsx` (5 touches, **new hotspot**), `backend/deny.toml` (5 touches, repeated — tracking cargo-deny advisory churn).
- **Carried blockers unchanged**: mobile-native-kmp cloud-unlandable (#2652), stalled drafts #3075/#3108/#3119, UC-ACC Phase-A #2555/#2559 (91 days operator-gated).

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level (all four named sprint epics are status: done in sprint-status.yaml after the 2026-10-26 reconciliation). Coverage holds 13 epics all at done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done · **last_checked 2026-10-31 (cursor re-check)** | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | #3188 (violations xtenant 404 contract) |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |
| 9 — 2FA/TOTP | done (1/1) | 1/1 done | — |
| 79/82/83/84/85 (extended) | — | all done | — |

## Shipped since last run (6 PRs merged)

- **#3185** — code-review-ppt-web-core-neighbors-stale-token-usememo (ppt-web auth-token refresh fix in useNeighbors + usePrivacySettings).
- **#3186** — code-review-ppt-web-core-iot-ws-hasgivenup-dropped (IoT dashboard consumes `hasGivenUp`).
- **#3188** — gh-issue-3187: fix cross-org IDOR in assign_violation (403 → 404) — unblocks backend CI on `dev`.
- **#3189** — code-review-mobile-rn-vote-detail-ranked-no-order (mobile RN ranked ballot UI).
- **#3190** — pm-frontend-ppt-web-authed-roots-session-leak (TanStack cache purge coverage regression).
- **#3191** — code-review-ppt-web-ui-invoice-qty-price-nan (NaN guard on invoice qty/unitPrice).

## What's next (top 5 from ranked plan)

1. **[high] Human review pass on the 6 merged `needs-human-review` PRs, prioritising #3188 (security IDOR contract change) and #3190 (session leak)** — owner: pm-scrum-master.
2. **[high] Route the two ready mobile-native-kmp plans to a runner with Android egress, or park them operator-gated so they stop counting as ready** — owner: pm-qa.
3. **[high] Land #3171 (add_evidence manager role gate), then rebase and finish #3075 (violations number-count race) sequentially** — owner: pm-scrum-master.
4. **[high] Add JVM-only contract test for /inquiries (fixture + kotlinx-serialization) OR backend OpenAPI-vs-DTO contract check** — owner: pm-qa.
5. **[high] Triage security-flavoured drafts #3192 (appeal no status guard), #3193 (moderation report flood), #3198 (os-rng expect panics): mark ready and request review** — owner: pm-scrum-master.

## Blockers

- **backlog: `code-review-mobile-native-kmp-inquiries-response-contract`** — mobile-native-kmp cannot land from cloud (AGP egress block, #2652). Owner: pm-tech-lead.
- **backlog: `code-review-mobile-native-kmp-create-listing-not-wired`** — same AGP egress block. Owner: pm-tech-lead.
- **PR #3075 / #3171 (violations.rs cluster)** — overlapping edits (5 churn/14d); #3075 draft 15d, #3171 iterating. Rebase/conflict risk. Owner: pm-backend.
- **PR #3119 NewsScreen i18n** — 13d open, non-draft, no reviewDecision. Owner: pm-frontend.
- **(carried) PR #2555 / #2559 UC-ACC Phase-A** — 91d in review, operator-gated.
- **(carried) #2652 AGP egress block** — mobile-native-kmp cloud-unlandable (keystone risk).

## Role focus today

- **pm-qa** (role_last_run was 2026-08-25 — longest stale slot in rotation; next_index 3 → 4).
- **pm-scrum-master** (always-on).

### Per-role summary

- **pm-scrum-master**: Review-queue drain + a stalled violations.rs cluster are the two near-term fronts. All MVP sprint epics done since Oct — a new sprint goal needs defining from coverage.json gaps.
- **pm-qa**: Test discipline strong this week (5/6 PRs shipped regression tests), but structural gaps remain: 2 ready mobile-native-kmp plans cannot land, stale #3119, hotspots lacking a coverage view. Six actions queued; two high (mobile-native-kmp runway + /inquiries JVM contract test).
