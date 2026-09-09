# code-review-api-handlers-delegation-null-unit-no-owner-check

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review 2026-09-09 api-handlers delegations (signals/2026-09-09-api-handlers-tier1d.json)
**Confidence:** high

## Hypothesis
`POST /api/v1/delegations` (`create_delegation`) only validates unit ownership *inside* the `if let Some(unit_id) = req.unit_id { ... }` branch. When the client omits `unit_id`, the ownership gate is skipped entirely and the handler mints a delegation with arbitrary scopes (up to `["all"]`) to any user. The companion `has_delegation()` in the delegation repository treats a NULL `unit_id` row as a **wildcard over every unit** and `"all"` as a wildcard over every scope — and the SQL runs against `&self.pool` (not an RLS-scoped connection), so the wildcard is effectively cross-tenant. Any caller can confer authority they do not themselves hold. Today the exposure is a delegation-model integrity break (only `check_delegation` reads it); it becomes a direct broken-access-control escalation the moment any handler starts trusting `has_delegation` to authorise an action. Smallest fix: require `unit_id` on create *or* constrain the NULL-unit grant to units the delegator actually owns; mirror the `get_owners_rls` ownership check already used on the `Some(unit_id)` branch.

## Evidence
- `backend/servers/api-server/src/routes/delegations.rs:207-236` — `create_delegation` only checks `get_owners_rls` + `is_owner` inside the `if let Some(unit_id) = req.unit_id { ... }` block; when the field is `None` the code falls straight through to `delegation_repo.create(auth.user_id, create_data)` with `unit_id: None` and unchecked scopes.
- `backend/crates/db/src/repositories/delegation.rs:277-303` — `has_delegation()` SQL: `WHERE delegate_user_id = $1 AND status = 'active' AND (unit_id = $2 OR unit_id IS NULL) AND ($3 = ANY(scopes) OR 'all' = ANY(scopes)) AND start_date <= CURRENT_DATE AND (end_date IS NULL OR end_date >= CURRENT_DATE)`. Runs against `&self.pool`; NULL row and `"all"` scope are both wildcards.
- `backend/servers/api-server/src/routes/delegations.rs:676-694` — `check_delegation` (`GET /api/v1/delegations/check/{unit_id}/{scope}`) returns `has_delegation: true` for a delegate holding an ownerless unit-less delegation, for units the delegator never owned.
- The evidence bundle is fully traced entry → repo → SQL and the finding is `confidence: high`.

## Files
- `backend/servers/api-server/src/routes/delegations.rs:207`
- `backend/crates/db/src/repositories/delegation.rs:277`
- `backend/servers/api-server/tests/suites/governance_delegations_neighbors_happy_path_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (security-critical access-control gap; the reachability trace has to hold)
- [x] C2 — Seed data (need a two-tenant fixture with owner + delegate + a third-party unit to prove the wildcard)
- [x] C3 — Dev instance running (integration test hits the real Postgres via SQLx)
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived):** `Mode: local-only` (issue #2949 blocks cloud builds of the `api-server` binary crate — `utoipa-swagger-ui` build-script egress-403; verify locally until that lifts).

Mode: local-only (reason: cloud runner cannot compile api-server due to #2949 utoipa-swagger-ui egress block)

## Repro steps
1. Bring up the local dev stack: `stack up pm-local` (see `ppt-dev-stack` skill).
2. Seed two orgs A and B; in A create user `owner@a` who owns unit `unit-A`; add user `delegate@a` (any unit membership fine); in B create unit `unit-B` owned by `owner@b`.
3. Authenticate as `owner@a` (or ANY member of org A who is not `owner@a`).
4. `POST /api/v1/delegations` with `{ "delegate_user_id": "<delegate-a>", "scopes": ["all"], "unit_id": null, "start_date": "today" }`. Observe HTTP 201 despite the caller not owning any unit (or, in the more damning case, not being a manager at all).
5. As `delegate@a` call `GET /api/v1/delegations/check/<unit-B>/all` — observe `has_delegation: true` even though `unit-B` is in a different org.
6. Expected after fix: step 4 returns HTTP 403 (or 400) when `unit_id` is `None` unless the caller owns at least one unit; step 5 returns `has_delegation: false` for a unit the delegator does not own.

## Suggested approach
1. In `backend/servers/api-server/src/routes/delegations.rs:207-236`, hoist the `get_owners_rls` + `is_owner` check *out* of the `if let Some(unit_id)` branch. When `req.unit_id` is `None`, require the caller to have `get_units_owned_by_user(auth.user_id)` return a non-empty list — otherwise return `Error::forbidden("NOT_OWNER")`.
2. If the product intent is "a NULL-unit delegation covers all units the delegator owns" (not all units, ever), narrow `has_delegation()` in `backend/crates/db/src/repositories/delegation.rs:277-303`: join to `unit_ownership` on `owner_user_id = delegation.owner_user_id` and require the target unit's ownership row to belong to the delegator. Preferred over widening the app-layer check.
3. Reject `["all"]` scopes on the NULL-unit branch (or require an explicit `super_admin` gate) to remove the compounded wildcard.
4. Add an integration test suite `delegations_null_unit_authz_tests.rs` covering: (a) non-owner + `unit_id: null` → 403; (b) owner + `unit_id: null` grant + `check(delegate, foreign-unit)` → `false`; (c) owner + `unit_id: null` grant + `check(delegate, own-unit)` → `true`.
5. Backfill: run a one-off SQL audit on staging for `SELECT id, owner_user_id, delegate_user_id, scopes FROM user_delegations WHERE unit_id IS NULL AND status = 'active';` and manually review each. Add the audit query to `docs/security/audits/` as a follow-up.

## Alternatives considered
- **Reject `unit_id: null` at the API layer entirely** — rejected because coverage-of-all-owned-units is a legitimate business shape (e.g. a landlord going on holiday delegates every unit at once). Narrowing the semantic to "all units the delegator owns" preserves the feature without the cross-tenant hole.
- **Add RLS to the `has_delegation` SQL and rely on that** — rejected because `has_delegation` is called from paths that legitimately run with the pool connection (e.g. system checks). Narrowing the join is a smaller blast radius than reshaping the connection contract for a single query.

## Root-cause trace
1. Symptom: `has_delegation(delegate, foreign_unit, "any_scope") == true` after a caller with no unit ownership POSTs `{ unit_id: null, scopes: ["all"] }`.
2. ← `backend/servers/api-server/src/routes/delegations.rs:207-236` — ownership check is *inside* the `Some(unit_id)` branch; the `None` path calls the repo directly.
3. ← `backend/crates/db/src/repositories/delegation.rs:277-303` — the `unit_id IS NULL` OR-clause turns a targeted delegation into a universal one; `&self.pool` runs it without RLS.
4. Origin: the delegation feature landed with the `unit_id` field optional (product need) but the API-layer guard was written for the `Some` branch only. `git log --follow backend/servers/api-server/src/routes/delegations.rs` around `create_delegation` will pinpoint the commit; the fix's own PR should reference it.

## Test plan
- [ ] `backend/servers/api-server/tests/delegations_null_unit_authz_tests.rs` — new integration file with the three cases above; must fail on `main` and pass after the fix.
- [ ] `backend/crates/db/tests/delegation_repo_null_unit_tests.rs` — new repo test asserting `has_delegation()` returns false for a foreign unit even when a NULL-unit row exists.
- [ ] Run: `cargo test -p api-server delegations_null_unit_authz` and `cargo test -p db delegation_repo_null_unit`.

## Out of scope
- Any changes to `check_delegation`'s response shape or the delegate-facing UI.
- Rewriting the delegation feature to use RLS end-to-end (larger architectural change).
- Filing follow-up cross-tenant IDOR issues #2944/#2945/#2946 (separate, already tracked).

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-delegation-null-unit-no-owner-check.md`
- Mark the matching `backlog.json` row as `status: "done"`
- Run the one-off staging SQL audit named in Step 5 above and log the result in `docs/security/audits/`.
