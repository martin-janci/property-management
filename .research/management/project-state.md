# PPT Project State

_Generated: 2026-11-08 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (74-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a re-checked — all 3 OAuth stories still done; PR #3353 session-revocation evidence appended to 10a-3). pm-scrum-master always-on._

## Delta since last routine run (2026-11-06)

- **5 PRs merged** to dev:
  - **#3353** — `gh-issue-3352: revoke other sessions on authenticated password change` (api-server auth/password-change path; security hardening). Closes #3352.
  - **#3354** — `code-review-ppt-web-ui-registry-page-no-auth: send auth context on registry requests` — registry client rebuilt per canonical `useBuildings.ts` getToken+useMemo pattern.
  - **#3355** — `code-review-ppt-web-ui-registry-page-i18n-gap: translate RegistryPage chrome` — title/subtitle/tabs/load-error routed through `t()`.
  - **#3351** — `code-review-ppt-web-ui-registry-no-i18n: wire registry pet/vehicle forms + lists through react-i18next` — 4 registry components fully i18n'd across all 6 locales.
  - **#3350** — `code-review-reality-web-share-cmp-unchecked-json: validate shared-comparison listing bodies at the API boundary` — unchecked cast replaced with parseJsonBody-style shape validation.
- **1 new issue: #3357** — `dispatcher: pipeline fully stalled — 3 approved PRs blocked on non-mechanical conflicts + Tier-1d signals are a dead-end`. Already queued as `pm-scrum-master-unblock-dispatcher-stall-3357-2026-11-08` (high). **Blocks dispatcher flow.**
- **1 open draft: #3356** — `code-review-ppt-web-ui-registry-mutation-silent-fail: surface RegistryPage delete/review mutation errors` — Registry destructive handlers now use useToast (success + persistent error); 6 new regression tests. Verify green. Ready to mark ready-for-review.
- **Carried blockers unchanged**: #2652 (KMP cloud builds), #2951 (RN jest-expo), plus (new this run) #3357 dispatcher stall.

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; evidence refreshed on epic-10a (cursor re-check — PR #3353 session-revocation strengthens 10a-3 OAuth Token Management surface, though 10a-3 was already shipped).
- **5 PRs merged** — four code-review-driven cleanups (registry i18n/auth-context + reality-web JSON shape) plus one security hardening (session revocation on password change). Pattern: the code-review signal pipeline is working on `done` surfaces, not new sprint stories.
- **pm-qa rotation came up 74 days stale** — last run 2026-08-25. Two risks surfaced: the swallow-mutation-error anti-pattern (pre-PR #3356 made delete/review look like success) and the sibling `BookFacilityPage.tsx` silent availability-fetch failure (Phase 1.5 finding). Three pm-qa next_actions queued (one high, two medium).
- **New blocker on dispatcher**: #3357 fully stalled on 3 approved PRs + Tier-1d dead-end — needs human triage.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level (all four named sprint epics are status: done in sprint-status.yaml). Coverage holds 13 epics all at done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done · **last_checked 2026-11-08 (cursor re-check, 10a-3 evidence appended)** | #3353 (session-revocation on password change — strengthens 10a-3) |
| 10B — Platform Administration | done (7/7) | 7/7 done | — |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |
| 9 — 2FA/TOTP | done (1/1) | 1/1 done | — |
| 79/82/83/84/85 (extended) | — | all done | #3350 (reality-web), #3351/3354/3355 (ppt-web registry) |

## Shipped since last run (5 PRs merged)

- **#3353** — revoke other sessions on authenticated password change (closes #3352). Security hardening on `POST /api/v1/auth/password/change` — other refresh tokens now invalidated, current session kept valid.
- **#3354** — registry client now reads `getToken()`+`useMemo` per canonical `useBuildings.ts` pattern — Bearer/tenantId now ship with every registry request; module-scope stub removed.
- **#3355** — RegistryPage title/subtitle/tabs/load-error routed through `t()`; added `registry.page.*` keys across 6 locales.
- **#3351** — Four registry components (PetRegistrationForm, VehicleRegistrationForm, PetRegistrationList, VehicleRegistrationList) wired through `useTranslation`; all strings in 6 locales.
- **#3350** — reality-web shared-comparison loader replaced unchecked `as Promise<ListingSummary>` cast with `parseJsonBody`-style boundary validation; malformed API bodies now rejected.

## What's next (top 5, replaces sprint-only view)

1. **[high]** Unblock dispatcher pipeline (gh-issue-3357) — 3 approved PRs on non-mechanical conflicts + Tier-1d dead-end. Owner: pm-scrum-master.
2. **[high]** Chase unblock of mobile-native KMP (#2652) + mobile-rn jest-expo (#2951) — RN regression coverage is dark. Owner: pm-devops.
3. **[medium]** Land PR #3356 (RegistryPage mutation error surfacing) — close out the registry triage cluster. Owner: pm-frontend.
4. **[medium]** Add integration test matrix for POST /auth/password/change session-revocation (PR #3353 follow-up). Owner: pm-qa.
5. **[medium]** Harden `rentals.connections.test.tsx` confirmation-dialog findByRole flake (surfaced on #3356 verify). Owner: pm-qa.

## Blockers

- **#3357** — Dispatcher pipeline fully stalled (new today, age 0d). Owner: pm-scrum-master.
- **#2652** — mobile-native KMP cloud builds blocked (age 50d+). Owner: pm-devops.
- **#2951** — RN jest-expo infrastructure (age 80d+). Owner: pm-devops.

## Role focus today: pm-qa, pm-scrum-master

### pm-qa — one-line summary

QA posture solid — all 5 merged PRs shipped with tests. Flagged the swallow-mutation-error anti-pattern (PR #3356 fixes four handlers; grep-sweep recommended for other ppt-web features) and the `BookFacilityPage.tsx:49-55` silent availability-fetch failure (Phase 1.5 finding). Mobile RN/KMP CI blockers still dark.

### pm-scrum-master — one-line summary

5 PRs merged; delivery continues on code-review surfaces (not sprint stories). New blocker #3357 (dispatcher stall) — needs human intervention on the 3 approved PRs + a decision on Tier-1d retirement.
