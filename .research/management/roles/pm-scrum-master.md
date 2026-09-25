# pm-scrum-master — 2026-09-25

## Summary

Zero application PRs merged since the last routine run (only `.research/` heartbeat commits) — delivery is stalled on human review, not on cloud work capacity. 7 PRs sit on `needs-human-review` (2 new: security IDOR retries #2977/#2976), and the remaining claimable backlog is cloud-unlandable (egress-blocked build issues), so the sprint's actual bottleneck this cycle is reviewer throughput.

## Sprint progress

- Sprint: **Epic 6, 7A, 8A & 10A - Announcements, Documents, Notifications & OAuth**
- Epics: **2 done / 6 total** (sprint-status.yaml rollup — see reconciliation ask below).

## Shipped since last run

- _nothing_ — 0 PRs merged in the 31h window; all 20 commits on `dev` are `.research/` dispatcher heartbeats.

## Next actions (top 3)

1. **[high]** Human-review the two new IDOR retry PRs — #2977 (violations comments/evidence/payments IDOR, gh-issue-2944) and #2976 (portfolio-property IDOR, gh-issue-2945) — ahead of the other 5 queued PRs. Owner: human reviewer / tech-lead. DoD: both PRs merged or sent back with actionable review comments.
2. **[high]** Clear the `needs-human-review` backlog (#2969, #2968, #2967, #2902, #2744) — some have been parked across multiple runs. Owner: human reviewer. DoD: each PR merged, closed, or explicitly re-queued with a note.
3. **[medium]** Unblock issue #2966 (`utoipa-swagger-ui` build script egress-blocked) outside the cloud sandbox — it gates the api-server verify pipeline. Owner: infra / tech-lead. DoD: vendored dep or mirrored crate resolves the build without proxy egress.

## Risks (top 3)

1. **[high · high]** Security-relevant IDOR fixes (#2976, #2977) sit unmerged in the review queue. Mitigation: fast-track ahead of dependabot PRs.
2. **[high · medium]** `needs-human-review` queue depth (7 PRs) keeps growing faster than it drains, stalling visible delivery run-over-run. Mitigation: dedicated reviewer slot per run to clear 2–3 PRs from the queue.
3. **[medium · medium]** Cloud claimable buffer is starved (8/72) with the remainder blocked by egress/version-rot infra issues — future runs will repeat this zero-merge pattern. Mitigation: escalate #2966/#2951 to a human with non-cloud build access.

## Open questions

- Who owns clearing the 7-PR `needs-human-review` queue and on what cadence?
- Is there a plan/owner to resolve #2966 and #2951 outside the cloud sandbox (both require non-proxy egress or local dependency resolution)?
- Should sprint-status.yaml's epics block (epic-6/7a/10b/80 marked in-progress/partial) be reconciled now, given every constituent story shows development_status:done?
- coverage.json (generated 2026-08-31) shows all 49 scanned stories done with no partial/not-started gaps — is a fresh `scan` due, or is that genuinely current?

## Decisions needed

- Set review priority order for the 7 `needs-human-review` PRs — security IDOR retries first. Owner: tech-lead.
- Decide remediation path (vendor/mirror vs. wait) for egress-blocked build issues #2966 and historically #2652. Owner: pm-tech-lead / infra.
- Confirm whether sprint-status.yaml epic-level rollups should be reconciled to `done` now or left pending a broader story audit. Owner: pm-scrum-master.

## Blockers

- **#2976 / #2977 (IDOR security retries)** — `needs-human-review`; cloud dispatcher cannot self-merge security-sensitive fixes. Owner: human reviewer / tech-lead.
- **#2969 / #2968 / #2967 / #2902 / #2744** — remaining `needs-human-review` PRs parked across multiple runs. Owner: human reviewer.
- **issue #2966** — utoipa-swagger-ui build-script egress in cloud runner blocks api-server verify. Owner: infra / rust-backend.
- **issue #2951** — jest-expo 56 vs RN 0.87 rot breaks the mobile RN jest suite. Owner: mobile/react-native.
- **mobile-native KMP ready plans** (inquiries-response-contract, create-listing-not-wired) — cloud-unlandable per AGP/egress constraint (closed #2652 still governs). Owner: mobile-native / kotlin.
