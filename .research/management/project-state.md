# PPT Project State

_Generated: 2026-10-28 — routine Phase 1.6 incremental upkeep. Role focus: **pm-frontend** (83-day stale rotation slot; pm-cursor idx 2 → 3). Coverage `scan_kind=upkeep`; coverage_cursor idx 12 → 0 (epic-9 TOTP 2FA re-checked, 1 story still done, no material change). pm-scrum-master always-on._

## Delta since last routine run (2026-10-27T01:07Z)

- **7 PRs merged** (#3177, #3178, #3179, #3180, #3181, #3182, #3183) — all narrow fixes with tests:
  - **#3177** — ppt-web sprint-status epic rollups fix (display-layer)
  - **#3178** — IoT WS reconnect test harden
  - **#3179** — reality-web price locale fix (i18n defect)
  - **#3180** — community IDOR test cover
  - **#3181** — assign_violation org-membership gate
  - **#3182** — moderation assign/unassign 404 fix
  - **#3183** — violations mutation cross-org IDOR regression + fix
  - **Zero coverage status flips** — coverage is 100% done; these are code-review-driven hardenings. (#3181/#3182/#3183 collectively close the violations/moderation tier-1 IDOR cluster surfaced over the last two weeks.)
- **1 open blocker: #3187** — `cross_org_assign_violation_is_not_found` red on dev (assign returns 403, expected 404). Dispatcher opened draft PR **#3188** awaiting human review. (Carried: #3168 cargo-deny RUSTSEC context.)
- **4 open draft dispatcher PRs:** #3184, #3185, #3186, #3188 — all `needs-human-review`.
- **Mobile RN quality signal:** rotating-expert pass (2026-10-27) surfaced two new VoteDetailScreen findings — ranked ballots silently collapse to unordered multi-select; `?? 'English'` fallbacks are dead code. Neither moves a coverage story (coverage already 100% done) but confirms a persistent frontend quality debt the schema can't see.

<sub>Prior upkeep note (archived): 2026-10-26 — pm-backend rotation; coverage_cursor 11 → 12 (epic-8a re-checked).</sub>

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on the 1 epic-9 story.
- **7 PRs merged** — three tier-1 IDOR hardenings on the violations/moderation stack, one test harden, one reality-web i18n fix, two display-layer fixes.
- **Buffer at 20/36 open** — above the half-empty floor (18) after three new pm-frontend items landed this run. Dispatcher backlog scan remains overdue to top the queue.
- **pm-frontend rotation came up 83 days stale** — last run 2026-08-06; today's lens surfaces persistent mobile RN i18n/hooks lint-gap debt that keeps generating code-review items, plus a reality-web `[locale]/*` churn cluster that deserves Playwright smokes after #3179.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** at sprint level (sprint-status.yaml); coverage-level all extended epics are complete.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6)† | 6/6 done | — |
| 7A — Basic Document Management | in-progress (2/5)† | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B — Platform Administration | in-progress (7/7)† | 7/7 done | — |
| 80 — Dispute Resolution | partial (1/3)† | 3/3 done | — |
| 79 — API client integration | (extended) | 4/4 done | — |
| 82 — iOS Reality Portal (SwiftUI) | (extended) | 5/5 done | — |
| 83 — Portal Integrations | (extended) | 3/3 done | — |
| 84 — Documents & e-signature | (extended) | 5/5 done | — |
| 85 — Build/CI configuration | (extended) | 2/2 done | — |
| 9 — 2FA/TOTP | (extended) | 1/1 done · **last_checked 2026-10-28 (cursor re-check)** | — |
| Cross-cutting (violations/moderation) | — | — | #3178, #3180, #3181, #3182, #3183 (tier-1 IDOR + test cover cluster — none map to coverage stories) |
| Cross-cutting (reality-web / ppt-web) | — | — | #3177, #3179 |

† sprint-status.yaml drift against coverage.json — carried as low-priority action item.

## Shipped since last run (7 PRs merged)

- **#3177** — ppt-web: sprint-status epic rollups fix
- **#3178** — IoT WS reconnect test harden
- **#3179** — reality-web: price locale fix
- **#3180** — community IDOR test cover
- **#3181** — api-server: assign_violation org-membership gate
- **#3182** — api-server: moderation assign/unassign 404 fix
- **#3183** — api-server: violations mutation cross-org IDOR regression + fix

## Blockers

- **#3187 (open, high)** — cross-tenant assign_violation 403 vs expected 404 red on dev; draft PR #3188 awaiting human review. Owner: **pm-security**.
- **#3131** — merge pipeline deadlock (carried, medium). Owner: pm-backend.
- **#2652** — mobile-native/KMP cloud builds blocked (carried, high). Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot (carried, medium). Owner: pm-devops.

## What's next (top 5 from the roadmap)

1. **Resolve #3187** (land dispatcher draft PR #3188 after human review) — owner: **pm-security** — unblocks backend CI on dev.
2. **Finish the `add_evidence` authz sweep (#3164)** on violations and audit sibling `add_*` sub-routes (announcements/attachments, documents/signatures, mediation/evidence) — owner: **pm-backend**.
3. **Ship VoteDetailScreen fix pass** — rank-position UI + remove dead `?? 'English'` i18n fallbacks — owner: **pm-frontend** (NEW).
4. **Adopt eslint-plugin-react-hooks + i18next-hardcoded-strings** in `frontend/apps/mobile`, gate CI on it — owner: **pm-frontend** (NEW).
5. **Diagnose #3131 merge-pipeline deadlock** — long-stale; needs a traced repro — owner: **pm-backend** (carried).

## Role focus today: pm-frontend (+ pm-scrum-master always-on)

- **pm-frontend** (83-day stale rotation): Concrete pattern — mobile RN quality debt leaks past CI as code-review-only findings (2 fresh on VoteDetailScreen 2026-10-27 — ranked-UI gap, dead i18n fallback). Three next_actions queued: VoteDetailScreen fix pass, eslint plugin adoption, reality-web `[locale]/*` sweep + Playwright smokes. One new risk: `pm-frontend-mobile-rn-i18n-and-hooks-lint-gap-2026-10-28`. Carried pm-qa risk `pm-qa-mobile-rn-lint-gap-hooks-i18n-2026-08-25` is now explicitly actionable through the lint-adoption item.
- **pm-scrum-master**: Hardening-heavy window — 7 PRs merged (tier-1 IDOR cluster closed), 4 drafts open, #3187 remains the single dev-blocking issue (pending human review on PR #3188). Buffer healthier at 20/36 after three new pm-frontend items landed.

## Coverage (upkeep this run — 2026-10-28)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-10-28T03:11:13Z`, no re-scan.
- **Epic re-check: epic-9 2FA/TOTP (cursor idx 12)** — the single story (9-1 TOTP 2FA Setup) is still `done`; no merged PR in the 2026-10-26..2026-10-28 window touched 2FA/TOTP enroll/confirm/verify/recovery-code surfaces. `last_checked = 2026-10-28` stamped.
- **Merged-PR evidence check:** #3177..#3183 — none map to a coverage story (no evidence appended).
- **`coverage_cursor` advances 12 → 0** (epic-9 → epic-10a next run; 13 distinct epics wrap).
- **`pm_cursor` advances 2 → 3** (pm-frontend → pm-qa next). role_last_run["pm-frontend"] = 2026-10-28.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, carried).
