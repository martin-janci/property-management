# pm-qa — 2026-11-08

## Summary

QA posture is solid — all 5 merged PRs this window shipped with tests (regression + i18n + boundary). One flaky test flagged by PR #3356 verify output (`frontend/apps/ppt-web/src/routes/groups/rentals.connections.test.tsx`, confirmation-dialog `findByRole` timeout under full-suite parallel load; passes 8/8 in isolation). Mobile RN jest-expo infra (#2951) and KMP cloud builds (#2652) remain blocked — RN regression coverage is dark.

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Chase unblock of mobile-native KMP (#2652) + mobile-rn jest-expo (#2951) — RN regression coverage is dark | pm-devops | RN unit suite runs green on CI |
| medium | Harden `frontend/apps/ppt-web/src/routes/groups/rentals.connections.test.tsx` confirmation-dialog `findByRole` flake (surfaced on PR #3356 verify) | none | Test passes 10/10 under `--test.pool=threads` |
| medium | Add integration test matrix for `POST /api/v1/auth/password/change` session-revocation (PR #3353): current session kept, other refresh tokens revoked, redis session eviction | none | Integration test in `backend/servers/api-server/tests` covers all 3 cases |

## Risks

| Probability | Impact | Risk | Mitigation |
|---|---|---|---|
| medium | medium | ppt-web RegistryPage destructive mutations (pre-PR #3356) silently fail — console.error only, no toast/inline surface. PR #3356 fixes the four handlers, but the swallow-mutation-error anti-pattern likely exists elsewhere in ppt-web. | Grep for `.catch\s*(\s*\(?[^)]*\)?\s*=>\s*\{?[^}]*console\.error` in ppt-web mutation handlers; expand regression tests for destructive mutations across features |
| medium | high | `BookFacilityPage.tsx:49-55` `fetchAvailability` catch swallows error (console.error only) — on load-failure the previously-rendered slots stay visible and user can submit a booking against stale availability. | Mirror sibling `fetchFacility` (L35-36) setError pattern; add regression test that an availability fetch failure clears the slot list and surfaces a load-error banner |

## Open questions

_None._

## Decisions needed

_None._

## Release-readiness signal

- All 5 merged PRs in window shipped with IG3-style fail-before-fix evidence (visible in PR #3353, #3351, #3356 commit splits).
- Draft PR #3356 is release-ready pending ready-for-review + merge.
- No test-gap blockers surfaced this run beyond the known RN/KMP CI holes and the one parallel-load flake.
