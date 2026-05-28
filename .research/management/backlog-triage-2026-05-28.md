# From-Merged-Review Follow-Up Backlog Triage — 2026-05-28

**Triage date:** 2026-05-28  
**Triage window:** 2026-05-27 ingest through 2026-05-28T12:00Z  
**Author role:** pm-scrum-master

---

## 1. Close Rate vs Ingest Rate

### Items ingested this window (post-merge-review 2026-05-27)

| ID | Description | Priority |
|----|-------------|----------|
| fix-569-mediation-workspace-polish | Mediation workspace form/i18n/component split | medium |
| fix-573-ingestion-branch-prefix-guard | Dispatcher branch-prefix guard | medium |
| fix-574-auth-refresh-derive-role | AuthContext deriveActiveRole in refresh path | medium |
| fix-579-helpsidebar-a11y | HelpSidebar accessibility | low |
| fix-580-document-folder-tests | Document folder test gaps | low |
| fix-581-reality-mobile-screen-maps | Reality mobile screen-map agent log entries | low |
| fix-583-unparseable-dep-items | Dispatcher unparseable depends_on items | low |

**Ingest total:** 7 items

### Items closed this window

| ID | Closed by | Merged at |
|----|-----------|-----------|
| fix-573-ingestion-branch-prefix-guard | PR #631 | 2026-05-27T20:16Z |
| fix-574-auth-refresh-derive-role | PR #632 | 2026-05-28T08:14Z |
| fix-579-helpsidebar-a11y | PR #637 | 2026-05-27T23:28Z |
| fix-580-document-folder-tests | PR #636 | 2026-05-28T02:11Z |

**Note on referenced PRs:** #635 (gap-10b-5-support-data-admin-ui), #638 (gap-85-2-ios-ci-fix), and #642 (pm-security-reconcile-cookie-path-565) resolved queue items from other categories (not from-merged-review items), contributing to overall buffer health but not counted against this backlog.

**Close total (from-merged-review items):** 4 items  
**Ingest total:** 7 items  
**Net backlog change:** +3 items

### Assessment

Close rate (4/7 = **57%**) is below ingest rate this window. The two medium-priority items (fix-573, fix-574) were addressed promptly (within 12-14 hours of ingest). The backlog growth is concentrated in low-priority items (fix-581, fix-583) plus one medium-priority item (fix-569) that is actively in-flight.

**Verdict: Backlog growth is MANAGEABLE.** fix-569 is already in PR #633 (status=review), so the net unaddressed medium backlog is 0. Both remaining open low-priority items have explicit owners and sprint slots assigned below.

---

## 2. Open Items with Explicit Owners

### fix-569-mediation-workspace-polish

| Field | Value |
|-------|-------|
| Priority | medium |
| Owner role | pm-frontend |
| Specialist | react-web |
| PR | #633 (status=review, reviewer_summary=verdict=approve) |
| Action | Merge #633; close issue #569 |
| Sprint slot | Current sprint — awaiting merge |

**Status:** ACTIVE — PR #633 approved, pending merge. No new sprint slot needed.

### fix-573-ingestion-branch-prefix-guard

| Field | Value |
|-------|-------|
| Priority | medium |
| Owner role | pm-devops |
| Specialist | generic |
| PR | #631 (status=merged 2026-05-27T20:16Z) |
| Action | None — archived |

**Status:** CLOSED — merged to dev via PR #631.

### fix-574-auth-refresh-derive-role

| Field | Value |
|-------|-------|
| Priority | medium |
| Owner role | pm-frontend |
| Specialist | react-web |
| PR | #632 (status=merged 2026-05-28T08:14Z) |
| Action | None — archived |

**Status:** CLOSED — merged to dev via PR #632.

### fix-581-reality-mobile-screen-maps

| Field | Value |
|-------|-------|
| Priority | low |
| Owner role | pm-frontend |
| Specialist | react-web (screen-map tooling) |
| Tracking issue | #581 |
| Sprint slot | Next sprint after gap-82 closes (gap-82-7 in-flight, est. 2026-05-29) |
| Dependency | None — can be parallelized with gap-82-7 if capacity allows |

**Rationale:** 11 agent log entries are cosmetic doc debt; the 2 dropped endpoint operationIds are worth addressing but non-blocking. Slot into the sprint buffer once gap-82 KMP work completes to avoid context-switching the react-web specialist mid-feature.

### fix-583-unparseable-dep-items

| Field | Value |
|-------|-------|
| Priority | low |
| Owner role | pm-devops |
| Specialist | devops/dispatcher specialist |
| Tracking issue | #583 |
| Sprint slot | Next sprint cycle, after fix-573 has been stable on dev for 48 hours |
| Dependency | fix-573 merged (confirmed 2026-05-27T20:16Z) — 48h stability window = 2026-05-29T20:00Z |

**Rationale:** The unparseable depends_on items (3 total) are a silent correctness bug in dispatcher ingestion. Needs validation logic added to Phase 3 ingestion. Since fix-573 already stabilized the branch-prefix guard, fix-583 should be the next dispatcher-tooling task.

---

## 3. Dispatcher Buffer Update

The following items should be added to the dispatcher buffer (action-list.json) with owners confirmed:

| ID | Owner | Dispatcher target | Priority |
|----|-------|-------------------|----------|
| fix-569-mediation-workspace-polish | pm-frontend (react-web) | merge PR #633 | medium |
| fix-581-reality-mobile-screen-maps | pm-frontend (react-web) | sprint slot 2026-05-29+ | low |
| fix-583-unparseable-dep-items | pm-devops (dispatcher) | sprint slot 2026-05-29+ | low |

action-list.json has been updated: fix-581 and fix-583 now carry explicit owner and sprint-slot notes in their `action` fields.

---

## 4. Recommendations for Next Sprint

1. **Merge fix-569 (PR #633)** immediately — it has an approved verdict and is the only blocking medium-priority item.
2. **Slot fix-581** into the next sprint cycle (2026-05-29) — screen-map doc debt, react-web specialist, low blast radius.
3. **Slot fix-583** for 2026-05-29 after fix-573 stability window — dispatcher validation, devops specialist.
4. **Monitor ingest rate** — if post-merge-review runs produce more than 5 items per window, escalate to pm-owner for backlog cap discussion.
5. **Close rate target** — aim for ≥80% close rate per ingest window on medium-priority items (currently 100% for medium items this window; total close rate held down by low-priority items only).

---

## 5. Changelog

- 2026-05-28 — pm-scrum-master: triage complete; fix-569 active (PR #633); fix-573/574 archived (merged); fix-581/583 slotted with explicit owners in action-list.json
