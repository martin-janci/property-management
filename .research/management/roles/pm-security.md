# pm-security — 2026-05-27

_Rotating role this run (pm_cursor idx 5). Static read; no compile/run._

## Summary

Authz/tenant-scoping is the dominant risk surface this window. Three open post-merge
security follow-ups are confirmed exploitable-by-omission on the new reports surface:
**#614** (`update_schedule` missing RBAC capability check), **#624** (`update_schedule`
missing tenant/org scope — cross-tenant schedule mutation), and **#617** (cookie `Path`
breaking change introduced by the #565 session-cookie scope hardening). The #565
SameSite/Secure/scoped-cookie hardening (P0-12) landed this window — good — but it shipped
a `Path` regression (#617) that must be reconciled before release, and the residual
P1-04 Debug-format audit-hash leak from PR #435 is still open.

## next_actions

- **[high]** Fix #624 + #614 together: thread `tenant_id`/`org_id` scope AND a
  `RequireCapability` extractor into `update_schedule` (PUT
  `/api/v1/reports/schedules/{id}`) in `report_schedule.rs`; add a cross-tenant +
  unauthorized-role regression test. DoD: foreign-tenant/unauthorized callers get 403/404,
  test in CI. dependency: pm-backend.
- **[high]** Reconcile the #617 cookie `Path` breaking change from PR #565 (P0-12): confirm
  the new `Path` scope does not silently log existing sessions out or widen/narrow the
  cookie beyond the API prefix; document the migration. DoD: session set/clear verified
  across `/` and API prefix; no auth regression. dependency: none.
- **[high]** Close P1-04 Debug-format audit-hash domain leak (PR #435 residual): replace
  `{:?}` on internal types in audit-trail records with `Display`/structured fields. DoD:
  no Debug-formatted internal types in audit log lines. dependency: none.
- **[medium]** Land the OAuth introspection + refresh-rotation security tests
  (pm-qa-oauth-security-tests): revoked-token rejection, family-reuse replay block, PKCE
  S256 enforcement — 10a-* shipped without end-to-end security coverage. dependency: pm-qa.
- **[medium]** Audit the reports.rs / report_schedule.rs churn cluster for sibling missing
  scope (the #614/#624 pattern often repeats across CRUD handlers in the same module).
  DoD: every mutating reports handler binds + uses the principal's tenant/org. dependency:
  pm-backend.

## risks

- **update_schedule cross-tenant + missing-RBAC (high/high):** #624 + #614 — an
  authenticated user can mutate another tenant's report schedule and/or mutate schedules
  without the required capability. Same omission class as the prior `ai.rs` equipment IDOR
  cluster. Mitigation: scope + capability extractor + regression test before promoting 81-1.
- **Cookie Path breaking change (#617) (medium/high):** the #565 cookie-scope hardening
  changed cookie `Path`; if mis-scoped it either logs users out or leaks the session to
  unintended paths. Mitigation: verify set/clear across paths; document migration.
- **Audit-hash Debug-format leak P1-04 (medium/medium):** internal type internals may reach
  audit logs via `{:?}`. Mitigation: structured/Display formatting.
- **OAuth provider untested security contract (medium/high):** 10a-1/10a-2/10a-3 shipped
  with no introspection/refresh-rotation/PKCE security tests; a refactor could silently
  reintroduce revoked-token acceptance or replay. Mitigation: add the security test suite.

## open_questions

- Does `security-test-gate.yml` actually block security-labelled PRs lacking a test file,
  or is it advisory? PR #497 and several reports PRs shipped without tests.
- Are #614 and #624 the only missing-scope handlers in `report_schedule.rs`, or does the
  whole CRUD set share the omission?

## decisions_needed

- Treat #614/#624/#617 as pre-release P0/P1 blockers gating Epic 81 promotion — owner: pm-security.
