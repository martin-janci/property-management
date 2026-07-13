# security-calevent-create-idor

**Vector:** bug
**Score:** 3
**Source:** code-review api-core 2026-07-13 (signal `code-review-api-core-sync-calevent-create-idor`); PR #2274 (adjacent auth-refactor churn)
**Confidence:** medium

## Hypothesis
`create_calendar_event` in `api-server` accepts a request-body `connection_id` and inserts a `calendar_events` row for it without verifying that the connection belongs to the caller's org. The path-segment `{id}` is discarded (`Path(_path): Path<ResourceIdPath>`), so the handler cannot even cross-check body vs URL. Every sibling handler in the same module load-then-`verify_org_access` on the resource's `organization_id`; only this one doesn't. Because no RLS policy exists on `calendar_events`, the `RlsConnection` does not backstop the missing check either — an authenticated user in org A can POST an event bound to org B's connection.

## Evidence
- `backend/servers/api-server/src/routes/integrations/sync.rs:991` — `create_calendar_event` signature: `Path(_path): Path<ResourceIdPath>`; body carries `data.connection_id`; no `verify_org_access(...)` call before the repo write
- `backend/servers/api-server/src/routes/integrations/sync.rs:489-570-643-734` — sibling handlers `get_calendar_connection`, `update_calendar_connection`, `delete_calendar_connection`, `sync_calendar_events` all load the resource then call `verify_org_access(state, headers, resource.organization_id)`
- `backend/crates/db/src/repositories/integration.rs:374` — repo insert binds `data.connection_id` verbatim, no org/tenant filter on the calling side
- `backend/crates/db/migrations/` — grep for `calendar_events` policies returns no `CREATE POLICY … ON calendar_events …` — the table appears only in INSERT statements
- `signals/2026-07-13.json` (this run) — expert=rust, segment=api-core, confidence=medium

## Files
- `backend/servers/api-server/src/routes/integrations/sync.rs`
- `backend/crates/db/src/repositories/integration.rs`

## Dependencies
(none — self-contained; sibling `list_calendar_events` read-IDOR is tracked separately as `code-review-api-core-sync-calevent-list-idor` and stays in backlog)

## Required capabilities
- [x] C1 — Systematic debugging (IDOR / cross-org write)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security-adjacent — reviewer will want independent thinking on the fix scope)

Mode: cloud-ok

## Repro steps
1. Seed two orgs A and B, each with one authenticated user and one `calendar_connections` row (`connA`, `connB`) tagged to its own org.
2. Sign in as user-A. `curl -X POST /api/v1/integrations/calendars/{connA.id}/events` with a JSON body whose `data.connection_id = connB.id`, minimal valid event payload.
3. Expected: 403 (or 404 to avoid probing) — the handler must refuse a body `connection_id` not owned by org A.
4. Actual: 200 / 201 — a `calendar_events` row is written for `connB` under user-A's session.

## Suggested approach
1. In `create_calendar_event` (`sync.rs:991`), stop discarding the path id: read `Path(path): Path<ResourceIdPath>` and treat `path.id` as the authoritative connection id.
2. Load the connection via the repo (`get_calendar_connection` equivalent) and call `verify_org_access(&state, &headers, connection.organization_id)` before the write — the same three-line preamble every sibling uses.
3. Reject any request whose `data.connection_id` differs from the resolved `path.id` (400 `INVALID_REQUEST` / `CONNECTION_MISMATCH`) so the URL and body cannot disagree.
4. Add a defence-in-depth migration: `CREATE POLICY calendar_events_org_isolation ON calendar_events USING (connection_id IN (SELECT id FROM calendar_connections WHERE organization_id = current_setting('app.current_org_id')::uuid))` (or the equivalent already in use for sibling tables). Keep the migration in a follow-up commit inside the same PR so RLS backstops the handler-level fix without expanding scope.
5. Update the existing `sync.rs` module test module (or `tests/integration_calendar_events.rs`) with the failing repro and a positive control.
6. `cargo fmt -p api-server`; `cargo clippy -p api-server -- -D warnings`; run the crate test lane.

## Alternatives considered
- **RLS-only fix (skip handler-level `verify_org_access`)** — rejected because the module convention is *both* handler-level and RLS-level defence-in-depth; a lone RLS policy would still leak error semantics (a distinct 500 vs 403 discloses org boundary) and diverges from every sibling handler.
- **Reject requests where path-id is discarded (surface today's bug as a compile-time signature change)** — rejected because it would break the public API contract for existing clients that omit or ignore the body `connection_id`; the safer path is to *use* the path id as authoritative and reject conflicting bodies.

## Root-cause trace
1. Symptom: authenticated user in org A can create a `calendar_events` row against org B's connection.
2. ← immediate cause at `backend/servers/api-server/src/routes/integrations/sync.rs:991`: handler ignores `path.id` and skips `verify_org_access`; passes `data.connection_id` straight to repo.
3. ← upstream cause at `backend/crates/db/src/repositories/integration.rs:374`: `create_calendar_event` INSERT binds `connection_id` without an org filter — assumes the caller has already authorised.
4. Origin: original `create_calendar_event` handler introduction in the integrations/sync feature branch (the exact commit needs `git log --follow -- backend/servers/api-server/src/routes/integrations/sync.rs` during implementation; every subsequent PR preserved the missing check).

## Test plan
- [ ] Handler-level integration test: two orgs seeded, user-A posts with body `connection_id=connB.id` → expect 403 / 404, no row written (Rust integration test under `backend/servers/api-server/tests/`)
- [ ] Positive control: user-A posts with body `connection_id=connA.id` and matching path → expect 201, row written and observable via `list_calendar_events`
- [ ] Repo-layer RLS regression: run the same INSERT via `RlsConnection(org_A)` against `connB.id` — must fail once the RLS policy lands
- [ ] Command: `cargo test -p api-server --test integration_calendar_events` (or the pre-existing sync-integrations test name — inspect and reuse)

## Out of scope
- The sibling read IDOR (`list_calendar_events`, tracked as `code-review-api-core-sync-calevent-list-idor` in backlog) — score 2, needs a separate promotion pass and its own regression test.
- Non-calendar integrations (`accounting`, `esignature`) — they already load-then-verify per the module convention.
- Broader RLS audit across `backend/crates/db/migrations/` — a separate finding.

## After-merge
- Move this file to `plans/_archive/security-calevent-create-idor.md`
- Mark the matching `backlog.json` row (`code-review-api-core-sync-calevent-create-idor`) as `status: "done"`
- Consider promoting `code-review-api-core-sync-calevent-list-idor` in a follow-up run if the merged fix hasn't already extended `verify_org_access` to the list handler.
