# pm-security — 2026-11-25

## Summary

UC-10 person-month submissions (#3775) are authorized correctly at the handler level — tenant check, unit-in-building check, active-residency for submit, manager role for approve/reject, and an atomic approve transaction with a pending-only guard. **They have no DB-level defense in depth**, because the submission repo calls run on the pool rather than the RLS connection, and the new `person_month_submissions` table does not have RLS `FORCE`'d. The RLS-migration residual (the `TODO: migrate to *_rls` markers) is closed; the remaining risk is the non-`FORCE` + pool-bypass pattern, which has been confirmed on UC-10 and is not yet audited repo-wide.

## Next actions

1. **[high]** Add `ALTER TABLE person_month_submissions FORCE ROW LEVEL SECURITY` (and the same for `person_months` if not already forced) in a new migration. Move submission repo reads/writes (`submit`, `find_submission`, `list_*_submissions`, `is_active_resident`) onto the request RLS connection. Add a cross-tenant integration test showing org B cannot read or decide org A submissions even when the handler check is bypassed.
2. **[medium]** Restrict non-manager reads on person-month endpoints (GET unit/yearly/by-id, POST /calculate, GET building list/summary). They currently only require org membership, so any resident can read other units' occupancy counts. Gate them on residency of that unit or manager role.
3. **[medium]** Fix the submit overwrite semantics. `ON CONFLICT DO UPDATE` lets any co-resident overwrite another resident's pending proposal and rewrites `submitted_by`. Either reject when a pending row by another user exists, or keep an audit trail of the replaced row.
4. **[medium]** Add negative-authz tests to `person_months_happy_path_tests.rs`: non-resident submit → 403, resident approve → 403, cross-building `submission_id` → 404, double-approve → 409, future / over-24-month period → 400.
5. **[medium]** Audit repo-wide for the same pool-instead-of-RLS-conn pattern and for RLS tables that are not `FORCE`'d. Prior migrations 00171, 00172, 00178, 00179, 00180 were force-fixes, which shows the pattern recurs. Rank tables by sensitivity.

## Risks

- **Defense-in-depth gap on new `person_month_submissions`** — policy not `FORCE`'d + repo uses the pool. Tenant isolation rests only on handler checks (building tenant filter + `belongs_to_building`). A future handler that omits those checks becomes a cross-tenant IDOR. (medium × high).
- **Occupancy data of any unit in the org is readable by any org member**, including residents of other units. Pre-existing on read endpoints; the new resident role widens exposure. (high × medium).
- **Billing integrity**: person counts feed utility cost allocation. A co-resident can replace a pending submission, and manager approval then overwrites the `person_months` count with `source='manual'`. Only guard is manager review. (low × medium).
- **Pattern recurrence of RLS migration residual** — marker backlog item is done, but the non-`FORCE` + pool-bypass pattern persists and is unaudited repo-wide. Residual score of 2 should not be read as fully closed. (medium × medium).

## Open questions

- Is `person_months` itself `FORCE`'d? Handler comment says RLS is not `FORCE`'d, but not verified against the migrations.
- Does the DB app role own the tables or hold `BYPASSRLS`? If so, non-`FORCE` RLS is a no-op for pool queries.
- Are `unit_residents` rows verified (invite / approval), or can a user self-assign as a resident?
- Are the 2026-11-22..24 security PRs (#3761, #3750, #3730, #3744, #3759, #3769) fully covered by tests? Does the SSRF validator also handle DNS-rebinding and redirect-follow?
- Do the 0 hits for `TODO migrate *_rls` still reflect reality, given the 2026-05-20 brief counted 31?

## Decisions needed

See `.research/management/decisions.md` — 2026-11-25 block.
