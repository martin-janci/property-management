# PPT roadmap — 2026-11-14

_Generated: 2026-11-14T03:17:06Z · scan_kind: upkeep_

## State of the project

- Stories: 49 done · 0 partial · 0 not-started (of 49)
- Sprint delivery (Epic 6, 7A, 8A, 10A + 10B + 80): fully done at coverage level.
- Screen coverage: 0 stories without screen-map · 0 orphan epics · 0 orphan screens · 3 missing UC links (UC-33.1/2/3).
- Top live gaps (from action-list `open`): #3505 ppt-web E2E regression (auth-guard batch fallout), #3427 JWT iss/aud hardening (plan ready, awaits design sign-off), mobile-native/KMP cloud-build starvation, Dependabot majors (vitest 5 / jsonwebtoken 11 / utoipa-swagger-ui 10).

## Ranked plan

### mvp / phase 2

- [high] Land #3507 (gh-issue-3505 disputes E2E auth) and resolve the ppt-web E2E regression tracked by #3505 — owner: pm-scrum-master — why: a sustained E2E red hides real regressions and starves the dispatcher.
- [high] Implement #3427 JWT iss/aud per `plans/security-jwt-iss-aud-cross-service.md` — owner: pm-security — why: cross-service token replay risk; plan is ready, needs design sign-off.
- [high] Ship #3508 (scheduler no-panic supervision, in-progress) — owner: pm-backend — why: crash-risk item sitting in review.
- [medium] Human-review the 3 major Dependabot bumps (#3447 vitest 5, #3444 jsonwebtoken 11, #3443 utoipa-swagger-ui 10) — owner: pm-devops — why: stacked majors are stalling; jsonwebtoken touches the same code as #3427.
- [medium] Finish #3506 (gh-issue-3502 price-map i18n) — owner: pm-frontend — why: sitting in review.
- [medium] Backend audit of community/disputes/messaging/neighbors/privacy endpoints — owner: pm-security — why: SPA guards alone are insufficient; verify role/tenant/RLS.
- [low] Unblock mobile-native/KMP cloud builds (long-standing structural gap; issue #2652 closed but gap persists) — owner: pm-devops — why: 5+ backlog rows remain permanently unclaimable in cloud.

### Screen-map drift

- none this run (drift signals fire on PR diffs, not stories; see signals/2026-11-14.json).

Buffer: 55/36 open · 0 candidates ranked but unqueued
