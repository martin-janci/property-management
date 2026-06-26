# Role: pm-scrum-master — 2026-06-26

> Delivery lead / coordinator. Always runs. Static read-only.

## Summary

Sprint "Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth" is at roughly 55 % by story count. Epic 6 is now 5/6 done (stories 6-1 + 6-3 verified 2026-06-25), Epic 8A is fully done, Epic 10B is fully done. Epic 7A holds at 1/5 (7a-2 in review with red CI); Epic 10A is 0/3 (gate-blocked by open security issues #481 + #487). The 10-day cursor lag means up to 96 merged PRs — including a 3672-file `emergency.rs` refactor (#1798) and a reverted delegation PR (#1713 reverts #1690 per board decision BIT-213) — require reconciliation against story statuses.

## Shipped since last run (top one-liners)

- #1849 — BIT-139 email/push transport drainer for saved-search alerts (reality-server)
- #1847 — BIT-140 saved-search alert_frequency cadence scheduler (reality-server)
- #1848 — BIT-206 messaging group participants backend
- #1853 — BIT-244 messaging group participants frontend (ppt-web)
- #1846 — BIT-80-3 mediation/dispute party submissions (Epic 80 advance, draft)
- #1833 — BIT-84-5 RAG view contract fix (reality-server)
- #1830/#1831/#1832/#1834/#1835 — Announcements UI iterations (Epic 6)
- #1822 — Epic ACC: accounting standalone foundation
- #1798 — refactor: emergency.rs split into modules (3672 file changes; infra hygiene)
- #1713 — REVERT of #1690 delegation frontend per board decision BIT-213
- #1567/#1568 — Dependabot dependency bumps

## Next actions (top 5)

1. **[high] Fix 7a-2 CI failure** (document_folder_tests FK/isolation) and re-green to move folder-organization from review to done — owner: pm-backend
2. **[high] Close or formally defer security gate issues #481 + #487** to unblock Epic 10A — owner: pm-backend / pm-security
3. **[high] Resolve issue #480** (JWT in WebSocket query param logged) — owner: pm-backend
4. **[medium] Pick up 7a-3-permission-based-access** (ready-for-dev) once 7a-2 lands — owner: pm-backend
5. **[medium] Reconcile sprint-status.yaml epic-6 (5/6) + epic-10b (7/7)** to 2026-06-25 verifications — owner: orchestrator

## Blockers

- **7a-2-folder-organization** — CI red (document_folder_tests FK/isolation); story stuck in review (owner: pm-backend)
- **10a-1 + 10a-3** — open security gates #481, #487 (owner: pm-backend)
- **10a-2** — open security gate #482 ProtectedRoute multi-tenant role fallback (owner: pm-frontend)
- **8a-3 publish leg gates** — issues #480, #484 still open even though sync leg shipped
- **sprint-status.yaml freshness** — story counts do not reflect 2026-06-25 verifications (owner: orchestrator)

## Risks (added to risks.json)

| Risk | P×I | Mitigation |
|---|---|---|
| PR #1798 (3672-file refactor) — high merge-conflict surface | high×medium | All open draft PRs rebase on dev immediately |
| Phase 1.5 panic paths in reality-server (rng/http-client/unwrap) | medium×high | File hardening issues; prioritize before ACC ships |
| #481 reusable revoked refresh tokens — live RFC 9700 violation if prod-exposed | medium×high | Treat as hotfix-eligible; do not ship 10A until patched |
| 10-day cursor lag — sprint-status + coverage may be materially stale | high×medium | Re-run scan locally; refresh sprint-status |
| Epic 7A 4/5 stories not done; cascade dependency 7a-3 not started | medium×medium | Prioritize 7a-2 CI fix; consider deferring 7a-4/7a-5 |

## Open questions

- Is story 6-6 (neighbor-information) truly done? sprint-status marks it done with no verification note.
- Status of Epic ACC (PR #1822 open, #1821 draft)? Is this a new sprint addition or carry-over?
- Board decision BIT-213 reverted delegation frontend — is the feature fully deferred or is a replacement approach planned?
- Are any of the 3 `needs-human-review` issues blocking current sprint stories or all in the follow-up queue?
- Phase 1.5 emitted 3 reality-server hardening findings — have these been filed as issues yet?

## Decisions needed

- Treat issue #481 as hotfix (ship fix to main independently) or hold until Epic 10A starts? — owner: pm-backend + tech-lead
- Add Epic ACC formally to active sprint scope or track separately? — owner: product-owner / pm-scrum-master
- Defer 7a-4/7a-5 to next sprint given 7a-2 still red and 7a-3 not started? — owner: product-owner
- Formal deferral or re-plan for board decision BIT-213 (delegation frontend)? — owner: product-owner / board
