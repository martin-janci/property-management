# PPT Project State

_Generated: 2026-05-29 — daily PM rotation (Scrum Master + pm-integration). Coverage `scan_kind=upkeep`; last deep scan 2026-05-23. Rotating-epic re-check this run: epic-81 (coverage_cursor idx 6 → 7/epic-82)._

## Executive summary

- **11 PRs merged this window (#717–#730), plus late-merges below the prior #709 cursor (#597/#657/#659/#685/#695/#706).** The application-code slice was small: **#718** iOS gesture-mask + sheet-env (LocationManager) fixes + SSO CSRF tests (closes #618/#625/#578); **#719** gap-84-2 e-signature signerParties (manager/landlord) + cs/de i18n — resolves all 6 PR#513 follow-ups; **#720** gap-10b-3 admin Platform Health MFA-interception test coverage; **#724** gap-10a-4 OAuth scope picker + scope-grant audit trail. The bulk of activity was research/dispatcher infrastructure (#717/#721/#722/#726/#727/#729/#730) and Dependabot.
- **Story coverage: 27 done / 22 partial / 0 not-started (49 total).** 84-2 e-signature email moved not-started → partial (#719). 2 of 13 epics fully done (epic-8a, epic-9); epic-10b is effectively complete in coverage (7/7 done) though sprint-status.yaml is stale.
- **New security finding this run (rotating Rust review, api-core):** a high-confidence cross-tenant IDOR cluster in the Epic-64 LLM-document handlers (`ai.rs` publish/list/get) — `publish_description` is a state-mutating IDOR (publish another tenant's listing description by UUID). Promoted to `plans/security-llm-doc-idor.md`. Distinct from in-flight PR #725 (maintenance/chat-session/sentiment IDOR).
- **11 open PRs, none stalled (oldest 1 day).** #725 (ai-maintenance IDOR fix) sits at verdict=changes; #662 (reports cross-tenant IDOR, closes #646/#647) is unreviewed; gap-82 mobile drafts (#639/#641/#705) and #723 (MFA recovery backend) await review. Dependabot #666 bumps **sqlx 0.8→0.9** (a workspace-wide DB-layer major bump — flagged as integration risk).
- **5 follow-up/from-merged-review issues CLOSED this window** (#578/#581/#618/#625/#629); no new untriaged issues.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

| Epic | Tracked status | Real status (from coverage upkeep) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6-1/6-5/6-6 done; 6-2/6-3/6-4 web UI partial (drafts) |
| 7A — Basic Document Management | in-progress | 7a-3/7a-5 done; 7a-2 folder web tested, mobile slice open; 7a-1/7a-4 partial |
| 8A — Basic Notification Preferences | done | 8a-1/8a-2 done; 8a-3 WS leg confirmed (#597), mobile-push leg open → partial |
| 10A — OAuth Provider | in-progress | 10a-1/10a-2/10a-3 done; scope picker UI shipped (#724); revoked-token bypass (#481) still gates production |
| 10B — Platform Admin | ready-for-dev (STALE) | 10b-1..10b-7 all done in coverage (#695/#706/#720); **sprint-status.yaml needs sync** |
| 81 — Reports | in-progress | 81-1/81-2 partial; #643 closed RBAC #614 + tenant-scope #624; remaining: cron_expression column (#616) + exec-history download/retry e2e |
| 82 — SwiftUI Reality Portal | in-progress | all 5 stories partial; #688 wired listing/favorites, #718 iOS fixes; drafts #639/#641/#705 in flight; no reality-mobile screen-maps |
| 84 — Advanced Features | in-progress | 84-1/84-3/84-4/84-5 done; 84-2 e-signature now partial (#719) |

## What's next (top actions)

1. **[high] Review + merge #662 (reports cross-tenant IDOR, closes #646/#647)** — owner: pm-security — unblocks Epic 81 authz promotion.
2. **[high] Land `plans/security-llm-doc-idor.md`** — owner: pm-security/pm-backend — new state-mutating cross-tenant IDOR on the LLM-document publish/list/get handlers.
3. **[high] Resolve #725 verdict=changes (ai-maintenance/session/sentiment IDOR + missing test)** — owner: pm-security — closes the maintenance IDOR vector.
4. **[high] Audit sqlx 0.9 (PR #666) before merge** — owner: pm-backend — workspace-wide query!/migrate breakage risk; freeze from auto-merge.
5. **[medium] Sync sprint-status.yaml to coverage reality (10b done, 8a-3 WS done)** — owner: pm-scrum-master.

## Blockers

- **Epic 81 — Reports promotion:** cron_expression column still missing (#616 / backlog `bug-report-schedule-update-no-sql`); 81-1/81-2 stay partial. (RBAC #614 + tenant-scope #624 now closed by #643.)
- **OAuth production readiness (Epic 10A):** refresh-token revocation bypass (#481, high) and JWT-in-WS-logs (#480, high) remain open; gate any external OAuth exposure.
- **Mobile coverage lag:** 10 of 22 partial stories are mobile; gap-82 drafts unreviewed.

## Role focus today

- **Role focus today:** Scrum Master + pm-integration.
- **pm-integration read:** integration surface carries three open reliability defects — Airbnb at-least-once webhook duplicates SYNC_EXTERNAL jobs, the Redis push-fanout queue is never drained (silent drop), and the marketplace install/OAuth UI is still stubbed — against active OAuth-provider work and a sqlx 0.9 major bump that touches every repository. Added the sqlx-0.9 audit + e-signature webhook idempotency guard as tracked actions; flagged the four integration risks.
