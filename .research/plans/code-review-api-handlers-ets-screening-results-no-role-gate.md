# code-review-api-handlers-ets-screening-results-no-role-gate

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 rotating expert review of `api-handlers` segment (2026-06-26)
**Confidence:** medium

## Hypothesis

Mutating endpoints under `backend/servers/api-server/src/routes/enhanced_tenant_screening/{screening_results,models}.rs` accept create/update from any authenticated org member, with no role check. This lets any tenant member inject credit / background / eviction result records for any screening in their org, and — more dangerously — flip the *active* risk-scoring model (`models.rs:98 activate_risk_model`) that drives the AI tenant approve/deny decision. RLS confines the writes to the caller's org but does not gate role-within-org; the fix is to add a `screening-admin` (or equivalent existing role) check at the top of each mutating handler, matching the role-gate pattern used in `routes/emergency/broadcasts.rs:40` and `incidents.rs:49`.

## Evidence

- `backend/servers/api-server/src/routes/enhanced_tenant_screening/screening_results.rs:44` — `create_credit_result` accepts `CreateScreeningCreditResult` with no role check
- `backend/servers/api-server/src/routes/enhanced_tenant_screening/screening_results.rs:87` — `create_background_result` same gap (criminal/background forge surface)
- `backend/servers/api-server/src/routes/enhanced_tenant_screening/screening_results.rs:130` — `create_eviction_result` same gap
- `backend/servers/api-server/src/routes/enhanced_tenant_screening/models.rs:39` — `create_risk_model` gates only on RLS
- `backend/servers/api-server/src/routes/enhanced_tenant_screening/models.rs:98` — `activate_risk_model` (flips the active AI model) gates only on RLS — any org member can swap the model driving tenant approve/deny

## Files

- `backend/servers/api-server/src/routes/enhanced_tenant_screening/screening_results.rs`
- `backend/servers/api-server/src/routes/enhanced_tenant_screening/models.rs`
- `backend/servers/api-server/src/routes/enhanced_tenant_screening/mod.rs`
- `backend/servers/api-server/src/routes/emergency/broadcasts.rs`
- `backend/servers/api-server/src/routes/emergency/shared.rs`
- `backend/crates/common/src/tenant.rs`

## Dependencies

## Required capabilities

- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. Start the api-server: `cargo run -p api-server`.
2. As a regular org member (not `screening_admin` / `manager`), call `POST /api/v1/enhanced-tenant-screening/screening-results/credit` with a valid `CreateScreeningCreditResult` body for any `screening_id` in the same org.
3. Expected: `403 Forbidden`.
4. Actual on `dev`: `201 Created` — the credit result is persisted. Repeat for `/background`, `/eviction`, `/models`, and `POST /models/{id}/activate`; all return 201/200 instead of 403.

## Suggested approach

1. Identify or introduce the right role check. The role enum is `TenantRole` at `backend/crates/common/src/tenant.rs:83`; sibling `routes/emergency/shared.rs:53` defines `is_emergency_manager(role: TenantRole) -> bool` and is used by `broadcasts.rs:40`. Choose between: (a) introducing an analogous `is_screening_admin` helper next to the ETS module (preferred for symmetry), or (b) reusing an existing `is_org_manager`/`is_manager` predicate if one already exists in `common::tenant`. Grep `TenantRole` usage to pick.
2. Open `screening_results.rs`. At the top of `create_credit_result` (line 44), `create_background_result` (line 87), `create_eviction_result` (line 130), add:
   ```rust
   if !is_screening_admin(rls.role()) {  // or the project's chosen role helper
       return Err(ApiError::forbidden("screening-admin role required"));
   }
   ```
3. Open `models.rs`. Add the same guard to `create_risk_model` (line 39) and `activate_risk_model` (line 98). The activate path is the most sensitive — it changes which model the AI uses to score future tenants.
4. Write a regression test at `backend/servers/api-server/tests/ets_role_gate_tests.rs`: seed admin + regular members, assert admin gets 201/200 and regular gets 403 for all 5 mutating endpoints.
5. Run `cargo fmt --all && cargo clippy -p api-server -- -D warnings && cargo test -p api-server ets_role_gate`.
6. Open the PR; include the cross-role 403 matrix in the description and note the `activate_risk_model` blast radius.

## Alternatives considered

- **Add the role check as an `axum::middleware::from_fn` layer at the mod.rs router** — rejected because the sibling read endpoints (`list_*`, `get_*`, `read_*`) legitimately serve all org members; a layer would block reads too, and splitting the router into "admin" and "read" sub-routers is more invasive than 5 one-line handler gates.
- **Rely on a Postgres-level RLS policy that consults the JWT role claim** — rejected because RLS rows already scope by org_id; adding a role predicate at the DB layer duplicates application logic and makes the 403 vs 404 distinction harder to test and audit.

## Root-cause trace

1. Symptom: regular org members can inject screening results and flip the active AI risk model.
2. ← Handlers at `screening_results.rs:44/87/130` and `models.rs:39/98` extract `rls: RlsConnection` but never inspect `rls.role()`.
3. ← The `enhanced_tenant_screening` module was introduced as Story-level work; the role-gate idiom from sibling `emergency/` and `incidents.rs:49` was not adopted because the module pre-dates the helper extraction.
4. Origin: enhanced_tenant_screening module introduction (pre-2026-06; the 2026-06-24 hotspot split (#1816) restructured the module but did not add role gates).

## Test plan

- [ ] `backend/servers/api-server/tests/ets_role_gate_tests.rs` — new integration test, 5 cases (create_credit/background/eviction + create_model + activate_model), asserts admin gets 201/200 and regular member gets 403.
- [ ] Read endpoints continue to serve all org members (regression test for at least one `list_*` to confirm we did not over-gate).
- [ ] `cargo test -p api-server --test ets_role_gate_tests` is the focused command.

## Out of scope

- Read endpoints (`list_*`, `get_*`, `read_*`) — those legitimately serve all org members.
- The wider `code-review-api-handlers-ets-error-leak` finding (raw sqlx text in 500 responses across ~27 ETS handler arms) — separate plan / separate scope.
- Refactoring role helpers into a single trait or extractor.

## After-merge

- Move this file to `plans/_archive/code-review-api-handlers-ets-screening-results-no-role-gate.md`
- Mark the matching `backlog.json` row as `status: "done"`
