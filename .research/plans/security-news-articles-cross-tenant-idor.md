# security-news-articles-cross-tenant-idor

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review 2026-06-17 api-handlers
**Confidence:** medium

## Hypothesis
Seven `news_articles.rs` handlers (`get_article`, `delete_article`, `pin_article`, `add_media`, `delete_media`, `list_media`, `record_view`) destructure `TenantExtractor(_tenant)` then discard it. The repository methods they call (`news_article::find_by_id_with_details`, `delete`, `set_pinned`, `add_media`/`delete_media`/`list_media`, `record_view`) execute raw SQL keyed on `article_id` alone — no `organization_id` filter, no RLS gate at the repo. Any authenticated caller from tenant A who guesses (or brute-forces) an article UUID can read, delete, pin/unpin, attach/detach media, or pollute view-stats on tenant B's news articles. Tighten the queries to include `organization_id` (or migrate the repo to an RLS-routed pool) and require the handlers to use the tenant context, not throw it away.

## Evidence
- `backend/servers/api-server/src/routes/news_articles.rs:363` — `get_article(TenantExtractor(_tenant), …)` calls `repo.find_by_id_with_details(id)` — `_tenant` discarded
- `backend/servers/api-server/src/routes/news_articles.rs:685` — `delete_article` calls `repo.delete(id)` which executes `DELETE FROM news_articles WHERE id = $1` (db/src/repositories/news_article.rs:247)
- `backend/servers/api-server/src/routes/news_articles.rs:720` — `pin_article` calls `repo.set_pinned(id, …)` (news_article.rs:255) with no org filter
- `backend/servers/api-server/src/routes/news_articles.rs:767,800,750` — `add_media` / `delete_media` / `list_media` discard `_tenant`; repo methods at `news_article.rs:279/322/313` operate on `article_id` / `media_id` directly
- `backend/servers/api-server/src/routes/news_articles.rs:1030` — `record_view` discards `_tenant`; `news_article.rs:555` INSERTs into `article_views` keyed by `article_id` only — view-stats pollution across tenants

## Files
- `backend/servers/api-server/src/routes/news_articles.rs`
- `backend/crates/db/src/repositories/news_article.rs`

## Dependencies
<!-- none -->

## Required capabilities
- [x] C1 — Systematic debugging (security-IDOR class)
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. As tenant A (manager role), create a news article. Capture its `article_id`.
2. As tenant B (any authenticated user — manager or owner) of an unrelated organization, call `GET /api/v1/news-articles/{article_id}` with B's JWT and B's `X-Tenant-ID` header.
3. Expected: 404 (or 403). Actual today: 200 with tenant A's article body.
4. Same for `DELETE /api/v1/news-articles/{article_id}` — expected 404/403, actual 204 (article gone for A).
5. Repeat for `pin_article`, `add_media`, `delete_media`, `list_media`, `record_view` — each currently writes/reads cross-tenant.

## Suggested approach
1. **Add `organization_id` to every repo method.** Update signatures at `news_article.rs:74,247,255,279,313,322,555` (and any others called by the listed handlers) to take `org_id: Uuid` and append `AND organization_id = $N` to each SQL statement (`SELECT`/`UPDATE`/`DELETE`/`INSERT`). Mirror the `announcement.rs` pattern (`announcement.rs:391,496,729,805,1036` all filter on `organization_id`).
2. **Rebind handlers to use the extracted tenant.** In each of the 7 handlers in `news_articles.rs`, rename `TenantExtractor(_tenant)` → `TenantExtractor(tenant)` and pass `tenant.organization_id` into the repo call.
3. **Map the “no row matched the org filter” result to 404 NotFound** (don't leak existence). The existing repo returns `Result<bool, SqlxError>` for `delete`/`set_pinned`; treat `Ok(false)` as 404 in the handler, same as a missing row.
4. **Add cross-tenant IDOR integration tests** under `backend/servers/api-server/tests/news_articles_cross_org_idor_tests.rs` mirroring `form_cross_org_idor_tests.rs` and `appeal_cross_org_idor_tests.rs`: two tenants, B asserts 404 on every endpoint, A still works.
5. **`grep -n TenantExtractor(_tenant) backend/servers/api-server/src/routes/news_articles.rs` after the fix → 0 hits.**
6. **Update OpenAPI sample responses** if any examples show cross-tenant fields (unlikely — most handlers don't have explicit examples).
7. **Optional follow-up (not in scope):** migrate the repo onto the RLS-routed pool used by `forms.rs` / `appeals.rs` so the filter becomes belt-and-braces (defense-in-depth). Tracked as a separate refactor if the IDOR fix lands first.

## Alternatives considered
- **Migrate `news_article` repo onto the RLS-routed pool right away** — rejected because the RLS migration is a much larger change (mod-pool routing, sqlx offline data regen, new connection acquire/release paths) and would block the IDOR fix on RLS-rollout schedule. The explicit `organization_id` filter is the minimal correct fix today; RLS routing is the right *belt-and-braces* layer to add next.
- **Reject the request at the extractor layer (make `TenantExtractor` infallible-or-bound)** — rejected because other handlers in this file (e.g. listing endpoints) legitimately scope by tenant via `tenant.organization_id`; the bug is that *these* handlers receive the tenant and silently throw it away. Fixing the call sites is the targeted fix; restructuring the extractor would touch unrelated handlers.

## Root-cause trace
1. Symptom: `GET /api/v1/news-articles/{id}` returns another tenant's article when authenticated as tenant B.
2. ← `routes/news_articles.rs:363` handler discards `_tenant`, calls `repo.find_by_id_with_details(id)` with no tenant context.
3. ← `db/src/repositories/news_article.rs:74` SELECT keyed only on `id`, no `organization_id` predicate.
4. Origin: the handler set was introduced (or refactored) without the tenant-filter pattern that `announcement.rs` uses. Likely pre-dates the multi-tenant hardening sprint that landed `forms.rs` / `appeals.rs` org gates (#1397, BIT-73 #1500/#1457). Specific origin commit can be pinpointed via `git blame` on `news_article.rs:74,247,255,555`.

## Test plan
- [ ] `backend/servers/api-server/tests/news_articles_cross_org_idor_tests.rs` — new file, 7 cross-tenant IDOR cases, one per handler. Asserts 404 for tenant-B caller on tenant-A article.
- [ ] Regression: `cargo test -p api-server --test news_articles_cross_org_idor_tests` red on `main`, green on the fix branch.
- [ ] `cargo test -p api-server news_articles` — broader smoke check that the org-filter additions didn't break the existing tests.
- [ ] `cargo clippy -p db --all-targets -- -D warnings` — repo SQL changes pass clippy.

## Out of scope
- RLS-routed pool migration for `news_article` repo (separate refactor — see Suggested approach #7).
- Comments / reactions sub-routes on news articles unless they're called by the same handler chain — check during implementation; if they share the IDOR vector, add to this PR; otherwise file a follow-up.
- Audit-log emission for the IDOR vector pre-fix (the PR closes the gap; backfill audit is a separate decision).

## After-merge
- Move this file to `plans/_archive/security-news-articles-cross-tenant-idor.md`
- Mark the matching `backlog.json` row (`code-review-api-handlers-news-cross-tenant-idor`) as `status: "done"`
