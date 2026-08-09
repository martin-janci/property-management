# code-review-api-handlers-community-unauthenticated-reads

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 rotating expert review 2026-07-23 (api-handlers segment); files `backend/servers/api-server/src/routes/community.rs`
**Confidence:** medium

## Hypothesis
Three read handlers in `backend/servers/api-server/src/routes/community.rs` accept no `RequestPrincipal` extractor and therefore run **unauthenticated**: `get_group` (line 289), `list_posts` (line 392), and `get_item` (line 727). The `/api/v1/community` router is mounted at `lib.rs:292` with only `security_headers`, `TraceLayer`, and `CorsLayer` globally — no auth middleware — so a handler that omits `RequestPrincipal` is reachable by anonymous callers. This contradicts the file's own SECURITY docstring (`community.rs:29-34`, "every handler now goes through RequestPrincipal") and lets an anonymous caller read any tenant's community groups, posts, and marketplace items by UUID. Add `principal: RequestPrincipal` to each handler and gate on the resource's tenant via a lookup-then-check pattern (mirror the existing `verify_building_access` helper at `community.rs:47`).

## Evidence
- `backend/servers/api-server/src/routes/community.rs:289` — `get_group(State, Path)` — no `RequestPrincipal`; leaks any `community_groups` row by UUID.
- `backend/servers/api-server/src/routes/community.rs:392` — `list_posts(State, Path, Query)` — no `RequestPrincipal`; repository `get_group_posts` is `WHERE group_id=$1` with no tenant filter (`repositories/community.rs:241`).
- `backend/servers/api-server/src/routes/community.rs:727` — `get_item(State, Path)` — no `RequestPrincipal`; leaks `marketplace_items` across tenants.
- `backend/servers/api-server/src/lib.rs:292` — router nested at `/api/v1/community`; global layers are only `security_headers`/`TraceLayer`/`CorsLayer` (`lib.rs:451-461`).
- `backend/servers/api-server/src/routes/mod.rs:76` — `pub mod community;` declared, so this is live production code (reachability gate satisfied).

## Files
- `backend/servers/api-server/src/routes/community.rs:289`
- `backend/servers/api-server/src/routes/community.rs:392`
- `backend/servers/api-server/src/routes/community.rs:727`
- `backend/crates/db/src/repositories/community.rs`
- `backend/servers/api-server/tests`

## Dependencies


## Required capabilities
- [x] C1 — Systematic debugging (security bug; trace the auth boundary end-to-end before touching handlers)
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (integration tests hit Postgres via SQLx)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** `cloud-ok`

Mode: cloud-ok

## Repro steps
1. Start the api-server against a two-tenant seeded DB (Tenant A owns community_group `G1`; Tenant B owns community_group `G2`, post `P2`, marketplace_item `I2`).
2. Without an Authorization header (anonymous) call `GET /api/v1/community/groups/{G2}`.
   - Expected: `401 Unauthorized`.
   - Actual: `200 OK` with `G2` body.
3. Without an Authorization header call `GET /api/v1/community/groups/{G2}/posts`.
   - Expected: `401`. Actual: `200` with `P2`.
4. Without an Authorization header call `GET /api/v1/community/items/{I2}`.
   - Expected: `401`. Actual: `200` with `I2`.

## Suggested approach
1. Add `principal: RequestPrincipal` to the three handler signatures at `community.rs:289`, `:392`, `:727`.
2. Wrap the existing repository call in a tenant-scope check. Two possible shapes (pick per handler):
   - Add `_for_org` repository variants (mirrors the `automation.rs` pattern noted in finding evidence): `get_group_for_org(id, org_id) -> Option<...>` returning `None` when the group's tenant doesn't match, then map `None → 404`.
   - Or: fetch the resource, extract its `tenant_id`/`org_id`, and call the existing `verify_building_access`-style helper (`community.rs:47`) to reject cross-tenant reads.
3. For `list_posts`, plumb the tenant filter into the repository — `get_group_posts(group_id, org_id)` — so no post from another tenant's group can be returned even if the group_id is guessed. This is the safer of the two shapes for a list endpoint.
4. Return `404 Not Found` (not `403`) on cross-tenant misses to avoid IDOR-oracle leakage.
5. Update the `#[utoipa::path]` `security(("bearer_auth" = []))` block if the schema needs regenerating; run `just openapi-check` or the equivalent to catch drift.
6. Do NOT alter the write-side handlers in this plan — those are covered by the sibling plan `code-review-api-handlers-community-cross-tenant-idor.md`.
7. Run `cargo test -p api-server community` and the new integration tests below.

## Alternatives considered
- **Move auth to a router-level middleware layer** — rejected because the existing routes/*.rs pattern is per-handler `RequestPrincipal` extraction; adding a layer would either duplicate the check (double-auth on already-scoped routes) or force a wholesale rewrite of the auth model. Follow existing convention.
- **Return an empty response for cross-tenant reads instead of 404** — rejected because it still betrays existence (empty vs "not found" differ in wire behavior); consistent 404 mirrors what happens when the row genuinely doesn't exist.

## Root-cause trace
1. Symptom: anonymous `GET /api/v1/community/groups/{id}` returns the row.
2. ← `community.rs:289` `get_group` signature has no `RequestPrincipal` extractor, so Axum never invokes the JWT extractor for this route.
3. ← `lib.rs:292` mounts `community_router()` under `/api/v1/community` with **no** `auth::require_auth` layer — every route inside relies solely on per-handler `RequestPrincipal` for gating.
4. ← `community.rs:29-34` docstring claims "every handler now goes through RequestPrincipal", but that invariant was never enforced by anything mechanical (no test, no layer, no macro). The three reads slipped through when the module was added.
5. Origin: the community routes were introduced without the same audit sweep that PR #2450 (disputes-IDOR) and PR #2438 (get_document org-scoping) applied to sibling handlers.

## Test plan
- [ ] `backend/servers/api-server/tests/community_auth_tests.rs` — new file. Cases: `get_group_anonymous_401`, `list_posts_anonymous_401`, `get_item_anonymous_401`; `get_group_cross_tenant_404`, `list_posts_cross_tenant_empty`, `get_item_cross_tenant_404`.
- [ ] Regression: the IG3-required failing-on-main test — `get_group_anonymous_401` MUST currently fail (returns 200) and pass after the fix.
- [ ] Command: `cargo test -p api-server --test community_auth_tests -- --nocapture`

## Out of scope
- The **write-side** cross-tenant IDOR on `create_post`/`add_reaction`/`create_comment`/`rsvp_event`/`create_inquiry` — that's tracked separately as `code-review-api-handlers-community-cross-tenant-idor.md`.
- Any repository refactor beyond adding the `_for_org` variants or plumbing `org_id` into the three affected reads.
- Auth-model overhaul (moving to router-level middleware).

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-community-unauthenticated-reads.md`
- Mark the matching `backlog.json` row (`code-review-api-handlers-community-unauthenticated-reads`) as `status: "done"`
