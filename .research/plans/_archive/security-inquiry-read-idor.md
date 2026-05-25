# security-inquiry-read-idor

**Vector:** security
**Score:** 2
**Source:** commit 4714c7f (code-review reality-server 2026-05-23) | inquiries.rs:554
**Confidence:** high

## Hypothesis
The reality-server `mark_as_read` handler accepts an authenticated principal but discards it (`_principal: RequestPrincipal`) and calls `mark_inquiry_read(id)` with only the path UUID. The underlying `UPDATE listing_inquiries ... WHERE id = $1 AND read_at IS NULL` has no `realtor_id` predicate, so any authenticated portal user can flip another realtor's inquiry to `read` by enumerating UUIDs — a cross-account write IDOR. The smallest fix is to scope the update by the calling realtor (mirror the sibling `respond_to_inquiry`/`get_inquiry_for_realtor` pattern) and return 404 when no row is owned.

## Evidence
- `backend/servers/reality-server/src/routes/inquiries.rs:554` — `mark_as_read` binds `_principal: RequestPrincipal` (discarded), calls `state.reality_portal_repo.mark_inquiry_read(id)` with only the path id
- `backend/servers/reality-server/src/routes/inquiries.rs:66` — route wired `PUT /api/v1/inquiries/{id}/read -> mark_as_read` (reachable production route)
- `backend/crates/db/src/repositories/reality_portal.rs:768` — `UPDATE listing_inquiries SET status = 'read', read_at = NOW() WHERE id = $1 AND read_at IS NULL` has no realtor/owner predicate
- `inquiries.rs:523` (`get_inquiry`) and `inquiries.rs:581` (`respond_to_inquiry`) scope by `principal.user_id` via `get_inquiry_for_realtor` / `respond_to_inquiry(id, …)` — `mark_as_read` is the lone unscoped write; not covered by open PR #435 (which defers a separate community-routes IDOR cluster)

## Files
- `backend/servers/reality-server/src/routes/inquiries.rs:554`
- `backend/crates/db/src/repositories/reality_portal.rs:767`

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**

Mode: cloud-ok

## Repro steps
1. Seed two realtors A and B; realtor A has a listing with one inbound inquiry (`listing_inquiries` row owned by A, `read_at IS NULL`).
2. Authenticate as realtor B (a principal with no relationship to A's inquiry).
3. As B, call `PUT /api/v1/inquiries/{A_inquiry_id}/read`.
4. Expected (after fix): `404 Not Found` and A's inquiry stays unread. Actual (today): `204 No Content` and A's inquiry `status` flips to `read`, `read_at` set — a cross-account write.

## Suggested approach
1. Add an owner-scoped repo method in `backend/crates/db/src/repositories/reality_portal.rs` near line 767, e.g. `mark_inquiry_read_for_realtor(&self, id: Uuid, realtor_id: Uuid) -> Result<bool, SqlxError>`, running `UPDATE listing_inquiries SET status = 'read', read_at = NOW() WHERE id = $1 AND realtor_id = $2 AND read_at IS NULL` and returning whether a row was affected (via `result.rows_affected() > 0`).
2. In `mark_as_read` (`inquiries.rs:554`), rename `_principal` to `principal` and call the scoped method with `principal.user_id`.
3. Return `404 Not Found` ("Inquiry not found") when no owned row matched, mirroring `get_inquiry`'s not-found branch; keep `204 No Content` on success.
4. Audit the `get_inquiry` side effect at `inquiries.rs:534` — it also calls the unscoped `mark_inquiry_read(id)`, but only after a scoped `get_inquiry_for_realtor` succeeds, so it is already owner-gated; leave it or switch it to the scoped method for consistency (note in the PR, do not expand scope).
5. Decide whether the old unscoped `mark_inquiry_read` still has callers; if `get_inquiry` is migrated too, remove it to prevent re-introduction.

## Alternatives considered
- **Check ownership in the handler with a separate SELECT then UPDATE** — rejected because it adds a TOCTOU window and a second round-trip; a single `WHERE id = $1 AND realtor_id = $2` UPDATE is atomic and matches the existing `respond_to_inquiry` pattern.
- **Enforce via Postgres RLS policy on `listing_inquiries`** — rejected for this fix because reality-server does not currently pass a per-request RLS connection on this path; an app-level owner predicate is the minimal, in-pattern change. (RLS hardening is a separate, larger vector.)

## Root-cause trace
1. Symptom: realtor B's `PUT /inquiries/{A_id}/read` returns 204 and mutates A's inquiry.
2. ← `mark_as_read` discards the principal and passes only `id` (`backend/servers/reality-server/src/routes/inquiries.rs:556,561`).
3. ← `mark_inquiry_read` UPDATEs by `id` alone with no owner predicate (`backend/crates/db/src/repositories/reality_portal.rs:768`).
4. Origin: the handler/repo pair was authored without the owner scoping its siblings use; introduced with the inquiries routes (no specific regression PR — latent since the endpoint landed).

## Test plan
- [ ] Integration test in the reality-server inquiries suite: realtor B calls `PUT /api/v1/inquiries/{A_inquiry_id}/read` and asserts `404` plus A's inquiry still `read_at IS NULL` (fails on main today — returns 204 and mutates the row).
- [ ] Regression: realtor A marks their own inquiry read → `204` and row updated (positive path still works).
- [ ] Run: `cargo test -p reality-server inquiries` (or the workspace path the suite lives under: `cargo test -p reality-server`).

## Out of scope
- The deferred community-routes IDOR cluster and other items listed in PR #435.
- Broad RLS rollout on reality-server.
- Conversation-message fetch (`InquiryDetailResponse.messages` is still an empty stub) — unrelated to this authorization gap.

## After-merge
- Move this file to `plans/_archive/security-inquiry-read-idor.md`
- Mark the matching `backlog.json` row as `status: "done"`
