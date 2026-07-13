# code-review-api-core-sync-calevent-create-idor

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review api-core 2026-07-13
**Confidence:** medium

## Hypothesis

`create_calendar_event` in `routes/integrations/sync.rs` accepts `POST /api/v1/integrations/calendars/{id}/events`, ignores the `{id}` path segment (bound as `Path(_path)`), and inserts the row using `data.connection_id` from the request body with no ownership check. Every sibling handler in the same module (`get_calendar_connection`, `update_calendar_connection`, `delete_calendar_connection`, sync trigger) load-then-verify with `verify_org_access` on the resource's `organization_id`. Because `calendar_events` has no RLS policy in migrations, an authenticated user in org A can create a calendar event scoped to a connection owned by org B — cross-org write / classic IDOR. Smallest fix: load the target `calendar_connection` by `data.connection_id`, call `verify_org_access` against its `organization_id`, then insert.

## Evidence

- `backend/servers/api-server/src/routes/integrations/sync.rs:991` — handler signature binds `Path(_path): Path<ResourceIdPath>`; the path id is discarded and never joined against the body-supplied `connection_id`
- `backend/servers/api-server/src/routes/integrations/sync.rs:734` — sibling `sync_calendar` demonstrates the correct pattern (load connection, `verify_org_access`, then act)
- `backend/servers/api-server/src/routes/integrations/sync.rs:507`, `:570`, `:643` — connection get/update/delete siblings all load-then-verify on `organization_id`
- `backend/crates/db/src/repositories/integration.rs:374` — `create_calendar_event` INSERT binds `data.connection_id` directly, no auxiliary org filter
- `backend/crates/db/migrations/*.sql` — grep finds no `CREATE POLICY` for `calendar_events`; the missing handler-level check is therefore not backstopped by RLS

## Files

- `backend/servers/api-server/src/routes/integrations/sync.rs`
- `backend/crates/db/src/repositories/integration.rs`

## Dependencies

## Required capabilities

- [x] C1 — Systematic debugging
- [x] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps

1. Seed two orgs (`org_a`, `org_b`) each with a user (`user_a`, `user_b`) and a `calendar_connection` (`conn_a` in org A, `conn_b` in org B).
2. As `user_a`, `POST /api/v1/integrations/calendars/<any-uuid>/events` with body `{ "connection_id": "<conn_b.id>", "title": "leak", ... }`.
3. Query `calendar_events` as `org_b`. Expected: 0 rows / 403 at the handler. Actual today: a row exists with `connection_id = conn_b.id`, authored by `user_a`.

## Suggested approach

1. In `routes/integrations/sync.rs:991`, replace `Path(_path)` with `Path(path): Path<ResourceIdPath>` (or drop the argument entirely if the URL doesn't need it — but preserve API compatibility).
2. Before delegating to the repo, load the connection: `let conn = integration_repo.get_calendar_connection(data.connection_id).await?;` (or the equivalent existing accessor used by the sibling handlers at :507/:570).
3. Call `verify_org_access(&auth, conn.organization_id).await?;` — mirror the sibling handlers verbatim; do not reinvent the check.
4. If the path `{id}` is meant to represent the connection id (which the sibling handlers imply by URL shape), assert `data.connection_id == path.id` and return `400 Bad Request` on mismatch — this closes the ambiguity that made the IDOR possible.
5. Add an integration test under `backend/servers/api-server/tests/` — pattern-match an existing IDOR test (e.g. `insurance_cross_tenant_idor_tests.rs`, `reserve_funds_cross_org_idor_tests.rs`) which already stand up two orgs and a cross-tenant probe.
6. While in the file, sanity-check `list_calendar_events` at `:951` — that has its own tracked backlog item (`code-review-api-core-sync-calevent-list-idor`, score 2) and is **out of scope for this plan** (see *Out of scope*).

## Alternatives considered

- **Add RLS on `calendar_events` and rely on the RlsConnection alone** — rejected as the fix for this ticket because (a) the module's convention is handler-level `verify_org_access` on top of RLS as defence-in-depth, (b) adding an RLS policy is a migration + backfill risk this PR shouldn't take on, and (c) the sibling handlers would still be inconsistent. RLS is worth adding *in addition*, but as a separate follow-up.
- **Ignore the body `connection_id` and derive it from the path** — rejected because the path binding is currently `_path` (discarded), and the sibling handlers accept the body shape used today; changing the wire contract is a bigger blast radius than needed for a security fix.

## Root-cause trace

1. Symptom: cross-org write to `calendar_events` succeeds when body `connection_id` is set to a foreign-org connection id
2. ← `create_calendar_event` handler at `sync.rs:991` never calls `verify_org_access` and discards the path id (`Path(_path)`)
3. ← the repo function `create_calendar_event` at `integration.rs:374` binds `data.connection_id` directly, no org filter
4. ← `calendar_events` table has no RLS policy in `backend/crates/db/migrations/*.sql`
5. Origin: introduction of the `create_calendar_event` route (likely with the integrations Epic — check `git log --diff-filter=A -- backend/servers/api-server/src/routes/integrations/sync.rs` for the earliest commit at line 991)

## Test plan

- [ ] `backend/servers/api-server/tests/calevent_cross_org_idor_tests.rs` — new integration test, follows `insurance_cross_tenant_idor_tests.rs` shape: seed 2 orgs, POST as `user_a` with `conn_b.id`, assert 403 and 0 rows written to `org_b`'s scope
- [ ] `backend/servers/api-server/tests/calevent_cross_org_idor_tests.rs` — happy-path test: POST as `user_a` with `conn_a.id` → 201, row visible under `org_a`
- [ ] Command: `cargo test -p api-server --test calevent_cross_org_idor_tests`

## Out of scope

- `list_calendar_events` read IDOR at `sync.rs:951` — tracked separately as `code-review-api-core-sync-calevent-list-idor`
- Adding an RLS policy for `calendar_events` — separate migration follow-up
- Sync-connection idempotency / calendar-event dedup — behavioural refactor, not part of this security fix

## After-merge

- Move this file to `plans/_archive/code-review-api-core-sync-calevent-create-idor.md`
- Mark the matching `backlog.json` row as `status: "done"`
- Open a follow-up ticket to add RLS on `calendar_events` (mention this plan as source)
