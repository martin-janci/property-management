# Decision log — PPT delivery

> Maintained by `pm-scrum-master`. Append decisions; never delete. Format below.

## Decisions made
_(none yet — first run will populate)_

## Decisions needed
- Whether to pull Epic 2B notification infrastructure into the current sprint to unblock Epic 6 publish + 8A.2 dispatch — owner: pm-scrum-master _(raised 2026-05-23)_
- Build-order: Epic 2B notification/WebSocket infra before vs. after the dependent Epic 6/8A slices — owner: pm-tech-lead _(raised 2026-05-23; DEC-001 / PR #442 in progress)_
- Assign a single owner for PR #435 post-merge findings (#438/#439) and decide whether P0-12 cookie scope + P1-04 Debug-hash ship as a hotfix off dev or batch into the next release — owner: pm-security/pm-tech-lead _(raised 2026-05-24)_
- Whether the ProtectedRoute.tsx:117 fail-open hardening ships inside PR #444 (79-2) or as a separate ticket before role-gating is enabled — owner: pm-frontend/pm-security _(raised 2026-05-24)_
- Whether the 10B stub handlers (10b-4/5/6/7) should return 501 until implemented or remain silent no-ops — owner: pm-tech-lead _(raised 2026-05-24)_

## Decisions resolved
- Delete the dead AuthHandler/BuildingHandler modules (vs. wire them canonical) — **resolved: deleted** in PR #437 (merged 2026-05-23); routes/ are the single source of truth _(raised 2026-05-23, resolved 2026-05-24)_
- Get a review decision on PR #435 — **resolved: merged** 2026-05-23T22:26Z, with deferred findings filed as #438/#439 _(resolved 2026-05-24)_
