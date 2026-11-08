# PPT routine Phase 1.6 — 2026-11-08

Role focus: **pm-qa** (74-day stale; pm-cursor idx 3 → 4). Coverage upkeep: epic-10a re-checked (cursor idx 0 → 1); PR #3353 evidence appended to 10a-3-oauth-token-management.

## Delta since 2026-11-06

- **5 PRs merged** to dev: #3350 (reality-web JSON shape), #3351 (ppt-web registry i18n), #3353 (api-server session revocation on password change — closes #3352), #3354 (registry auth-context), #3355 (RegistryPage i18n chrome).
- **1 new issue** #3357 (follow-up/blocked): dispatcher pipeline fully stalled on 3 approved PRs + Tier-1d dead-end.
- **1 open draft** #3356 (ppt-web RegistryPage mutation error surfacing — verify green, ready for review).

## Scrum Master verdict

Delivery continues on code-review surfaces, not sprint stories (all 49/49 stories done across 13 epics). Biggest blocker: #3357 dispatcher stall — needs human intervention.

## pm-qa findings

- **Risk** (medium/medium): swallow-mutation-error anti-pattern — pre-PR #3356 silently succeeded on destructive registry mutations. Grep-sweep recommended across ppt-web mutation handlers.
- **Risk** (medium/high): `BookFacilityPage.tsx:49-55` silent availability-fetch failure — user may book against stale slots. Mirror sibling `fetchFacility` setError pattern.
- **Next action** (high): chase unblock of mobile-native KMP (#2652) + mobile-rn jest-expo (#2951) — RN regression coverage dark.
- **Next action** (medium): harden `rentals.connections.test.tsx` flake surfaced on PR #3356 verify.
- **Next action** (medium): add integration test matrix for `POST /auth/password/change` session-revocation (PR #3353 follow-up).

## Coverage upkeep

Epic re-checked: **epic-10a** (OAuth Provider Foundation) — 3/3 stories still done; last_checked refreshed to 2026-11-08; PR #3353 session-revocation evidence appended to 10a-3-oauth-token-management.

Cursor: coverage_cursor idx 0 → 1 (next re-check epic will be epic-10b).

## Buffer health

action-list.json: 14 open · 12 in-progress · 0 done · 26 total. **Below the 36-target** — upkeep mode never refills aggressively (that's the `scan` mode's job). No action.
