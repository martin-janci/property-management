# code-review-api-handlers-disputes-idor

**Vector:** security
**Score:** 3
**Source:** code-review 2026-10-14 (api-handlers Phase 1.5)
**Confidence:** high

## Hypothesis
`list_disputes` and `get_statistics` in `backend/servers/api-server/src/routes/disputes.rs` discard the authenticated `AuthUser` and trust the client-supplied `organization_id` query parameter. The backing `DisputeRepository::list` runs on the non-RLS management pool, so no DB-side guard catches the missing handler check. Any authenticated principal in org A can issue `GET /api/v1/disputes/?organization_id=<org_B_uuid>` and receive org B's complete dispute catalog (titles, descriptions, party counts) and statistics. The smallest fix mirrors the pattern already present in-file at `get_kpis`: derive `organization_id` from `AuthUser` via `require_org(&user)?` and drop the client-supplied field from `ListDisputesQuery` / `OrgQuery`.

## Evidence
- `backend/servers/api-server/src/routes/disputes.rs:367-384` — `list_disputes(_user: AuthUser, Query(query): Query<ListDisputesQuery>)` calls `state.dispute_repo.list(query.organization_id, (&query).into())`; `_user` is discarded.
- `backend/servers/api-server/src/routes/disputes.rs:387-404` — `get_statistics(_user: AuthUser, Query(query): Query<OrgQuery>)` calls `state.dispute_repo.get_statistics(query.organization_id)`; same shape.
- `backend/servers/api-server/src/routes/disputes.rs:415-420` — comment claims "The organization is derived from the caller's JWT tenant (never a query- or body-supplied org)", which is true for the sibling `get_kpis` (uses `require_org(&user)?`) but violated by the two handlers above.
- `backend/crates/db/src/repositories/dispute.rs:120-168` — `pub async fn list` runs `.fetch_all(&self.pool)` on the raw pool; `:171-177` comment confirms "the management API pool does not enforce RLS", so no DB-level guard exists.
- `backend/servers/api-server/src/lib.rs:333` — router mounted at `/api/v1/disputes`; routes registered at `disputes.rs:37-38` as `GET /` and `GET /statistics`.

## Files
- `backend/servers/api-server/src/routes/disputes.rs:367`
- `backend/servers/api-server/src/routes/disputes.rs:387`
- `backend/crates/db/src/repositories/dispute.rs:120`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Create two orgs (A, B) with at least one dispute each; obtain an auth token for a member of org A.
2. Request `GET /api/v1/disputes/?organization_id=<org_B_uuid>` with the org-A token.
3. Expected: 403 FORBIDDEN or the response filtered to org A's rows only. Actual: 200 OK returning org B's full dispute list (titles + descriptions + party counts). Same with `GET /api/v1/disputes/statistics?organization_id=<org_B_uuid>` returning org B's stats.

## Suggested approach
1. In `routes/disputes.rs`, rename the two `_user: AuthUser` parameters to `user: AuthUser`.
2. Remove `organization_id` from `ListDisputesQuery` and from `OrgQuery` (or deprecate them if other handlers still consume them — grep confirms only these two handlers use `organization_id` from those structs).
3. Derive the org inside each handler: `let organization_id = require_org(&user)?;` mirroring `get_kpis` (`disputes.rs:415-420`).
4. Call `state.dispute_repo.list(organization_id, (&query).into())` and `state.dispute_repo.get_statistics(organization_id)` with the JWT-derived value.
5. Add an integration test in `backend/servers/api-server/tests/suites/` that creates two orgs and asserts the cross-tenant request is rejected (403) or returns an empty list — never org B's rows.
6. Regenerate the OpenAPI spec + TypeScript client (`just typespec` + `pnpm --filter @ppt/api-client build`) if the request shape changes.

## Alternatives considered
- **Add row-level security on the management pool and switch dispute_repo to RlsConnection.** — rejected because flipping dispute_repo to RLS is a cross-cutting repo refactor (every call site in `disputes.rs` + related repos would need `rls.conn()` plumbing); the handler-level fix is a one-file surgical change that closes the IDOR immediately, with the RLS migration pursuable as a separate hardening track.
- **Keep `organization_id` in the query but validate `user.tenant_id == query.organization_id`.** — rejected because clients still supply the value and the handler path remains error-prone (one missed check reopens the IDOR); dropping the client-supplied field is the simpler contract.

## Root-cause trace
1. Symptom: `GET /api/v1/disputes/?organization_id=<other_org>` returns another tenant's dispute list.
2. ← `list_disputes` at `routes/disputes.rs:367-384` passes `query.organization_id` to the repo without checking it against `_user.tenant_id`.
3. ← `DisputeRepository::list` at `crates/db/src/repositories/dispute.rs:120-168` runs on the non-RLS pool with no org filter beyond the one the handler passes in.
4. Origin: handler was authored without the `require_org(&user)?` pattern that `get_kpis` (same file, same router) uses — the plural-list and statistics endpoints predate the KPI endpoint's hardening.

## Test plan
- [ ] Integration test: two-org fixture, assert `GET /api/v1/disputes/?organization_id=<other_org>` from org-A caller returns rows scoped to org A (or 403), never org B's rows. File `backend/servers/api-server/tests/suites/disputes_cross_tenant_idor_tests.rs`.
- [ ] Integration test: same shape for `GET /api/v1/disputes/statistics?organization_id=<other_org>` — statistics must reflect caller's org only.
- [ ] Fail-before verification (IG3): the regression test must fail on the current branch (`main` snapshot) and pass once the fix lands.
- [ ] Exact command: `cargo test -p api-server --test disputes_cross_tenant_idor_tests`
- [ ] Fast backend check: `cargo clippy --workspace --all-targets -- -D warnings && cargo test -p api-server`

## Out of scope
- Full RLS rollout for the dispute repo (tracked separately as a hardening track).
- Rate limiting of dispute listing endpoints.
- Pagination semantics changes.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-disputes-idor.md`
- Mark the matching `backlog.json` row as `status: "done"`
