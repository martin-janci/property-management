# pm-scrum-master

_Generated: 2026-10-31 — daily routine Phase 1.6 always-on_

## Summary

Sprint spine (Epic 6/7A/8A/10A) is fully done and stale as a planning anchor; the last 64h were a code-review/bug-fix sweep (6 PRs merged, all needs-human-review) with no story status changes. Remaining work is review-queue drain and a stalled cluster of violations PRs, and the two ready plans are mobile-native-kmp (cloud-unlandable).

## Sprint progress

- Sprint: Epic 6, 7A, 8A & 10A - Announcements, Documents, Notifications & OAuth
- Epics done: 4 / 4

## Shipped since last run

- #3185 ppt-web useNeighbors/usePrivacySettings stale-token useMemo fix
- #3186 ppt-web IoT WS hasGivenUp dropped fix
- #3188 fix cross-org IDOR in assign_violation (403->404), closes #3187
- #3189 mobile VoteDetailScreen ranked-vote order fix
- #3190 ppt-web authed-roots session-leak queryKeys regression fix
- #3191 ppt-web AccountingInvoiceForm qty/price NaN guard

## Blockers

- backlog: code-review-mobile-native-kmp-inquiries-response-contract — mobile-native-kmp cannot land from cloud (AGP egress block, gh issue #2652) (owner: pm-tech-lead)
- backlog: code-review-mobile-native-kmp-create-listing-not-wired — Same AGP egress block (#2652) (owner: pm-tech-lead)
- PR #3075 / #3171 (violations.rs cluster) — Overlapping edits to violations route/repository (5 churn each in 14d); #3075 draft 15d, #3171 iterating; rebase/conflict risk (owner: pm-backend)
- PR #3119 NewsScreen i18n — 13d open, non-draft, no reviewDecision (owner: pm-frontend)

## Next actions

- [high] Human review pass on the 6 merged needs-human-review PRs, prioritising #3188 (security IDOR contract change) and #3190 (session leak) — dependency: human reviewer — DoD: Each PR reviewed and label cleared or follow-up issue filed
- [high] Land #3171 (add_evidence manager role gate), then rebase and finish #3075 (violations number-count race) sequentially — dependency: pm-backend — DoD: Both merged to dev without conflicts; CI green
- [high] Triage security-flavoured drafts #3192 (appeal no status guard), #3193 (moderation report flood), #3198 (os-rng expect panics): mark ready and request review — dependency: pm-security — DoD: Each draft ready-for-review or closed with rationale
- [medium] Resolve #3108 (owner-analytics portfolio-summary 500 schema drift): fix migration/query mismatch and un-draft — dependency: pm-backend — DoD: Endpoint returns 200 in tests; PR merged
- [medium] Get reviewer decision on #3119 and un-draft/finish #3197 (reality-web listing-edit price-zero) — dependency: pm-frontend — DoD: Both have reviewDecision and are merged or closed
- [low] Investigate churn hotspots (reality-web agency/[slug]/page.tsx, deny.toml, migration.rs) for instability and run a coverage scan to refresh planning — dependency: pm-tech-lead — DoD: Root cause noted; coverage.json refreshed

## Risks

- [medium / high] Six PRs merged in a 16-minute window all with needs-human-review; review debt may let regressions (esp. the IDOR 403->404 contract change) go unvalidated
  - Mitigation: Schedule explicit human review; check clients depending on the 403 status
- [high / medium] Violations files churn heavily with multiple concurrent PRs (#3171, #3075, #3188 merged), causing conflicts or regressions
  - Mitigation: Serialize merges; rebase #3075 after #3171
- [high / medium] Only ready backlog plans are mobile-native-kmp and unlandable in cloud, so improvement track has no executable ready items
  - Mitigation: Promote cloud-landable backlog items to ready; solve or route around #2652
- [medium / low] Stale drafts (#3075 15d, #3108 13d, #3119 13d) rot and drift from dev
  - Mitigation: Set a 14d staleness rule: rebase, finish, or close
- [high / low] sprint-status.yaml sprint goal is stale (all epics done since 2025/2026), so it no longer guides prioritisation
  - Mitigation: Define the next sprint from coverage.json gaps

## Open questions

- Is a new sprint defined, given all four sprint epics are done and the YAML sprint name dates from 2025-12-21?
- Do any clients (web, mobile, KMP) depend on the previous 403 from assign_violation after #3188 changed it to 404?
- Is there a plan or owner to resolve the AGP egress block (#2652) so mobile-native-kmp items can land?
- Why did six PRs merge within 16 minutes while all carrying needs-human-review: batch auto-merge, or pending post-merge review?
- What is the intended disposition of the 7 dependabot PRs updated by the weekly sweep, and is deny.toml churn (5 in 14d) related?

## Decisions needed

- Approve/define next sprint scope from coverage.json partial/not-started stories — owner: product owner
- Decide whether to merge #3171 before #3075 or consolidate into one violations PR — owner: pm-backend
- Decide on mobile-native-kmp strategy (local runner vs fix egress allowlist) — owner: pm-tech-lead
