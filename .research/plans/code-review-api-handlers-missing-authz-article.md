# code-review-api-handlers-missing-authz-article

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review (segment `api-handlers`, 2026-07-15 dispatcher tier1d)
**Confidence:** medium

## Hypothesis
Four state-changing news-article endpoints — `publish_article`, `archive_article`, `restore_article`, `pin_article` — extract `_user: AuthUser` but never check `is_manager()` or authorship, while `create_article` (line 446) and `update_article` (line 542) explicitly gate on `is_manager || is_author`. Worse, `delete_article` at line 710 permanently deletes an article with **no** role/author check, and `add_media` / `delete_media` at lines 794-851 also have zero authorization. Net effect: any authenticated org member can publish, archive, restore, pin, delete, or mutate media on any article in their tenant, silently violating the manager-only authoring model documented in the handler comments for `create_article`. Smallest fix: apply the same `is_manager || is_author` gate to every state-changing handler and consolidate the check into a helper so future article handlers can't drop it.

## Evidence
- `backend/servers/api-server/src/routes/news_articles.rs:607-728` — `publish_article`, `archive_article`, `restore_article`, `delete_article` bodies show only `_user: AuthUser` (leading underscore proves the extractor is discarded) plus a `NewsArticleRepository` call scoped by `tenant_id`; no `is_manager()` / author check anywhere.
- `backend/servers/api-server/src/routes/news_articles.rs:710-728` — `delete_article` calls `repo.delete(id, tenant.tenant_id)` unconditionally, so any tenant member can permanently destroy any article in their org — the most destructive of the missing gates.
- `backend/servers/api-server/src/routes/news_articles.rs:794-851` — `add_media` and `delete_media` accept `user: AuthUser` (no underscore, but still no check) then call `repo.add_media` / `repo.delete_media` gated only by `tenant_id`, so any authenticated member can attach or remove media on any article.
- `backend/servers/api-server/src/routes/news_articles.rs:439-446, 524-542, 1047` — the *shipped* gate pattern is `let is_manager = user.role.as_ref().map(|r| r.is_manager()).unwrap_or(false);` in `create_article` / `update_article` / a third handler at 1047, then a check against author id — used as the reference shape for the fix.
- `backend/servers/api-server/src/routes/mod.rs:91` — `pub mod news_articles;` confirms the module is reachable (dead-code filter cleared); `backend/servers/api-server/src/routes/news_articles.rs:292-300` wires every affected handler into the router, so this is production-hot code, not a compile-only artifact.
- `.research/signals/2026-07-15-api-handlers-tier1d.json` — the tier1d review that surfaced this finding at `score_delta=3`, `confidence=medium`, `candidate_vector=bug`, expert `rust`.

## Files
- `backend/servers/api-server/src/routes/news_articles.rs:607`
- `backend/servers/api-server/src/routes/news_articles.rs:642`
- `backend/servers/api-server/src/routes/news_articles.rs:676`
- `backend/servers/api-server/src/routes/news_articles.rs:710`
- `backend/servers/api-server/src/routes/news_articles.rs:745`
- `backend/servers/api-server/src/routes/news_articles.rs:794`
- `backend/servers/api-server/src/routes/news_articles.rs:831`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector — trace the authz gap through the repository layer to confirm the tenant-only scoping is not itself sufficient)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Start the api-server test harness: `cd backend && cargo test -p api-server --test news_articles_authz_tests -- --nocapture` (test file to be created — currently a coverage gap, part of IG3).
2. Seed two users in one org: `manager` (role: manager) and `resident` (role: resident); seed one article authored by `manager` (`article_id = A`).
3. As `resident`, issue `DELETE /api/v1/news/A` with the resident's JWT.
4. Expected (after the fix): `403 Forbidden` with `ErrorResponse.code = "FORBIDDEN"` or similar, and `SELECT * FROM articles WHERE id = A` still returns the row.
5. Actual on `dev` today: `200 OK`, and `SELECT * FROM articles WHERE id = A` returns `0` rows — the article is gone. Reproduce equivalently for `POST /api/v1/news/A/publish`, `/archive`, `/restore`, `/pin`, `POST /api/v1/news/A/media`, `DELETE /api/v1/news/A/media/{media_id}`.

## Suggested approach
1. Extract the shipped guard into a private helper local to `news_articles.rs`, e.g. `fn ensure_manager_or_author(user: &AuthUser, article: &NewsArticle) -> Result<(), ApiError>` — takes the same `is_manager = user.role.as_ref().map(|r| r.is_manager()).unwrap_or(false)` computation used at lines 446 / 542 / 1047 and pairs it with an `author_id`/`user.id` check, returning `ApiError::forbidden("Not authorized to modify this article")` on failure. Keeps the pattern already merged, doesn't invent a new authorization primitive.
2. Rebind `_user` → `user` in `publish_article` (607), `archive_article` (642), `restore_article` (676), `delete_article` (710), `pin_article` (745, if it also lacks the check) — the underscore-prefix hides the extractor from clippy today; unclipping is required so the guard can read `user.role` / `user.id`.
3. In each handler, fetch the article first (`repo.get(id, tenant.tenant_id)`) to get the `author_id`; if `Ok(None)` return `not_found`; if `Ok(Some(article))` run the helper; only if the helper returns `Ok(())` proceed to the mutation. This is the exact ordering `update_article` uses today at line 542; mirror it.
4. For `add_media` (794) and `delete_media` (831), the same pattern applies — fetch the article, run the helper, then mutate. Do NOT allow "any authenticated member can add media" — the manager/author gate is the shipped intent per `create_article`'s handler comment.
5. Add a small integration-test harness `backend/servers/api-server/tests/news_articles_authz_tests.rs` (mirrors the existing `agencies_authz_tests.rs` / `ai_automation_batch2_tests.rs` shape) — for each affected method, one positive (manager, 200 / 204) + one negative (resident, 403) + one author-non-manager positive (author, 200) case. Uses `#[sqlx::test(migrator = "db::MIGRATOR")]` with `TestApp` + `TestUser` helpers already present in `tests/common/mod.rs`.
6. Confirm the test set fails against `main` before the fix (IG3) and passes after. Run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` — the underscore-rebind will surface unused-variable clippy hits that must be resolved.

## Alternatives considered
- **Add a route-level middleware that gates the whole `/news/*` router on `is_manager`** — rejected because `update_article` and `create_article` deliberately allow authorship as an alternative to the manager role. A blanket middleware would break the author path (residents who legitimately authored an article could no longer edit it). The per-handler helper preserves the shipped `is_manager || is_author` semantics.
- **Move the check into `NewsArticleRepository` (repo-level gate)** — rejected because the repository layer today is intentionally tenant-scoped only (it accepts a `tenant_id` and enforces nothing else); pushing per-user authz into repos would spread the concern across every mutation method and diverge from the pattern used by every other resource (`update_article` calls the helper in the handler). Keeps the seam explicit at the HTTP layer.

## Root-cause trace
1. Symptom: any authenticated org member can delete/publish/archive/restore/pin/media-edit any article in their tenant via the news routes.
2. ← `news_articles.rs:710-728` (`delete_article`) issues `repo.delete(id, tenant.tenant_id)` with no role/author check; `_user: AuthUser` is extracted but discarded.
3. ← `news_articles.rs:607-704` (`publish_article`, `archive_article`, `restore_article`) each extract `_user: AuthUser` and mutate the article state through the tenant-scoped repo, but skip the `is_manager || is_author` guard that `update_article` at 542 uses.
4. ← `news_articles.rs:794-851` (`add_media`, `delete_media`) accept `user: AuthUser` but never look at `user.role` or the article's `author_id`.
5. Origin: when the news module was extended past the CRUD create/update pair, later handlers were written from the create/update template but the authorization guard was dropped, likely because the state-transition handlers looked "small" (just a status flip) and the reviewer relied on the `_user` prefix as a signal that authz was intentional — it wasn't. The lack of a `news_articles_authz_tests.rs` file (compare `agencies_authz_tests.rs`) let the gap sail through review.

## Test plan
- [ ] `backend/servers/api-server/tests/news_articles_authz_tests.rs` — new file; per affected handler (delete / publish / archive / restore / pin / add_media / delete_media), assert (a) resident → 403, article unchanged; (b) manager → 200/204, mutation applied; (c) author-non-manager → 200/204, mutation applied. Uses `#[sqlx::test(migrator = "db::MIGRATOR")]`.
- [ ] Extend `backend/servers/api-server/tests/news_articles_happy_path_tests.rs` (or the equivalent already-present suite) with a regression assertion that the shipped `update_article` path is unchanged — guard against helper refactor accidentally tightening the `is_author` half.
- [ ] Exact commands: `cd backend && cargo test -p api-server --test news_articles_authz_tests`; then the full workspace `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`.

## Out of scope
- Reworking the news domain model or any `NewsArticleRepository` method signatures — the fix is a handler-layer authz gap; repository changes belong to a separate refactor if desired.
- Extending the gate to *read* handlers (`get_article`, `list_articles`) — those are tenant-scoped-only by design (all members can read their org's articles). This plan only closes the state-changing gaps.
- Auditing every other route module for the same pattern — do the news module first (it's the surfaced finding); a broader sweep is a separate follow-up if the same class shows up elsewhere.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-missing-authz-article.md`
- Mark the matching `backlog.json` row (`code-review-api-handlers-missing-authz-article`) as `status: "done"` with an evidence line `"resolved: PR #<N> merged YYYY-MM-DD — <title>"`.
