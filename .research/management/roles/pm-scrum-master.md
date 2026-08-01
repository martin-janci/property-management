# Role: pm-scrum-master — 2026-08-01

> Delivery lead / coordinator. Always runs. Static read-only.

## Summary

5 PRs merged in the 2026-07-31 -> 2026-08-01 window, all post-merge hygiene resolving in-progress items. Composition unchanged at 47/49 done + 2 partial; the two `partial` frontend stories (84-1, 84-2) are the only thing between the project and 49/49. Accounting MVP-loop trio (#2555 / #2558 / #2559) reviewer-starvation extended from 2 days to 4 days — DEC-107 (reviewer-slot policy, raised 2026-07-30) is now overdue for resolution.

## shipped_since_last_run

- **#2613** — refactor(api-server): extract scheduler vote-lifecycle jobs to submodule
- **#2614** — test(api-server): integration test for outbound layout webhook replay/timestamp (closes gh-issue-2485)
- **#2615** — chore(api-validation): harden openapi-ts drift-gate generator steps
- **#2616** — dx-fixme(admin-web): consolidate admin-web inline primitives onto @ppt/ui-kit
- **#2618** — test-gap(reality-web): reconcile reality-web screen-maps with PR #2600 inline-script hardening

## sprint_progress

- sprint: "Epic 6, 7A, 8A & 10A - Announcements, Documents, Notifications & OAuth"
- epics_done: 3
- epics_total: 5

## next_actions

1. **Split 84-2 signer sign page into 3 mergeable slices** — retry3 single-squash pattern has failed; go incremental (route/manifest -> capture -> verify+delivery). Priority: **high**. Dependency: pm-frontend. DoD: three merged PRs, 84-2 flips to done.
2. **Escalate gh-issue-2573 to a named pm-backend owner** — 3 days stalled with zero churn; it's the blocker for 84-1 which is the last mvp partial. Priority: **high**. Dependency: pm-backend. DoD: #2573 has an owner and a claim event within 48h.
3. **Resolve DEC-107 reviewer-slot policy for the accounting trio** — now 4 days without reviewer engagement; either assign a named reviewer or split the trio. Priority: **high**. Dependency: pm-tech-lead. DoD: trio has reviewer engagement or is re-scoped as multiple smaller PRs.
4. **Triage new open follow-up issue #2612 (fire-once notifications)** — decide owner_role (pm-backend vs pm-mobile), scope, whether it blocks 8A extended-scope closure. Priority: **medium**. Dependency: pm-tech-lead. DoD: #2612 labeled + owner-role assigned + action-list slot updated.
5. **Extend the #2616 ui-kit consolidation pattern to ppt-web** — leverage this window's admin-web win to sweep ppt-web's duplicated primitives. Priority: **medium**. Dependency: pm-frontend. DoD: one PR merged addressing top-10 duplications.

## risks

1. **Accounting trio drift compounds daily** — 4 days without reviewer, will start conflicting with dev churn (auth.rs, integrations/booking split, layout webhook test) — probability high, impact medium. mitigation: DEC-107 must resolve this window.
2. **84-1 blocked chain (frontend blocked on backend blocked on nothing)** — coordination gap, not implementer capacity — probability medium, impact high. mitigation: explicit hand-off of #2573 this window.
3. **84-2 retry pattern will burn a 4th slot** without a slice-split — probability high, impact medium. mitigation: pm-frontend #1 above.
4. **Fresh-open buffer at 10/36** — the dispatcher may starve for genuinely-open work in the next 6-12 hours unless a `scan` refresh is triggered — probability medium, impact medium. mitigation: consider `/ppt-project-management scan` on next run.

## open_questions

- Is #2612 (fire-once notifications) a regression of shipped 8A code or a gap surfaced by a new consumer? (Affects owner_role assignment.)
- Should the reviewer-slot policy (DEC-107) apply retroactively to already-starving trio, or only to new large-scope PRs?

## decisions_needed

- **DEC-107 (2026-07-30, reviewer-slot policy) is now overdue** — the accounting trio going from 2 days -> 4 days reviewer-starved is direct evidence the policy is needed. Owner: pm-tech-lead. (raised 2026-07-30; escalated 2026-08-01 by pm-scrum-master.)
- **Slice-vs-squash policy for retry-3+ stories** — see pm-frontend's decisions_needed; supported here — the 84-2 pattern is the second retry-3+ single-squash failure in a month (first was gh-issue-2318). Owner: pm-tech-lead + pm-scrum-master.

## blockers

- **gh-issue-2573** — DELETE /documents/by-file-key same-org reference gap; 3 days unclaimed; blocks 84-1. Owner: pm-backend.
- **Accounting MVP-loop trio (#2555 / #2558 / #2559)** — 4 days without reviewer. Owner: pm-tech-lead.
- **84-2 retry loop** — 3 failed single-squash attempts; pattern itself is the blocker. Owner: pm-frontend.
