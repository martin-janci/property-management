# pm-tech-lead — 2026-10-20

**Summary:** The main architectural risk right now is the merge-pipeline deadlock from #3131: the dispatcher bot cannot self-approve its own PRs and 16 approved PRs are stuck in review, including the ammonia/zip RUSTSEC bumps and the migration-upload data-loss fix. Beyond that, the themes are RUSTSEC advisory churn (#3124/#3126) and unbounded-input hardening (price_map cache in #3092; `migration/import/upload` dead-store in #3127; `add_to_compare` TOCTOU still open in #3028). I read sprint-status.yaml only; the PR/issue facts come from Phase 1 observations, not independent re-verification.

## Next actions
| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Resolve the approval-identity deadlock in #3131: pick a separate approver identity or a GitHub App for auto-approve, drain the 16 stuck PRs | pm-devops | Bot-authored PRs merge without manual approval, branch protection intact, queue cleared |
| high | Land the RUSTSEC bumps: ammonia 4.2.4 (#3130, issue #3126) and rust_xlsxwriter/zip 9.x (#3125, issue #3124) | pm-security | cargo-deny green in CI; xlsx export + HTML sanitizer tests pass |
| medium | Review #3127 (migration upload persisted to storage) for storage-key tenancy, size caps, and cleanup on failure | pm-backend | Keys org-scoped; max-size cap present; failed import leaves no orphaned object |
| medium | Audit public-endpoint in-memory caches in reality-server for the unbounded-key pattern fixed in #3092 (PRICE_MAP_CACHE) | pm-security | Every public cache has a bounded size + TTL, or shares one bounded-cache helper |
| medium | Decide whether to keep or close the 14-day-old draft #3028 (add_to_compare TOCTOU) and fix with an atomic DB constraint / upsert instead of check-then-insert | pm-backend | Either merged with a unique constraint or closed with a reason |
| low | Merge #3129 (type the AML assessments list response) and confirm OpenAPI + generated clients stay in sync | pm-integration | api-validation CI passes; client regenerated with no drift |

## Risks
| Risk | Prob | Impact | Mitigation |
|---|---|---|---|
| Merge-pipeline deadlock: 16 approved PRs stuck, with security fixes queued behind them; long-lived branches add conflict and rework risk | high | high | Fix the approver identity (#3131); rebase and drain the queue oldest-first |
| Repeated dependency-advisory churn (ammonia, zip) with no RUSTSEC SLA or pin policy — cargo-deny keeps breaking CI | medium | medium | Set a RUSTSEC SLA; scheduled dependabot / cargo-deny; time-box temporary ignores with an owner |
| Per-process in-memory caches on public routes (the price_map pattern) are bounded per instance only; not shared across replicas | medium | medium | Shared bounded-cache abstraction, or Redis for caches needing cross-instance consistency |
| Untyped or stubbed API responses (AML list, migration upload that dropped the file) → contract drift between spec and handlers | medium | medium | Require utoipa response types; add handler-level persistence tests in review checklists |
| sprint-status.yaml stale — sprint name/goal from 2025-12 while work has moved to hardening + security follow-ups; weak planning visibility | high | low | Refresh sprint file, or add a rolling hardening epic |

## Open questions
- Why can the dispatcher bot not self-approve: is it GitHub policy (no self-approval), or a missing second identity in `approve-pr.yml` / `auto-approve.yml`?
- Are the 16 stuck PRs conflicting with each other, and are any security fixes among them?
- Does #3127 target the same storage abstraction and bucket layout as existing document uploads?
- Is there a documented cargo-deny exception policy, and who owns RUSTSEC triage?
- Which reality-server public caches besides `PRICE_MAP_CACHE` are keyed by user-supplied input?

## Decisions needed
- Approver identity for bot-authored PRs (GitHub App, second bot, or a human on a rota) — owner: pm-devops with the tech lead
- RUSTSEC response SLA and ignore-list policy — owner: pm-security
- Standard shared bounded-cache approach for public endpoints (in-process vs Redis) — owner: tech lead
- Sprint reset: define the current sprint goal beyond the 2025 Epic 6/7A/8A/10A framing — owner: pm-scrum-master
