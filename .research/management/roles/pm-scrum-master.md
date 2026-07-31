# Role: pm-scrum-master — 2026-07-31

> Delivery lead / coordinator. Always runs. Static read-only.

## Summary

The 2026-07-30 → 2026-07-31 window shipped **19 non-dispatcher PRs** (range #2580 → #2610), heavily weighted toward **security hardening + follow-through refactors**: 5 security fixes (#2593 Android SSO CSRF mint, #2596 AML cast, #2597 DELETE-by-file-key ref guard, #2600 tenant JSON XSS, #2603 viewsource cast), 3 test additions (#2594 support-tooling retention prune, #2595 portfolio analytics zero-view, #2604 voice webhook signature), 2 refactor extractions (#2599 schedule cadence, #2610 scheduler retention/prune), and infrastructure (#2607 reality-api-client drift gate, #2602 pre-existing frontend test failures repaired, #2601 admin-web mobile config Save fix). All 4 opened issues this window are already closed. The auto-review loop is genuinely converging — this run's follow-ups all hit their targets.

## Sprint progress

- Sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**
- epics_done: **3 / 5** (unchanged)
- Coverage composition unchanged: **47 done · 2 partial · 0 not-started** across 13 epics
- **84-1 unblocked** — dependency #2573 closed by PR #2597 (DELETE-by-file-key reference-check guard); the gap-84-1 retry chain can now be claimed without risk of the reap-race regression
- **84-2 unchanged** — signer document-sign page still not built; retry3 open
- Epic-7a re-checked (coverage_cursor idx 4 → 5): all 5 stories still done. Evidence for 7a-1 appended with PR #2597

## Shipped since last run (19 non-dispatcher PRs > #2579)

- **#2610** — refactor(api-server): extract scheduler retention/prune jobs to submodule
- **#2609** — code-review api-core resolved.rs: stop leaking raw sqlx/serde error text on public GET /layout/resolved
- **#2608** — code-review api-core scheduler: surface DB errors on scheduler notification-target lookups
- **#2607** — chore(api-validation): add reality-api-client drift gate (closes #2556)
- **#2606** — code-review api-core admin.rs: return 500 on failed layout serialize instead of null body
- **#2605** — docs(api-server): remove stale TODO(security) headers in faults/critical_notifications
- **#2604** — test(api-server): add voice webhook signature/verification unit tests
- **#2603** — code-review reality-web: validate untrusted ?source= against ViewSource set
- **#2602** — fix(admin-web): repair pre-existing frontend test failures blocking verify gate
- **#2601** — dx-admin-web: fix no-op Save paths (platform-settings + mobile-config)
- **#2600** — code-review reality-web: escape inlined tenant-config JSON
- **#2599** — refactor: extract schedule cadence/cron helpers from reports.rs
- **#2597** — fix(api-server): guard DELETE /documents/by-file-key against reaping a referenced object (closes #2573)
- **#2596** — code-review ppt-web: validate AML review decision before submit
- **#2595** — test(mobile-native): pin portfolio-analytics zero-view-day behaviour (no bug)
- **#2594** — test(api-server): DB-backed regression for support_tooling_events retention prune
- **#2593** — gh-issue-2574: mobile-native Android SSO initiation leg (mint CSRF nonce)
- **#2592** — fix(ppt-web): gate WebSocket console diagnostics behind import.meta.env.DEV
- **#2580** — refactor: de-duplicate platform_admin authz batch2 tests

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Claim 84-1 in the next dispatcher round (direct-to-S3 wire is now unblocked) | none | 84-1 flips partial → done; regression test lands |
| high | Cross-cutting frontend security pattern: lint/codemod for untrusted-to-union casts + SSR string interpolation | pm-tech-lead choose mechanism | Rule/codemod PR merged; sweep result reports zero remaining sites |
| medium | Shepherd accounting MVP-loop trio (#2555/#2558/#2559) — draft since 2026-07-28, still zero reviewer engagement | pm-tech-lead reviewer slot | Trio merges or explicit deferral logged |
| medium | Frontend verify-gate hygiene (post-#2602 silent failure window) — first-class dev-push signal | pm-devops | frontend.yml on-push test job OR verify-gate → routine signal |
| medium | Package scoped implementer brief for 84-2 signer page (3 no-PR attempts already) | none | Retry4 spawns with a green-test target and screen-map anchor |

## Blockers

- **Accounting MVP-loop trio (#2555 / #2558 / #2559)** — still zero reviewer engagement 3 days after drafting; dispatcher throughput bottlenecked on reviewer capacity. Owner: pm-tech-lead.
- **84-2 signer page retry3** — 3 no-PR attempts on record; needs scoped brief before retry4 spawns. Owner: pm-frontend.

## Risks

| Risk | Prob | Impact | Mitigation |
|---|---|---|---|
| Repeated XSS/cast pattern across ppt-web + reality-web same window (#2596/#2600/#2603) | high | high | Lint/codemod (queued) + SSR surface audit |
| Verify-gate silently permitted pre-existing frontend test failures for ≥1 window | medium | medium | On-push CI test job (queued) |
| 84-1 has been top partial for weeks; unblock signal now landed but consumer wiring still unclaimed | medium | medium | Explicit next-round claim with scoped brief |
| Accounting MVP-loop trio starvation continues (3rd day drafted with no reviewer) | medium | medium | Named reviewer slot; reviewer-rotation decision from 2026-07-30 still pending |

## Open questions

- Is the drift gate on reality-api-client (#2607) sufficient to catch ppt-web consumers that duplicate types, or does it only guard reality-web?
- After #2610's scheduler split, does schedule_cadence.rs (still churn hotspot this window) need further extraction before it stabilizes?

## Decisions needed

- Reviewer-slot policy for large-scope feature PRs (accounting trio) — owner: pm-tech-lead (carried from 2026-07-30, still unresolved).
- Frontend security pattern lint mechanism — owner: pm-frontend + pm-tech-lead (new 2026-07-31).
- Frontend verify-gate escalation — owner: pm-devops (new 2026-07-31).
