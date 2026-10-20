# pm-scrum-master — 2026-10-20

_Always-on synthesis run. 1-day lag since 2026-10-19 — on schedule._

## Summary

Narrow but security-forward delivery window: **2 PRs merged** (#3092 reality-server price-map cache DoS cap, #2902 saved-searches screen-map reconcile). Three new drafts opened (#3127 migration-upload data-loss, #3129 AML list typing, #3130 ammonia 4.2.4). **One critical new blocker**: #3131 — the dispatcher bot cannot self-approve its own PRs, leaving 16 approved PRs stuck in review (RUSTSEC bumps and the migration-upload fix among them). The RUSTSEC chain extends: #3124 (zip) → #3126 (ammonia follow-on).

## Shipped since last run (2 PRs)

- **#3092** reality-server PRICE_MAP_CACHE bounded (SECURITY) — anonymous key-flood DoS mitigation; inline tests included (+163/-4). Resolves `code-review-reality-server-price-map-cache-unbounded`.
- **#2902** saved-searches screen-map i18n reconcile — docs-only (+2 lines). Resolves earlier screen-map-drift from PR #2894.

## Sprint progress

- **Sprint:** _Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth_
- **Epics done:** 3/5 at sprint level; coverage-level 13/13 epics complete at 49/49 stories done (coverage-cursor `next_index` advancing from 10 → 11 today, epic-85 re-checked).

## Next actions (top 3)

| # | Priority | Dependency | Action | Definition of done |
|---|---|---|---|---|
| 1 | high | pm-devops | Fix approver-identity deadlock in #3131 (bot cannot self-approve), drain the 16-PR queue oldest-first | Bot PRs merge without manual approval; stuck queue cleared |
| 2 | high | pm-security | Land ammonia 4.2.4 (#3130, issue #3126) and rust_xlsxwriter/zip 9.x (#3125, issue #3124) | cargo-deny green on dev; xlsx export + HTML sanitizer tests pass |
| 3 | medium | pm-backend | Review #3127 for storage-key tenancy, size caps, and cleanup on failed import | Keys org-scoped; max-size cap; failed import leaves no orphaned object |

## Risks

| Risk | P | I | Mitigation |
|---|---|---|---|
| Merge-pipeline deadlock blocks 16 PRs including security fixes; conflict + rework risk grows per day | high | high | Fix approver identity (#3131); drain queue; rebase oldest-first |
| RUSTSEC advisory churn (ammonia, zip) with no SLA/pin policy — cargo-deny keeps breaking CI | medium | medium | Set RUSTSEC SLA; time-box ignores with owner; scheduled dependabot |
| Per-process caches on public endpoints (price_map pattern) not bounded across replicas | medium | medium | Shared bounded-cache helper or Redis for cross-instance consistency |

## Open questions

- Why can the dispatcher bot not self-approve: GitHub policy or missing second identity?
- Which security fixes are among the 16 stuck PRs?
- Does #3127 reuse the existing documents bucket layout and tenancy keying?

## Dependency chain this run

- #3131 (deadlock) blocks: #3125, #3127, #3129, #3130 (and 12 others not enumerated here).
- #3124 (zip RUSTSEC) feeds #3125 (fix PR, draft).
- #3126 (ammonia RUSTSEC follow-on) feeds #3130 (fix PR, draft).
