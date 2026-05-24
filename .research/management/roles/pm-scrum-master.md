# pm-scrum-master — 2026-05-24

**Summary:** Four PRs merged since the last run (2026-05-23T17:47Z): dead handler modules deleted (#437), Epic 8A formally closed (#436), story 6.1 review pass completed (#440), and gap-7a-4 PDF.js preview shipped (#446). PR #435 (security P0/P1 batch) merged 2026-05-23T22:26Z with deferred findings — issues #438/#439 capture actionable post-merge security/correctness items, and a newly surfaced latent fail-open authorization guard in ProtectedRoute.tsx needs tracking before role-gating goes live.

## Shipped since last run
- **#437** — remove dead AuthHandler/BuildingHandler modules (2495 lines; resolves duplicate-handler divergence risk)
- **#436** — Epic 8A code review marked done; 8a-1/8a-2/8a-3 → done in sprint-status.yaml
- **#440** — story 6.1 announcement creation review pass completed
- **#446** — gap-7a-4: PDF.js client-side document preview (react-pdf) in ppt-web DocumentDetail
- **#435** — security P0/P1 auth/security fixes merged; deferred findings tracked in #438/#439

## Sprint progress
- Sprint: Epic 6, 7A, 8A & 10A (Announcements, Documents, Notifications & OAuth)
- Epics done: 2/5 (8A done 3/3; 10A backend done 3/3 but no admin UI). 6, 7A, 10B still in-progress.
- Coverage scan: 16/49 stories done, 32 partial, 1 not-started.

## Next actions
1. **[high · pm-security]** Resolve post-merge security findings from #438/#439 (P1-05 SSRF, P1-01/P1-04 ordering + Debug-format hash, P0-12 cookie scope, IG3 test gap).
2. **[high · pm-scrum-master]** Review and merge #447 (gap-7a-1 mobile doc upload) + #445 (gap-7a-5 mobile doc sharing) to close the mobile slice of Epic 7A.
3. **[high · pm-frontend]** Advance #443 (gap-7a-3 permission-based doc access API) to ready-for-review; promotes apiStatus stub → integrated.
4. **[high · pm-frontend]** Advance #444 (gap-79-2 login-flow wiring); also harden the fail-open ProtectedRoute.tsx:117 guard before role-gating.
5. **[high · pm-security]** Advance #441 (gap-9-1 MFA frontend integration) to ready-for-review.
6. **[medium · pm-tech-lead]** Approve/merge #442 (DEC-001: sequence Epic 2B before Epic 6 publish + 8A dispatch).

## Blockers
- Epic 2B WebSocket/notification infra — gates 6.2–6.6 (dispatch) and 8a-3 (WS sync).
- Epic 81 stories 81-1/81-2 — frontend calls `/schedules/{id}/pause|resume`, `/executions` that don't exist (404).

## Risks
- PR #435 merged with deferred security findings (#438/#439): P1-05 SSRF unfixed, P0-12 cookie scope, Debug-format hash exposure — high/high.
- Latent fail-open authorization guard in ProtectedRoute.tsx:117 — medium/high; masked today because no route passes requiredRoles and AuthContext doesn't populate role.
- Six open/draft PRs (#441–#447) all opened 2026-05-24 — review-queue depth — medium/medium.

## Decisions needed
- Sequence Epic 2B notification infra before Epic 6 publish + 8A dispatch (or defer) — unblocked by merging PR #442.
- Assign sole fix owner for #438/#439 and set a merge deadline this sprint.
- Decide whether ProtectedRoute.tsx:117 hardening ships inside PR #444 or as a separate security ticket.
