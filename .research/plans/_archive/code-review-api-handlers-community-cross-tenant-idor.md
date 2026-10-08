# code-review-api-handlers-community-cross-tenant-idor

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 rotating expert review 2026-07-23 (api-handlers segment); files `backend/servers/api-server/src/routes/community.rs`
**Confidence:** medium

## Hypothesis
Five **authenticated** write handlers in `backend/servers/api-server/src/routes/community.rs` accept `RequestPrincipal` but never check that the URL-path resource (`group_id`, `post_id`, `event_id`, `item_id`) belongs to the caller's tenant. Any authenticated user of tenant A can POST/PUT into tenant B's community groups, posts, events, and marketplace items by guessing (or scraping) IDs. The sibling `verify_building_access` helper (`community.rs:47`) is deliberately called on `building_id` routes but never on the resource-id routes; `community_repo` has no `*_for_org` variants at all. Add a lookup-then-check pattern (fetch resource, compare its `tenant_id`/`org_id` against `principal.org_id`, reject with 404) on every affected write. This is the same fix pattern PR #2450 applied to disputes and PR #2438 applied to `get_document`.

## Evidence
- `backend/servers/api-server/src/routes/community.rs:426` — `create_post` — `group_id` from URL path, no `verify_group_tenant`; repository `INSERT` binds `group_id` verbatim (`repositories/community.rs:194`).
- `backend/servers/api-server/src/routes/community.rs:463` — `add_reaction` — `post_id` from URL, no tenant check; `INSERT INTO community_post_reactions` binds `post_id` verbatim (`repositories/community.rs:263`).
- `backend/servers/api-server/src/routes/community.rs:500` — `create_comment` — `post_id` from URL, no tenant check.
- `backend/servers/api-server/src/routes/community.rs:615` — `rsvp_event` — `event_id` from URL, no tenant check; `UPSERT` on `(event_id, user_id)` (`repositories/community.rs:381`).
- `backend/servers/api-server/src/routes/community.rs:762` — `create_inquiry` — `item_id` from URL, no tenant check; sibling `verify_building_access` (`community.rs:47-100`) is called on `building_id` routes but not on `group_id`/`post_id`/`event_id`/`item_id` routes.

## Files
- `backend/servers/api-server/src/routes/community.rs:426`
- `backend/servers/api-server/src/routes/community.rs:463`
- `backend/servers/api-server/src/routes/community.rs:500`
- `backend/servers/api-server/src/routes/community.rs:615`
- `backend/servers/api-server/src/routes/community.rs:762`
- `backend/crates/db/src/repositories/community.rs`
- `backend/servers/api-server/tests`

## Dependencies


## Required capabilities
- [x] C1 — Systematic debugging (security bug; verify each of the five handlers end-to-end)
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (integration tests hit Postgres via SQLx)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** `cloud-ok`

Mode: cloud-ok

## Repro steps
1. Start the api-server against a two-tenant seeded DB. Tenant A has user `UA` (JWT `TA`); Tenant B owns group `G2`, post `P2`, event `E2`, marketplace_item `I2`.
2. As `UA` (Tenant A) POST `/api/v1/community/groups/{G2}/posts` with a valid body.
   - Expected: `403 Forbidden` (or `404 Not Found`).
   - Actual: `201 Created` — post is inserted into Tenant B's group.
3. As `UA` POST `/api/v1/community/posts/{P2}/reactions`.
   - Expected: `403`/`404`. Actual: `201` — reaction lands on Tenant B's post.
4. As `UA` POST `/api/v1/community/posts/{P2}/comments`.
   - Expected: `403`/`404`. Actual: `201` — comment lands on Tenant B's post.
5. As `UA` POST `/api/v1/community/events/{E2}/rsvp`.
   - Expected: `403`/`404`. Actual: `200`/`201` — RSVP lands on Tenant B's event.
6. As `UA` POST `/api/v1/community/items/{I2}/inquiries` with a message body.
   - Expected: `403`/`404`. Actual: `201` — inquiry lands on Tenant B's marketplace item.

## Suggested approach
1. Add three `_for_org` fetch helpers in `backend/crates/db/src/repositories/community.rs`:
   - `get_group_for_org(id, org_id) -> Option<CommunityGroup>`
   - `get_post_for_org(id, org_id) -> Option<CommunityPost>` (returns `None` when the post's group's `tenant_id != org_id`)
   - `get_event_for_org(id, org_id) -> Option<CommunityEvent>`
   - `get_item_for_org(id, org_id) -> Option<MarketplaceItem>`
2. In each write handler, call the matching `_for_org` fetch before the write; map `None → StatusCode::NOT_FOUND` (avoid `403` to prevent IDOR-oracle enumeration).
3. `create_post` (line 426): `get_group_for_org(group_id, principal.org_id)` → 404 or proceed to insert.
4. `add_reaction` (line 463) and `create_comment` (line 500): `get_post_for_org(post_id, principal.org_id)` → 404 or proceed.
5. `rsvp_event` (line 615): `get_event_for_org(event_id, principal.org_id)` → 404 or proceed.
6. `create_inquiry` (line 762): `get_item_for_org(item_id, principal.org_id)` → 404 or proceed.
7. Do NOT touch the read-side handlers in this plan — those are covered by the sibling plan `code-review-api-handlers-community-unauthenticated-reads.md`.

## Alternatives considered
- **Row-Level Security (RLS) via PostgreSQL** — rejected for this plan because the api-server is still mid-migration toward RLS-per-request-connection (see prior `security-rls-migration-residual` items) and the community routes are not yet on the RLS-connection path; a handler-level check ships now without waiting for the RLS rollout.
- **Rely on the caller-supplied `org_id` in the body** — rejected because IDOR fixes must NOT trust request-body scoping; the enforced scope has to come from `principal.org_id` (JWT-derived, server-side).

## Root-cause trace
1. Symptom: authenticated Tenant A user writes into Tenant B's community_group/post/event/item.
2. ← `community.rs:426`/`:463`/`:500`/`:615`/`:762` — five handlers extract `RequestPrincipal` (so they pass auth) but never call any tenant-scoping helper before invoking the repository write.
3. ← `repositories/community.rs:194`/`:263`/`:381` — repository methods bind `group_id`/`post_id`/`event_id`/`item_id` verbatim; no `WHERE tenant_id = $2` filter on the underlying INSERT/UPSERT.
4. ← The file has `verify_building_access` (`community.rs:47-100`) but no equivalent for `group_id`/`post_id`/`event_id`/`item_id` — the helper set is incomplete.
5. Origin: same audit gap as PR #2441 → PR #2450 (disputes-IDOR cluster) — the community module was added before the audit sweep and never revisited.

## Test plan
- [ ] `backend/servers/api-server/tests/community_authz_tests.rs` — new file. Cases: `create_post_cross_tenant_404`, `add_reaction_cross_tenant_404`, `create_comment_cross_tenant_404`, `rsvp_event_cross_tenant_404`, `create_inquiry_cross_tenant_404`.
- [ ] Regression (IG3, must fail on `dev` before the fix): `create_post_cross_tenant_404` — assert an authenticated Tenant A caller POSTing to Tenant B's group receives 404 and no row is inserted (assert-count on `SELECT COUNT(*) FROM community_posts WHERE group_id = $B2$` unchanged).
- [ ] Regression: same-tenant paths (`create_post_same_tenant_201`, etc.) still succeed — no regression to the happy path.
- [ ] Command: `cargo test -p api-server --test community_authz_tests -- --nocapture`

## Out of scope
- The three **read-side** handlers with no `RequestPrincipal` — tracked in `code-review-api-handlers-community-unauthenticated-reads.md`.
- Repository-wide RLS migration (a much larger refactor tracked elsewhere).
- Adding a fresh test file for the same helper structure across other modules — only community handlers here.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-community-cross-tenant-idor.md`
- Mark the matching `backlog.json` row (`code-review-api-handlers-community-cross-tenant-idor`) as `status: "done"`
