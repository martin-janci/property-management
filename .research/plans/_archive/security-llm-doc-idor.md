# security-llm-doc-idor

**Vector:** security
**Score:** 3
**Source:** code-review api-core 2026-05-29 · ai.rs:2620 / ai.rs:2599 / ai.rs:2847
**Confidence:** high

## Hypothesis
Three Epic-64 LLM-document handlers in `routes/ai.rs` bind `_principal: RequestPrincipal` and discard it, then call repository methods that run tenant-blind SQL keyed only on a path UUID. The worst is `publish_description`, a state-mutating cross-tenant IDOR: any authenticated user can publish (make public) another tenant's generated listing description by enumerating its UUID. `list_listing_descriptions` and `get_photo_enhancement` are cross-tenant reads. The smallest fix is to bind the principal, resolve its tenant/org, and add an org/owner predicate to the three repo queries (or filter in the handler), mirroring the already-shipped equipment/voice-device IDOR fixes.

## Evidence
- `backend/servers/api-server/src/routes/ai.rs:2620` — `publish_description(State, _principal: RequestPrincipal, Path(id))` discards the principal and calls `llm_document_repo.publish_description(id)`; wired `POST /api/v1/ai/llm/listing/descriptions/{id}/publish` in `llm_router()` (mounted `lib.rs:231` + `main.rs:568`).
- `backend/crates/db/src/repositories/llm_document.rs:397` — `UPDATE generated_listing_descriptions SET is_published = TRUE WHERE id = $1 RETURNING *` has no org/owner/tenant predicate (state-mutating cross-tenant write).
- `ai.rs:2599` `list_listing_descriptions` discards `_principal`; `llm_document.rs:358` `SELECT * FROM generated_listing_descriptions WHERE listing_id = $1` is tenant-blind (cross-tenant read).
- `ai.rs:2847` `get_photo_enhancement` discards `_principal`; `llm_document.rs:977` `SELECT * FROM photo_enhancements WHERE id = $1` is tenant-blind (cross-tenant read).
- Sibling handlers in the same file call `require_tenant_id`; this Epic-64 LLM-document cluster does not. Distinct from in-flight PR #725 (maintenance/chat-session/sentiment IDOR) and the done equipment/voice-device clusters.

## Files
- `backend/servers/api-server/src/routes/ai.rs`
- `backend/crates/db/src/repositories/llm_document.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** no C4/C5 → cloud-ok.

Mode: cloud-ok

## Repro steps
1. As tenant A, generate a listing description: `POST /api/v1/ai/llm/listing/description` and note the returned description `id` (and its `listing_id`).
2. As an unrelated tenant B (different org, valid JWT), call `POST /api/v1/ai/llm/listing/descriptions/{id}/publish` with tenant A's description id.
3. Expected: `404 NOT_FOUND` (B may not publish A's description). Actual (on `dev`): `200 OK` with `is_published = TRUE` — B published A's description (cross-tenant write). Same pattern for `GET /listing/descriptions/{listing_id}` and the photo-enhancement read returning another tenant's rows.

## Suggested approach
1. In `routes/ai.rs`, replace `_principal` with `principal` in `publish_description`, `list_listing_descriptions`, and `get_photo_enhancement`; resolve the tenant/org id via the same helper the sibling handlers use (`require_tenant_id(&principal)` or equivalent).
2. Add an org/owner predicate to the three `llm_document.rs` queries: `publish_description(id, org_id)` → `UPDATE … WHERE id = $1 AND organization_id = $2 RETURNING *`; `list_listing_descriptions(listing_id, org_id)` and `find_photo_enhancement(id, org_id)` → add `AND organization_id = $2` (confirm the column name on `generated_listing_descriptions` / `photo_enhancements`; if no direct org column, join through the owning listing/equipment as the equipment fix did).
3. Return `404 NOT_FOUND` when the scoped query matches no row (do not leak existence) — `publish_description` already maps `Ok(None) -> 404`, so the scoped `UPDATE … RETURNING` naturally yields that path.
4. Run `backend/scripts/lints/check-discarded-principal.sh` (if present) to confirm no remaining `_principal` discards in the touched handlers.
5. Add the regression test in the Test plan; verify it fails on `dev` and passes after the fix.

## Alternatives considered
- **Middleware-level RLS only** — rejected because these queries run on `self.pool` (not an RLS-scoped connection), so row-level security is not enforced on this path; the predicate must be explicit in the query like the equipment/voice-device fixes.
- **Filter in the handler after fetching** — rejected because `publish_description` mutates before any ownership check; post-fetch filtering still performs the cross-tenant write. The predicate must be in the SQL `WHERE`.

## Root-cause trace
1. Symptom: tenant B publishes/reads tenant A's LLM-generated listing description / photo enhancement.
2. ← `routes/ai.rs:2620/2599/2847` bind `_principal` and discard it, passing only the path id to the repo.
3. ← `crates/db/src/repositories/llm_document.rs:397/358/977` run SQL keyed solely on `id`/`listing_id` with no tenant predicate, on `self.pool` (no RLS scoping).
4. Origin: Epic 64 (Stories 64.2/64.4) LLM-document handlers added without the tenant-scoping convention the rest of `ai.rs` follows (`require_tenant_id`); same omission class as the equipment IDOR (PR fixed) and voice-device IDOR (PR #461).

## Test plan
- [ ] Backend integration test: tenant A creates a generated listing description; tenant B's `POST /api/v1/ai/llm/listing/descriptions/{id}/publish` returns 404 and the row's `is_published` stays FALSE; A's own publish returns 200.
- [ ] Cross-tenant read scenario: B's `GET /api/v1/ai/llm/listing/descriptions/{listing_id}` (A's listing) returns no rows; B's photo-enhancement GET for A's id returns 404.
- [ ] Command: `cargo test -p api-server idor` (or place under `backend/servers/api-server/tests/`, mirroring `dispute_cross_org_idor_tests.rs`).

## Out of scope
- The `ai.rs` module-split refactor (tracked separately as `refactor-ai-rs-module-split`).
- The maintenance/chat-session/sentiment IDOR cluster being fixed in PR #725.

## After-merge
- Move this file to `plans/_archive/security-llm-doc-idor.md`
- Mark the matching `backlog.json` row as `status: "done"`
