# pm-scrum-master — delivery synthesis

_Generated: 2026-11-03 (routine Phase 1.6 incremental upkeep)_

## Summary

Quiet-but-productive window: 5 PRs merged (#3216–#3220), all quality-track — i18n coverage, docs reconcile, extracted + tested helpers, LIKE-escape hardening. No story status flips (coverage 49/49 across 13 epics unchanged). Carried blockers unchanged: #3187 (new last run) still red on dev; #3168 cargo-deny, #3164 add_evidence authz, #3131 merge deadlock, #2652 KMP, #2951 jest-expo all unresolved. Role focus today is **pm-qa** (70 days stale; test strategy ownership relevant because every PR this run shipped with explicit IG3 TDD evidence, and the dev-red CI is a test-vs-code contract gap).

## shipped_since_last_run

- **#3216** screen-map-drift-pr-3212-reality — reconcile `docs/screens/reality/account-listing-edit.md` with PR #3212 (buildStatus/apiStatus, agent log, notes)
- **#3217** repeated-churn reality-web listing-edit — extracted silent-zero numeric coercion into tested `_number` helper (preserves explicit `0`, matches `_price`/`_status` sibling pattern)
- **#3218** code-review ppt-web EvidenceUploader — i18n validate errors, drop-zone, placeholder across 6 locales; added evidenceUploaderI18n.test.ts
- **#3219** churn-hotspot reality-server price_map.rs — escape LIKE metacharacters in `city` filter (public unauthenticated endpoint hardening) + regression test
- **#3220** code-review ppt-web community feature — react-i18next across every community component + 323 leaf keys × 6 locales + communityI18n.test.ts

## sprint_progress

```json
{"sprint":"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth","epics_done":4,"epics_total":4}
```

(13 epics in coverage, all `done`; the active sprint names 4 of them.)

## Blockers (unchanged from 2026-11-02)

| # | Why | Owner |
|---|---|---|
| #3187 | `cross_org_assign_violation_is_not_found` expects 404, gets 403 — dev `backend.yml` red | pm-security |
| #3168 | cargo-deny RUSTSEC advisories gate every backend PR merge | pm-security |
| #3164 | violations `add_evidence` sub-resource ungated | pm-backend |
| #3131 | merge-pipeline deadlock | pm-backend |
| #2652 | mobile-native/KMP builds blocked in cloud runner | pm-devops |
| #2951 | mobile RN jest-expo version rot | pm-devops |

## Open PRs touched this window

- **#3221, #3222** — dispatcher drafts awaiting review (gated on #3187 if backend path)
- **#3193** — carried draft, awaiting review

## next_actions (top delivery moves)

```json
[
  {"action":"Resolve #3187 cross-tenant assign-404 contract (pm-security decides contract; pm-backend implements; pm-qa pins the regression) — unblocks dev backend.yml","priority":"high","dependency":"pm-security","definition_of_done":"backend.yml green on dev; violations_mutation_cross_org_idor_tests.rs green with documented contract"},
  {"action":"Resolve #3168 cargo-deny RUSTSEC advisories (bump transitive crate or bounded exception with deadline captured in action-list) — unblocks every backend PR merge","priority":"high","dependency":"pm-security","definition_of_done":"cargo-deny advisories green on dev; exception if any carries a deadline"},
  {"action":"Land violations add_evidence tenant-scoped authz gate (#3164) + sweep add_* sub-routes for the same pattern (community add_* / documents add_signature / mediation add_evidence)","priority":"high","dependency":"pm-backend","definition_of_done":"PR merged with cross-tenant integration test; sweep report notes which sister sub-routes are already gated"},
  {"action":"Lift the mobile-rn fake-submit-pattern cluster (ReportFault / reality-web /sell / VoteDetail ranked ballot) into a single per-screen MSW Submit-POST gate — close the chronic defect class","priority":"medium","dependency":"pm-frontend","definition_of_done":"CI gate added; at least 2 of 3 clustered bugs' tests retrofitted"},
  {"action":"Clear buffer underflow — action-list at 18/36 open; dispatcher backlog scan is overdue","priority":"low","dependency":"none","definition_of_done":"next dispatcher sweep lands ≥18 new ranked items OR routine confirms backlog is genuinely drained"}
]
```

## risks

```json
[
  {"risk":"Backend dev CI red for ≥24h (#3187 + #3168) — long red windows normalize working around the gate, which erodes the TDD discipline every PR this week showed","probability":"high","impact":"high","mitigation":"Treat both as release-blockers; routine Phase 6 to alarm on dev-red > 24h; pm-security owns both resolutions"},
  {"risk":"Role rotation is skewing backend-heavy — pm-qa 70-day stale, pm-tech-lead 12d, pm-security 33d, pm-devops 64d; staleness-weighted rotation was proposed 2026-10-28 but is still unqueued","probability":"medium","impact":"medium","mitigation":"Land the staleness-bias change on pm_cursor (prefer any role >60d stale over the current index) — single file change to routine code; closes the carried pm-frontend-rotation-stale meta-risk"}
]
```

## open_questions

- Is #3131 (merge pipeline deadlock) a quiet carry or an active blocker right now? It has survived multiple runs without a reproducer — may be stale.

## decisions_needed

- Cross-tenant 404-vs-403 contract for violations.assign — owner: pm-security
- Staleness-bias on pm_cursor — owner: pm-scrum-master
