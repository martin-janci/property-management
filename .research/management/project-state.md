# PPT Project State

_Generated: 2026-10-01 — routine Phase 1.6 upkeep (pm-security rotation slot; pm-cursor idx 5 → 6) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 7 → 8 (epic-82 re-checked, no material change; advances to epic-83). 4-day lag since last routine run (2026-09-27)._

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** Sprint-status.yaml still shows some epics as in-progress/partial (6, 7A, 10B, 80) — that's sprint-level drift against the deeper coverage.json truth and does not reflect new regressions; a deep scan would reconcile it.
- **7 PRs merged** in the observation window since #2991: a defense-in-depth RBAC pass (role gates, protected routes), one SSO i18n fix, one bug boundary cast, one mobile feature wiring, one cross-cutting auth-client refactor. Zero coverage status flips (all touched surfaces already shipped).
- **Buffer-low alarm (new): `claimable=9/72`** (floor 36). Root cause is **#2966 utoipa-swagger-ui egress 403** blocking every backend PR — including all three open cross-tenant IDOR retries. Compounded by #2652 (KMP cloud builds blocked) and 11 stalled open PRs (bulk dependabot plus the 64d accounting trio).
- **No new blockers.** Standing infra blockers (#2966, #2951, #2946, #2652) carry forward.
- **One new noteworthy issue: #3006** — post-merge review of PR #3004 surfaced triplicated auth-header + MFA-retry logic across @ppt/api-client fetch primitives. Medium-priority refactor with security overtones.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** at sprint level (sprint-status.yaml); coverage-level all extended epics are complete.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6)† | 6/6 done | — |
| 7A — Basic Document Management | in-progress (2/5)† | 5/5 done | #3001 (pattern reuse: presigned S3 for fault photos — adjacent vertical) |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | #2993 (reality-web SSO locale) |
| 10B — Platform Administration | in-progress (7/7)† | 7/7 done | #2994 (ppt-web dashboard ProtectedRoute), #3002 (manager-dashboard role gate), #2995 (Home CTA role gate) |
| 80 — Dispute Resolution | partial (1/3)† | 3/3 done | — |
| 79 — API client integration | (extended) | 4/4 done | #3004 (gdprClient + paymentMatching → shared auth) |
| 82 — iOS Reality Portal (SwiftUI) | (extended) | 5/5 done · **last_checked 2026-10-01 (cursor re-check)** | — |
| Cross-cutting | — | — | #2996 (automation-rule boundary cast) |

† sprint-status.yaml drift against coverage.json — no action required this run; deep scan will reconcile.

## Shipped since last run (7 PRs merged since #2991)

- **#2993** — reality-web SSO locale: fix locale propagation through SSO consumer path (6 locales)
- **#2994** — ppt-web dashboard ProtectedRoute: tighten route-level RBAC on admin/manager dashboard
- **#2995** — Home CTA role gate: role-based visibility on Home dashboard CTAs
- **#2996** — automation-rule boundary cast: fix integer-boundary cast in automation rule evaluator
- **#3001** — mobile fault photos presigned: wire RN mobile fault reporting to presigned S3 upload (parity with 7a-1 upload pattern on a new vertical)
- **#3002** — manager dashboard role gate: role gate on manager dashboard entry
- **#3004** — gdprClient + paymentMatching via shared auth: migrate two @ppt/api-client fetch primitives onto shared authenticatedFetchJson (post-merge review surfaced #3006)

## Buffer-low alarm

- Dispatcher reports **claimable = 9 / 72 open** (floor 36). Buffer below half.
- Root cause 1 — **#2966 utoipa-swagger-ui egress 403** in the cloud runner. Every backend-touching dispatcher task is `CLOUD-BUILD-BLOCKED`. The 3 IDOR retries (#2944/#2945/#2946), the integrations-webhook refactor, and the AUTHED_QUERY_KEY_ROOTS session-leak item are all parked here.
- Root cause 2 — **#2652 mobile-native/KMP cloud build gate** still open; 6 KMP items are blocked downstream.
- Root cause 3 — reviewer starvation on the accounting trio **#2559** (PAY by Square, 64d) + **#2555** (invoice lifecycle, 64d). Not infra, but holds reviewer slots.

## Blockers (carried, none new)

- **#2966** — api-server utoipa-swagger-ui egress 403 (NEW top blocker — surfaced as highest-leverage infra fix). Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot. Blocks mobile-rn code-review items from landing. Owner: pm-devops.
- **#2946** — portfolio_analytics cross-tenant IDOR (retry 2/2 draft open as `gh-issue-2946-retry2`; blocked by #2966). Owner: pm-security.
- **#2652** — mobile-native/KMP builds unlandable in cloud runner. Owner: pm-devops.

## Infra follow-ups needed per #2966

1. **Decide vendoring vs egress-allowlist vs base-image-cache** for `utoipa-swagger-ui` crates.io assets. Any of the three unblocks all backend security PRs. (pm-devops)
2. **Pin + test** the chosen path in the cloud runner's base image. (pm-devops)
3. **Backfill a dispatcher trigger** that re-mints `gh-issue-294[456]-retryN` the moment the egress fix lands, so the IDOR fixes get first slot on the newly-unblocked runner. (pm-scrum-master + pm-devops)
4. **Add CI alert** on egress 403 from crates.io so the next instance of this class surfaces in minutes, not days. (pm-devops)

## Noteworthy new issue

- **#3006** — unify triplicated auth-header + MFA-retry in @ppt/api-client fetch primitives. Surfaced by post-merge review of PR #3004. Three copies of the same auth logic will drift (one will forget MFA-retry on a 403; one will send a stale token; one will log the token). Classic refactor-with-security-overtones target. Medium priority, owner pm-tech-lead.

## Role analysis — pm-security (2026-10-01)

**Open security-tagged items in backlog + issues (snapshot)**

- **Tier-1 blocker: #2946 portfolio_analytics cross-tenant IDOR** — still open. Retry 2/2 draft branch `gh-issue-2946-retry2` is pushed but has no landed PR; last `failed-no-pr` cycle 2026-09-21. The underlying issue is that the handler computes `org_id` then discards it before the SQL query — a one-line fix in principle, but the implementer branch keeps failing to produce a PR because the cloud runner can't compile api-server (#2966). The security tier does not change until the fix lands.
- **Tier-1 companions: #2944 + #2945** — same shape (cross-tenant IDOR), same parking lot behind #2966. Both at retry 1/2.
- **Standing security risks** (carried, no change): #2485 layout webhook replay, #2486 mobile layout-cache cross-tenant, #2483 add_evidence dispute sub-resource IDOR (awaiting #2490), #2574 SSO CSRF state fix non-functional (owner pm-mobile).

**Security-adjacent signals from the 7 merged PRs (all positive this window)**

- **PR #2994** — ppt-web dashboard ProtectedRoute hardened: route-level RBAC gate added on the manager/admin dashboard entry. Reduces the surface where a client-side session with a stale role could reach an admin dashboard.
- **PR #3002** — manager-dashboard role gate: companion to #2994 on the server-fetched side of the manager dashboard. Defense in depth.
- **PR #2995** — Home CTA role gate: Home dashboard CTAs now hidden for roles that cannot act on them. Low security value but removes a social-engineering vector.
- **PR #2993** — reality-web SSO locale: a correctness fix on the SSO consumer path. Not a security change per se, but touches the SSO exchange — no regression spotted; evidence appended on 10a-1 (OAuth authorization server) story.

**Medium-priority refactor-with-security-overtones**

- **#3006** — triplicated auth-header + MFA-retry. The security angle is that drift between the three copies is almost certain over time. One copy will omit MFA-retry on a 403 (user locked out). One will log the token on a transient error. One will send a stale token after refresh. Consolidate into a single `authenticatedFetchJson` primitive and add a biome rule forbidding new fetch primitives that do not import from the shared module.

**Suggested next security actions (3)**

1. **Unblock #2966 as a security dependency** — this is the single highest-leverage security move this cycle because it is the sole thing standing between the 3 open IDOR retries and a merged PR. Treat it as a security-tier work item routed to pm-devops with explicit security owner-sign-off.
2. **Hand-land #2946 locally** if #2966 slips another week — fix is one-line (restore the org_id filter in the portfolio_analytics query); add a cross-tenant repro test that fails on main as the IG3 anchor; push a reviewed PR manually to break the retry loop. Document the manual-land path for IDOR tier-1 bugs that get stuck behind cloud-build gates.
3. **Open #3006 as a tracked refactor** — fold the unify-auth-header work into the next frontend sprint; add the biome rule as part of the fix PR; audit the remaining @ppt/api-client fetch primitives for the same pattern (prediction: 1-2 more latent sites).

## Coverage (upkeep this run — 2026-10-01)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-10-01T04:30:00Z`, no re-scan.
- **Epic re-check: epic-82 (cursor idx 7)** — all 5 stories (82-1..82-5) still `done`; no PR in the 2026-09-28..10-01 window touched iOS SwiftUI RealityPortalApp surfaces. `last_checked = 2026-10-01` stamped on all 5. Evidence line appended on 82-5-inquiries-account noting the negative check.
- **Merged-PR evidence appended** on 10b-1-organization-management-dashboard (PR #2994 + #3002), 10a-1-oauth-authorization-server (PR #2993), 79-1-api-client-integration (PR #3004), 7a-1-document-upload-metadata (PR #3001, pattern reuse). No status flips.
- **`coverage_cursor` advances 7 → 8** (epic-82 → epic-83 next run).
- **`pm_cursor` advances 5 → 6** (pm-security → next role in rotation). role_last_run["pm-security"] = 2026-10-01.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, already queued).
