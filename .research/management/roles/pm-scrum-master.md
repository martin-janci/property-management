# pm-scrum-master — 2026-07-06

_Always-on delivery synthesis. Static read; no compile/run._

## Summary

This run's 18 merged PRs (#2094-#2120) were all backlog/hardening/security-guard fixes with zero sprint-story movement; sprint stories remain where they were on 2026-07-04 (7a-3/7a-4 done, 7a-2 stuck in review on red CI, 7a-5/10a-1/2/3 still gated by the open 2026-05-25 test-hardening batch).

## Sprint progress

**Sprint:** Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth · **epics_done:** 1/6.

## next_actions

- **[high]** Fix CI (`document_folder_tests` FK/isolation) blocking 7a-2-folder-organization. DoD: PR #1316 CI green, story flips review→done. dependency: rust-backend.
- **[high]** Un-draft and land #1797 (auth on OCR endpoints + manager-gate rental guest PII reads) — open 12+ days. DoD: PR merged to dev, PII/authz gap closed. dependency: rust-backend/security.
- **[high]** Close or explicitly defer test-hardening batch thb-2026-05-25 items #481/#482/#487 gating 10a-1/2/3. DoD: story_gate cleared or notes updated with deferral rationale. dependency: rust-backend, react-web.
- **[medium]** Resolve #485 (document share panel `window.confirm` + no UUID validation) to unblock 7a-5-document-sharing. DoD: issue closed, 7a-5 movable to in-progress. dependency: react-web.
- **[medium]** Reconcile stale `epics.stories_completed` counters in sprint-status.yaml for epic-6 (says 3/6, detail shows 6/6 done), epic-7a (says 2, detail shows 3), epic-10b (says 7/7 but header still in-progress). DoD: epic header status/counts match development_status block. dependency: none.
- **[medium]** Start 10a-1-oauth-authorization-server (ready-for-dev, foundational for 10a-2/10a-3). DoD: story moved to in-progress with an owner. dependency: rust-backend.

## risks

- Draft PR #1797 (OCR auth + PII read gate) has sat open since 2026-06-23 with no forward motion. probability=high impact=high. Mitigation: Prioritize as security fix, assign owner this week.
- sprint-status.yaml epic-level counters are stale vs. story-level detail, risking mis-reported sprint progress to stakeholders. probability=high impact=medium. Mitigation: Reconcile counters in next status update.
- Old test-hardening batch (#480-487, distinct from the newer closed #2082-2110 batch) still has 6 open items gating three epics. probability=medium impact=medium. Mitigation: Triage batch weekly; close or formally defer each item.
- coverage.json (generated 2026-07-02) is now stale relative to sprint-status.yaml (7a-3/7a-4 verified done 2026-07-04) — gap-driven ranking may double-count already-closed gaps. probability=medium impact=low. Mitigation: Re-run coverage scan before next planning cycle.

## Shipped since last run (18 PRs)

- #2120 gh-issue-2107: authorize outage mutations on DB role, not JWT claim
- #2119 gh-issue-2110: restore executable bit on check-ignore-reason.sh
- #2118 code-review-mobile-rn-screens-mock-data: wire mobile Meters/Leases/Forms/Threads to api-server
- #2117 gh-issue-2103: named-field constructor for SlovakAccountingExport
- #2116 gh-issue-2109: cover include_system=true pagination tie-break
- #2115 gh-issue-2108: fix ListingDetail auth-change spinner reset
- #2114 gh-issue-2102: wire MCP-push size guard into dispatcher Phase 6
- #2113/#2098 enum-sync guard for currency/country lists
- #2112 refresh stale BIT-351 quarantine docstrings
- #2111 gate backend/deny.toml with code-owner review
- #2100 split rental repository into sub-modules (churn-hotspot refactor)
- #2101 triage-dispatcher-mcp-push-large-file-issue-1014
- #2099/#2086 SlovakAccountingExport honesty invariant by construction
- #2097/#2082 un-quarantine #1771 soft-delete unread invariant
- #2096/#2084 enforce quick-xml XXE pin via cargo-deny
- #2095/#2085 assign_fault recipient guard extraction+coverage
- #2094/#2087 shared seed_org fixture reuse

All 15 follow-up issues from this window's post-merge review batch (#2082-#2110) are CLOSED — no residual technical debt from this batch.

## Blockers

- **7a-2-folder-organization** — CI red (`document_folder_tests` FK/isolation), reverted from done. owner_role: rust-backend.
- **7a-5-document-sharing** — gated by open issue #485 (`window.confirm` + no UUID validation). owner_role: react-web.
- **10a-1/10a-2/10a-3 (OAuth Provider Foundation)** — gated by open test-hardening items #481, #482, #487. owner_role: rust-backend / react-web.
- **#1797 (auth on OCR endpoints + PII guard)** — draft PR open 12+ days, no movement. owner_role: rust-backend.

## open_questions

- Is epic-6 now fully done (all 6 stories show done) — should the epic header be flipped from in-progress to done?
- Is epic-10b fully done (7/7 stories) — should its header move from in-progress to done?
- What is blocking #1797 and #1812 from leaving draft state — reviewer availability or unresolved design questions?
- Should the 2026-05-25 test-hardening batch (#480-#487) be re-triaged now that a newer, fully-closed follow-up batch (#2082-#2110) exists — is the old batch still tracked/owned?

## decisions_needed

- Reconcile sprint-status.yaml epic-level status/counts vs. story-level detail — owner: pm-scrum-master
- Decide whether #1797 (security/PII) should be escalated ahead of routine backlog work — owner: pm-tech-lead / pm-security
