# PPT Project State

_Generated: 2026-10-17 — routine Phase 1.6 upkeep (pm-integration rotation slot; pm-cursor idx 7 → 0 wrap) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 9 → 10 (one epic re-checked, no material change). 10h lag — on schedule._

## Delta since last routine run (2026-10-16)

- **0 PRs merged** since 2026-10-16T10:27Z. All 8 commits on `dev` are `.research/*` dispatcher + routine bookkeeping (no application-code churn).
- **1 new issue:** #3109 "Follow-up: implement api-server DELETE /community/groups/{id}" (labels: follow-up, from-merged-review). Already-labeled, routed via post-merge review of PR #3101.
- **4 new code-review-finding items** added to backlog (dispatcher Tier-1d today):
  - `code-review-api-core-totp-rng-expect-panic` (score 1, confidence medium) — totp.rs:153 .expect panic on CSPRNG failure contradicts surrounding error-handling contract on MFA path.
  - `code-review-reality-web-realtor-id-shared-mock` (score 2, confidence high) — public SEO-indexed `/[locale]/realtor/[id]` renders MOCK_AGENT on every URL.
  - `code-review-reality-web-realtor-id-dead-contact-form` (score 2, confidence high) — contact form drops user input (no network call).
  - `code-review-reality-web-realtor-id-no-i18n` (score 1, confidence high) — hard-coded Slovak on locale route.
- **1 item decayed+dropped:** `code-review-reality-server-add-to-compare-toctou-max-cap` (score 1 → 0 after 14 days without reinforcement; evidence still intact in compare.rs — re-detectable).
- **pm-integration verified facts (static analysis):**
  - Reality-server **already** mounts public `GET /api/v1/realtors/{user_id}/profile` — so the realtor-id defect is a frontend wiring + ID-semantics gap, not a missing endpoint.
  - api-server `community.rs` mounts `GET /groups/{id}` but **no DELETE/PUT/PATCH** — explains issue #3109.
- **Human-approval bottleneck persists:** PR #3072 (`code-review-api-handlers-violations-missing-role-authz`, score 3) still awaiting human reviewer. The research routine cannot resolve it.

<sub>Prior upkeep note (archived): 2026-10-16 — pm-data rotation; coverage_cursor 8 → 9.</sub>

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** Sprint-status.yaml still shows some epics as in-progress/partial (6, 7A, 10B, 80) — sprint-level drift against coverage.json truth; no new regressions, deep scan would reconcile it. **Sprint header stale** (goal says "Epic 6, 7A, 8A & 10A"), epic-6/epic-7a counters mis-match story-level `done` counts, duplicate 80-2 entry.
- **Buffer-low alarm continues:** `claimable=6/72` (floor 36). Deficit confirmed as human-approval throughput, not supply (per yesterday's re-fire self-diagnosis and the Scrum Master's reading today).
- **No new blockers.** Standing infra blockers (#2966 swagger-ui egress, #2951 jest-expo, #2946 cross-tenant IDOR retry, #2652 KMP cloud builds) carry forward.
- **New integration-surface signal:** PR #3101 → issue #3109 is the second instance in a week of the "frontend wires a mutation whose backend endpoint doesn't yet exist" anti-pattern. pm-integration recommends a merge-prerequisite rule + CI route-drift check.

## What's next (top 5)

1. **Get a human reviewer on PR #3072** (code-review-api-handlers-violations-missing-role-authz). Blocked on reviewer slot, not code. _owner: human reviewer / escalate via pm-security._
2. **Implement `DELETE /api/v1/community/groups/{id}` on api-server** (issue #3109) + regenerate api-client so ppt-web PR #3101 delete works end-to-end. _owner: pm-backend._
3. **Wire reality-web `/[locale]/realtor/[id]` page to existing `GET /realtors/{user_id}/profile`** + add 404 state + thread next-intl + remove MOCK_AGENT. _owner: pm-frontend._
4. **Cross-service contract-drift CI check** — diff frontend-called paths against axum routes + OpenAPI; add to `api-validation.yml`. Prevents the next #3101→#3109 class. _owner: pm-qa._
5. **Reconcile sprint-status.yaml** — rename or close the stale sprint; fix epic-6/epic-7a counter drift; remove duplicate 80-2 entry; decide epic-80 status. _owner: pm-scrum-master._

## Blockers

- **PR #3072 awaiting human reviewer** — sole gate on `code-review-api-handlers-violations-missing-role-authz` (score 3, open). ~10h without application-code movement on `dev`.
- **Issue #3109 waiting on backend `DELETE /community/groups/{id}`** — ppt-web frontend wired but endpoint doesn't exist; shipped UI currently 404s/405s against real API.

## Role focus today

- **pm-integration** (full rotation deep-dive after 3-month gap): 6 next_actions, 5 risks, 5 open questions, 4 decisions needed — see `roles/pm-integration.md`.
- **pm-scrum-master** (always-on): sprint-progress reconciliation, PR #3072 escalation, #3109 implementation plan.

## Standing infra blockers (carry-forward)

- #2966 utoipa-swagger-ui egress 403 — api-server cloud-build-blocked.
- #2652 KMP mobile-native cloud builds — AGP egress-blocked (`gh-issue-2652`).
- #2951 jest-expo version rot — mobile RN test infra.
- #2946 cross-tenant IDOR retry 2/2 — portfolio_analytics; blocked by #2966.
