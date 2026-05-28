# Role: pm-scrum-master — 2026-05-28

> Delivery lead / coordinator. Always runs. Static read-only.

**Summary:** Quiet-but-productive window — 5 PRs merged (#635–#638, #642), all from-merged-review / gap delivery, advancing four coverage stories to fresh `done` evidence (7a-2, 10b-5, 10b-7, 79-2 security leg) and closing issue #580. The most material outcome is PR #642 reconciling the #617 cookie-Path regression with tests, clearing the last security blocker on story 79-2; only an e2e verification remains there. Backend authz follow-ups (#614/#624) still gate Epic 81, and mobile remains the most-behind platform (10 of 22 candidates).

## Shipped since last run

- **#635** — admin-web Support Data page (SupportDataPage.tsx + FaultByStatusTable); 10b-5 frontend surface now exists (+395/-0).
- **#636** — 7 backend integration test cases for document folders (document_folder_tests.rs); CLOSES issue #580 (+331/-98); 7a-2 web slice now tested.
- **#637** — HelpSidebar a11y (focus-trap, dialog role, tooltips) across 11 admin-web pages + HelpSidebar.test.tsx; 10b-7 contextual-help surface (+490/-51).
- **#638** — mobile EAS iOS CI config: removed submit.staging, guarded updates.url against missing EXPO_PROJECT_ID.
- **#642** — auth.rs + sso.rs cookie-Path reconciliation + runbook, with inline tests; resolves the #617 cookie-Path regression class (+370/-0).

## Sprint progress

- Sprint: "Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"
- Epics done: 2 / 13 (epic-8a, epic-9 fully done; coverage 27 done / 21 partial / 1 not-started across 49 stories).

## Next actions

| Action | Priority | Dependency | Definition of done |
|---|---|---|---|
| Close report-schedule authz holes #614 (missing RBAC) + #624 (missing tenant/org scope) together; cross-tenant regression test. | high | pm-security/pm-backend | Both issues closed; Epic 81 unblocked for promotion. |
| Implement/confirm POST `/api/v1/documents/upload` + 81 backend pause/resume + executions-download routes. | high | pm-backend | Routes return non-404; 7a-1/81-1/81-2 promotable. |
| Schedule gap-79-2-auth-callback-e2e now that #642 reconciled cookie-Path. | medium | pm-qa | e2e covers /auth/callback token store + refresh + logout; 79-2 promotes done. |
| Triage remaining from-merged-review follow-ups (fix-569/573/574/581/583) into the dispatcher buffer with owners. | medium | none | Each item owned; close-rate confirmed ≥ ingest-rate. |
| Land 6-2/6-3/6-4 announcement web UI out of draft in #474→#475→#479 order. | medium | pm-frontend | Three drafts merged without AnnouncementsPage conflict; apiStatus verified. |

## Risks

| Risk | Prob | Impact | Mitigation |
|---|---|---|---|
| Reports authz #614/#624 cross-tenant report-schedule IDOR gates Epic 81 promotion. | high | high | RequireCapability extractor + principal tenant/org in WHERE; regression test; block 81 until closed. |
| 9 open PRs all draft + unreviewed (#597,#632,#633,#639–#641,#643–#645) — review-queue backlog can stall the merge train. | medium | medium | Walk the draft queue; promote ready ones, request review, drop stale. |

## Open questions

- Are any of the 9 open drafts ready to promote, or all still in active implementation?

## Decisions needed

- (carried) Merge sequence for the Epic 6 announcement web UI drafts (#474→#475→#479) — owner: pm-frontend / pm-scrum-master.

## Blockers

- Reports authz (#614 + #624) — `update_schedule` cross-tenant + missing-RBAC; Epic 81 cannot promote. Owner: pm-backend.
- 7a-1 / 81 backend endpoints — upload + pause/resume/executions-download routes still to confirm. Owner: pm-backend.
