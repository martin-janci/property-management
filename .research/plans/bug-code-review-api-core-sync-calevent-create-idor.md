# bug-code-review-api-core-sync-calevent-create-idor

**Vector:** bug
**Score:** 3
**Source:** signal `code-review-api-core-sync-calevent-create-idor` (dispatcher Tier-1d dev-review, api-core segment, 2026-07-13)
**Confidence:** medium

## Hypothesis
`create_calendar_event` in `backend/servers/api-server/src/routes/integrations/sync.rs:991` accepts the connection id from the URL path segment as `Path(_path): Path<ResourceIdPath>` (underscore-prefixed → ignored) and never calls `verify_org_access` on the target resource. The repository INSERT at `backend/crates/db/src/repositories/integration.rs:374` then binds `data.connection_id` straight from the request body with no owner check. Because there is no RLS policy for `calendar_events` in `backend/crates/db/migrations/*.sql` (the table appears only in INSERT queries), the missing verify_org_access is not backstopped at the DB level. Net: an authenticated user in org A can POST to `/api/v1/integrations/calendars/{id}/events` with a body `connection_id` pointing at a connection in org B and successfully write a calendar event into org B's data — cross-org write / IDOR.

## Evidence
- `backend/servers/api-server/src/routes/integrations/sync.rs:991` — `pub async fn create_calendar_event(State, RlsConnection, Path(_path): Path<ResourceIdPath>, Json(data): Json<CreateCalendarEvent>)` — `_path` is discarded; no verify_org_access call in the body
- `backend/servers/api-server/src/routes/integrations/sync.rs:507`,`:570`,`:643`,`:734` — sibling handlers (get/update/delete connection, sync) all load-then-verify the resource's `organization_id` before mutating
- `backend/crates/db/src/repositories/integration.rs:374` — INSERT uses `data.connection_id` directly; no `WHERE organization_id = $x` join or scope check
- `backend/crates/db/migrations/*.sql` — no RLS policy for `calendar_events` found (grep for `CREATE POLICY.*calendar_events` returns empty); table appears in migration 00xxx but no `ENABLE ROW LEVEL SECURITY`
- Signal source: `.research/signals/2026-07-13.json` `code-review-api-core-sync-calevent-create-idor` (dispatcher Tier-1d, expert=rust, score_delta=3, confidence=medium)

## Files
- `backend/servers/api-server/src/routes/integrations/sync.rs:991`
- `backend/crates/db/src/repositories/integration.rs:374`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector, cross-org IDOR)
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (integration test needs Postgres via `stack up pm-local` or `ppt_dev_up`)
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security-adjacent finding — expect careful review)

**Execution mode (auto-derived from the ticks):**

Mode: cloud-ok
(no C4 or C5; Postgres via `ppt-bridge` MCP is sufficient)

## Repro steps
1. Bring up the local stack with two seeded orgs (org A, org B) and one calendar connection per org: `stack up pm-local` (or via ppt-bridge `ppt_dev_up`) and seed via `ppt_seed --orgs 2 --calendar-connections`.
2. As an authenticated user in org A, issue: `POST /api/v1/integrations/calendars/{ORG_A_CONNECTION_UUID}/events` with a JSON body whose `connection_id` field points at **ORG_B**'s calendar connection UUID.
3. Expected (after fix): 403 or 404. Actual (today): 201 Created — the event is written into org B's calendar_events with `connection_id = ORG_B_CONNECTION_UUID`.

## Suggested approach
1. In `create_calendar_event`, remove the underscore prefix on the Path binding (`Path(path): Path<ResourceIdPath>`) and add a `verify_org_access` call on `path.id` (the connection id from the URL) against the caller's org before the INSERT — mirror the shape used by `get_calendar_connection` at `:489/:507`.
2. In `CreateCalendarEvent` request-body deserialization, either (a) drop the `connection_id` field entirely and use `path.id` as the sole source of truth, or (b) validate `data.connection_id == path.id` and reject otherwise with 400. Option (a) is cleaner — the URL already carries the connection id.
3. In `integration.rs:374`'s `create_calendar_event` INSERT, replace the raw `connection_id` bind with `connection_id = (SELECT id FROM calendar_connections WHERE id = $1 AND organization_id = $ORG)` — belt-and-suspenders against future callers that forget the handler-level check.
4. Also fix `list_calendar_events` at `:951` in the same PR — companion finding (`bug-code-review-api-core-sync-calevent-list-idor`, backlog score 2) is the read-side mirror of this write-side IDOR; addressing them together avoids a second review round.
5. Add an RLS policy migration for `calendar_events` — `CREATE POLICY calendar_events_by_org ON calendar_events USING (connection_id IN (SELECT id FROM calendar_connections WHERE organization_id = current_setting('app.current_org_id')::uuid))`. This closes the defence-in-depth gap and matches the pattern used by other integrations tables.
6. Wire the new handler-level check + the RLS policy into `backend/servers/api-server/tests/calendar_events_cross_tenant_idor_tests.rs` (new file). Two `#[sqlx::test]` cases:
   - `create_calendar_event_rejects_cross_org_connection_id` — sets up two orgs with one connection each, authenticates as org A, POSTs with org B's connection_id → expects 403 (or 404 if you prefer to hide existence).
   - `list_calendar_events_rejects_cross_org_connection_id` — companion for the read side.

## Alternatives considered
- **Path-only fix (drop the `connection_id` from the body, use `path.id`)** — rejected as the *sole* fix because it patches the visible surface but leaves the repo INSERT still capable of cross-org writes if a future caller re-introduces the body-driven pattern; the DB-level RLS policy is worth adding too.
- **RLS-only fix (no handler-level `verify_org_access`)** — rejected because RLS alone gives a 500 (INSERT fails on policy) rather than a clean 403, and the handler's shape drifts further from the load-then-verify pattern the rest of the module uses. Keep both layers.

## Root-cause trace
1. Symptom: authenticated user in org A can POST to `/api/v1/integrations/calendars/{ORG_A_ID}/events` with a body `connection_id` in org B and successfully write into org B's `calendar_events`.
2. ← Handler-level cause at `backend/servers/api-server/src/routes/integrations/sync.rs:991` — the `Path(_path)` binding discards the URL segment and no `verify_org_access` is called before the repo INSERT.
3. ← Repo-level cause at `backend/crates/db/src/repositories/integration.rs:374` — INSERT binds `data.connection_id` from the request body with no `WHERE organization_id` scope.
4. ← Schema cause at `backend/crates/db/migrations/*.sql` — no `CREATE POLICY … ON calendar_events` (grep empty). The table participates in RLS by convention but has no policy declared.
5. Origin: PR that added the `create_calendar_event` route (grep suggests early integrations-server work — precise commit sha requires `git log -p -S "create_calendar_event"` on the file; adversarial reader should confirm before merge). The load-then-verify pattern used by sibling handlers was skipped when this route was authored.

## Test plan
- [ ] `backend/servers/api-server/tests/calendar_events_cross_tenant_idor_tests.rs::create_calendar_event_rejects_cross_org_connection_id` — new `#[sqlx::test]`, must FAIL on `dev` today (writes cross-org event) and PASS after the handler + RLS change (403/404 returned, no row written).
- [ ] `backend/servers/api-server/tests/calendar_events_cross_tenant_idor_tests.rs::list_calendar_events_rejects_cross_org_connection_id` — companion for the read-side finding; same failing-on-main / passing-on-branch contract.
- [ ] `cargo test -p api-server --test calendar_events_cross_tenant_idor_tests` — local runner (needs `stack up pm-local` first).
- [ ] `cargo test -p db --test calendar_events_rls_policy_tests` — asserts the new RLS policy rejects cross-org INSERT at the DB layer even if a caller forgets `verify_org_access`.

## Out of scope
- The wider integrations-module RLS audit — this plan closes the calendar_events specific gap; other integration tables should be audited in a separate pass.
- Refactoring the `ResourceIdPath` extractor to reject Path-ignoring handlers at compile time via a lint — worth considering but architectural; a separate `dx` vector, not this bug.

## After-merge
- Move this file to `plans/_archive/bug-code-review-api-core-sync-calevent-create-idor.md`
- Mark the matching `backlog.json` row (`bug-code-review-api-core-sync-calevent-create-idor`) as `status: "done"`
- If the list-idor companion (`bug-code-review-api-core-sync-calevent-list-idor`) is fixed in the same PR, mark it `status: "done"` too with the same PR reference
