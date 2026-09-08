# security-idor-violations-2944

**Vector:** security
**Score:** 3
**Source:** Issue #2944
**Confidence:** high

## Hypothesis
Three violation-read endpoints (`list_evidence`, `list_payments`, `list_comments` in `backend/servers/api-server/src/routes/violations.rs`) call their repository methods without the caller's `org_id`, so any authenticated user can read another org's evidence, payments, and comment threads. Additionally, `list_comments` accepts a caller-supplied `include_internal` query flag and forwards it verbatim, allowing a non-manager caller to reveal internal notes on any violation — an intra-org privilege leak on top of the cross-tenant leak. The smallest fix: thread `org_id` into each repo signature (matching the pattern already used by every other handler in the file — e.g. `get_violation_for_org`, `list_violation_summaries`), and gate `include_internal` on a manager-role check.

## Evidence
- Issue #2944 (bug+security, OPEN 2026-09-06): "violation comments/evidence/payments read without org scoping (+ internal-notes privilege leak)"
- `backend/servers/api-server/src/routes/violations.rs:322` — `list_evidence(violation_id)` — no `org_id`
- `backend/servers/api-server/src/routes/violations.rs:475` — `list_payments(action_id)` — no `org_id`
- `backend/servers/api-server/src/routes/violations.rs:625` — `list_comments(violation_id, query.include_internal.unwrap_or(false))` — no `org_id` and caller-controlled `include_internal`
- Sibling handlers in the same file already thread `org_id` correctly (`:126,151,215,232,250,268,291,313,346,373,390,410`) — the fix stays parallel to the established pattern.

## Files
- `backend/servers/api-server/src/routes/violations.rs`
- `backend/crates/db/src/repositories/violations.rs`

## Required capabilities
- [x] C1 — Systematic debugging (security fix; walk the call chain)
- [x] C2 — Seed data (two tenants each with a violation + evidence + payments + comments)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Seed two orgs A and B, each with a violation V_A / V_B plus one evidence row, one action + payment, and one comment (with `is_internal = true` alongside a public one).
2. Auth as an org-A user; `GET /api/v1/violations/{V_B}/evidence` — expect 404; actual: 200 with org-B evidence.
3. Same caller; `GET /api/v1/violations/{V_B}/actions/{action_id}/payments` — expect 404; actual: 200 with org-B payments.
4. Same caller; `GET /api/v1/violations/{V_A}/comments?include_internal=true` while the caller has a non-manager role — expect internal comments to be filtered; actual: they leak.

## Suggested approach
1. In `violations.rs:322`, resolve `org_id` from `user.tenant_id` (match the pattern at `:103` etc.) and change the repo call to `list_evidence(violation_id, org_id)`.
2. Same at `:475` — `list_payments(action_id, org_id)`.
3. Same at `:625` — `list_comments(violation_id, org_id, include_internal)`.
4. In `backend/crates/db/src/repositories/violations.rs`, add `org_id: Uuid` to those three methods and extend each SQL with `AND organization_id = $N` (evidence, payments) or an `EXISTS (SELECT 1 FROM violations WHERE id = $violation_id AND organization_id = $org_id)` guard for the join-heavy comments query.
5. Gate `include_internal`: resolve caller role from `AuthUser` (e.g. via the same helper `get_violation_for_org` uses); if the caller is not a manager/admin, force `include_internal = false` and note this in the response headers or log line so the reviewer sees the coercion.
6. `cargo sqlx prepare` for the DB crate; make sure the offline data lands.
7. Integration test per handler (see *Test plan*).

## Alternatives considered
- **Route-level middleware that resolves and injects `org_id`** — rejected because the codebase resolves `org_id` inline in every handler; a wrapper would be an isolated pattern shift with no test coverage established for it.
- **Silently drop `include_internal` for non-managers with no error** — rejected because a downstream client that relies on the flag would get confusing "empty internal-notes" responses; better to enforce with a documented coercion and log line so the behavior is auditable.

## Root-cause trace
1. Symptom: cross-tenant reads on violation evidence/payments/comments, plus caller-controlled internal-notes disclosure.
2. ← `list_evidence` (`:322`), `list_payments` (`:475`), `list_comments` (`:625`) drop `org_id` and pass `include_internal` verbatim.
3. ← Repository methods `list_evidence(violation_id)`, `list_payments(action_id)`, `list_comments(violation_id, include_internal)` do not filter by tenant, and there is no role check on the include-internal branch.
4. Origin: the violations sub-tree was built incrementally — write handlers threaded `org_id` from day one; the three list handlers were added later and missed the pattern.

## Test plan
- [ ] `backend/servers/api-server/tests/violations_idor.rs` — new file. Two-tenant setup; asserts (a) list_evidence 404 across tenants, (b) list_payments 404 across tenants, (c) list_comments filters same-tenant + role-gates include_internal.
- [ ] `backend/servers/api-server/tests/violations_idor.rs::include_internal_ignored_for_non_manager` — non-manager caller with `include_internal=true` receives no internal rows.
- [ ] `cd backend && cargo test -p api-server violations_idor` and `cargo test -p db violations`.

## Out of scope
- Write endpoints (`add_evidence`, `add_comment`, etc.) — they already thread `org_id`.
- Refactoring `include_internal` into a request DTO — keep the query-param surface stable so clients don't need to change.
- Any UI/reality-server side; those routes are separate.

## After-merge
- Move this file to `plans/_archive/security-idor-violations-2944.md`
- Mark the matching `backlog.json` row as `status: "done"`
