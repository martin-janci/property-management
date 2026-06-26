# pm-scrum-master — 2026-06-26

## Summary

A large reconciliation wave on 2026-06-25 closed all 5 open epic-6 stories (6-1 through 6-5), epic-79 (79-1 API client, 79-2 auth), epic-10b stories 10b-5 and 10b-7, and advanced epic-16 (BIT-139/140) and messaging (BIT-206/244). Coverage now sits at 46/49 done with epic-10a (OAuth) the last in-progress sprint epic — all 3 stories still gated by open test-hardening issues (#481, #482, #487). Epic-80 has 80-3 partial (draft PR #1846 open).

## Sprint progress

- **Sprint:** Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth
- **Epics done:** 4 / 6 (epic-6 full, epic-7a full, epic-8a, epic-10b full)
- **Open:** epic-10a 0/3, epic-80 1 partial (80-3)

## Next actions

1. **[high]** Resolve test-hardening blockers #481 (OAuth refresh-token revocation) and #487 (MFA rate-limit) to ungate 10a-1 + 10a-3 — owner: pm-backend
2. **[high]** Resolve #482 (ProtectedRoute multi-tenant role fallback + missing tests) to ungate 10a-2 — owner: pm-frontend
3. **[high]** Land or close draft PR #1846 (80-3 mediation party submissions) — owner: pm-frontend
4. **[high]** Triage 2026-06-25 follow-up issues #1851/#1852/#1828/#1827/#1826/#1793/#1792 — owner: pm-scrum-master
5. **[high]** Review/merge/reject 8 open security-hardening drafts (#1797/#1799/#1801/#1802/#1804/#1806/#1807/#1823) — owner: pm-security
6. **[medium]** Resolve test-hardening #480 (WS JWT in logs) and #484 (notification dispatch + FCM stub) — owner: pm-backend

## Risks

- **Security drafts wave stalled** (8 PRs, IDOR + PII class) — probability medium, impact high — owner: pm-security
- **epic-10a fully gated** by 3 stale issues — probability medium, impact high — owner: pm-backend / pm-frontend
- **coverage.json staleness window** (deep scan 2026-06-23 + upkeep today) — probability high, impact medium — owner: pm-scrum-master
- **PR #1821 ACC accounting-server MVP** scope-creep risk — probability low, impact medium — owner: product owner

## Open questions

- Are all 8 open security drafts confirmed-vulnerability fixes vs speculative hardening?
- Is dev backend compile (issue #1437) resolved post the 2026-06-23..26 wave?
- Is PR #1821 (ACC MVP) in-scope for current sprint or a new track?
- 80-2 redesign wizard — confirmed shipped or only single-page form reconciled?
- Should epic-10a be formally declared at risk for this sprint?

## Decisions needed

- Close-or-defer for issues #480, #481, #482, #484, #487 (each >30 days open) — owner: pm-scrum-master + pm-backend/pm-frontend
- PR #1821 in-scope or deferred to new sprint track — owner: product owner
- Confirm dev backend compile state + #1437 resolution — owner: pm-devops
- Promote or close draft PR #1846 (80-3) this sprint — owner: pm-frontend

## Shipped since last run (29 PRs)

Reconciliation: #1832 (6-1), #1834 (6-2), #1835 (6-3), #1843 (6-4), #1844 (6-5), #1845 (80-2), #1829 (10b-5), #1831 (10b-7), #1830 (79-1), #1822 (79-2). Feature: #1847 (16.3 BIT-140), #1849 (BIT-139 drainer), #1850 (BIT-242 favorite worker), #1848 (BIT-206 N-party), #1853 (BIT-244 web). Churn refactors: #1816, #1815, #1813, #1810, #1800, #1798, #1796, #1781, #1683. Infra: #1809, #1808, #1755, #1779, #1752.
