# pm-scrum-master — 2026-10-21

**Routine Phase 1.6 (always-on Scrum Master synthesis)**

## Summary

Four backend PRs (#3125, #3127, #3129, #3130) merged since the last run, but the backend merge pipeline is still blocked. Issue **#3134** (residual transitive `zip 3.0.0` failing cargo-deny after the `rust_xlsxwriter` bump) and issue **#3131** (dispatcher bot cannot self-approve — WIP is 16 of 16 and ~16 approved PRs wait on a human merge) are open. `sprint-status.yaml` is stale: its sprint header is Dec 2025, and epic-6 shows 3/6 completed and epic-7a 2/5 while every story under them is marked `done`.

## Sprint progress

- Sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"**
- Sprint epics_done: **2/6** (sprint-status.yaml — stale)
- Coverage-level: all 13 extended epics are 49/49 done.

## Shipped since last run

- **PR #3125** — bump `rust_xlsxwriter` → `zip` 9.x (RUSTSEC-2026-0367/0368/0369, one path only).
- **PR #3127** — persist migration import upload to storage; fixes silent data loss (regression tests shipped).
- **PR #3129** — typed AML assessments list response with `utoipa` ToSchema.
- **PR #3130** — ammonia 4.2.4 (RUSTSEC-2026-0373).

## Next actions

- **[high · pm-security]** Find which crate pulls `zip 3.0.0` (`cargo tree -i zip`) and bump or patch it, or add a justified `deny.toml` ignore with an expiry → clear issue **#3134**. DoD: `cargo-deny advisories` pass on `dev`; issue closed; backend PRs can merge.
- **[high · human maintainer]** Human merge sweep of the approved dispatcher PRs, oldest first: #3024, #3028, then the violations family #3070/#3072/#3074/#3075 and #3107/#3108/#3111/#3119/#3120/#3123. Decide bot-self-approval vs trusted-author auto-merge (issue #3131). DoD: WIP below `DISPATCHER_PR_MAX=16`; no approved PR >7 days old.
- **[medium · pm-backend]** Land dependabot **#3133** (18 cargo bumps) *after* the cargo-deny fix; confirm it supersedes #2981 so #2981 can be closed.
- **[medium · pm-scrum-master]** Reconcile `sprint-status.yaml` (epic-6 6/6, epic-7a 5/5, epic-10b 7/7, sprint rollover record) and file the price_map DoS follow-up (#3132).

## Risks

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| Merge-pipeline deadlock (#3131) — oldest stuck PR is #3024 (16d), planner cannot claim fresh work | high | high | Time-boxed human merge sweep; decide on a bot-self-approval or auto-merge policy |
| cargo-deny blocks all backend PRs incl. security fixes while transitive zip 3.0.0 remains (#3134) | high | high | Identify the dependent crate and bump it, or add a time-boxed advisory ignore with rationale |
| Merged cache/security work (#3092 price_map) still has a DoS vector (#3132); violations IDOR/authz drafts #3070–#3075 unmerged | medium | high | Prioritise violations security family in the merge sweep; add a cache cap + rate limit follow-up |
| sprint-status.yaml stale → skews planning/reporting | high | low | Reconcile and roll the sprint over |
| mobile-native cloud builds (#2652) stay unclaimable → ~9/72 backlog items reachable | medium | medium | pm-devops to unblock the runner |

## Blockers

- **Issue #3134** — cargo-deny advisories failing (transitive `zip 3.0.0`) — blocks every backend PR merge — owner: pm-backend.
- **Issue #3131** — dispatcher PR deadlock; WIP 16/16; oldest stuck PR #3024 since 2026-10-05 — owner: pm-devops.
- **PRs #3028 / #3070 / #3072 / #3074 / #3075** (security / authz drafts) — approved internally, waiting on human merge AND the cargo-deny fix — owner: pm-security.
- **Issue #2652** — mobile-native cloud builds, backlog items structurally unclaimable — owner: pm-devops.

## Open questions

- Which crate pulls transitive `zip 3.0.0`, and is a bump available or is an ignore with expiry acceptable? (role is read-only — not run)
- Can GitHub rulesets scope a bypass or auto-merge rule to the `auto-impl` label / dispatcher app identity, and does the dispatcher have a second GitHub App or PAT identity available?
- What is the cargo-deny policy for ignore expiry, and who owns triage (pm-security or pm-devops)?
- Should the sprint roll over to a new name/goal now that all 13 coverage epics are 49/49 done?

## Decisions needed

- Merge policy for approved dispatcher PRs: human sweep vs bot self-approval vs raising DISPATCHER_PR_MAX — owner: pm-devops with human maintainer sign-off.
- cargo-deny advisory handling: bump-only vs time-boxed ignore — owner: pm-security / pm-tech-lead.
- Sprint rollover and next sprint goal — owner: pm-scrum-master with the human maintainer.
