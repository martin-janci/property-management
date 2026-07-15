# code-review-api-handlers-missing-authz-article

**Vector:** bug
**Score:** 3
**Source:** dispatcher review — .research/signals/2026-07-15-api-handlers-tier1d.json
**Confidence:** medium

## Hypothesis
Seven `news_articles` handlers (`publish_article`, `archive_article`, `restore_article`, `pin_article`, `delete_article`, `add_media`, `delete_media`) extract `_user: AuthUser` but never gate on `is_manager()`/authorship, unlike the sibling `create_article` (line 446) and `update_article` (line 542) which explicitly enforce `is_manager || is_author`. Post-#2316 (cross-tenant IDOR fix) the repo layer scopes queries by `organization_id`, so this is now a *within-tenant* privilege gap: any authenticated org member — resident, tenant, junior — can publish, archive, restore, pin, delete, or attach/detach media on any article in their own org, bypassing the manager-only authoring model the neighbouring handlers document and enforce. Fix is a one-line RBAC check per handler (mirror lines 446–447 / 542–544) plus regression tests.

## Evidence
- `backend/servers/api-server/src/routes/news_articles.rs:607-770` — `publish_article`, `archive_article`, `restore_article`, `pin_article` accept `_user: AuthUser` but call `repo.<op>(id, tenant.tenant_id, …)` without any RBAC branch.
- `backend/servers/api-server/src/routes/news_articles.rs:710-728` — `delete_article` permanently deletes an article with no role/author check at all; any authenticated org member can destroy any article in their tenant.
- `backend/servers/api-server/src/routes/news_articles.rs:794-851` — `add_media` and `delete_media` both accept `_user: AuthUser` and forward to the repo with zero authorization; media on manager-authored articles can be swapped or removed by any org member.
- `backend/servers/api-server/src/routes/news_articles.rs:446-447,542-544` — pattern the fix must mirror: `let is_manager = user.role.as_ref().map(|r| r.is_manager()).unwrap_or(false); if !is_manager { return Err(forbidden(…)); }`.
- Post-#2316 the repo now filters by `tenant_id`, so a wrong `id` returns 404 rather than crossing orgs — this refined the finding scope from cross-tenant to within-tenant privilege escalation. Signal `code-review-api-handlers-news-articles-idor` is separately closed by #2316.

## Files
- `backend/servers/api-server/src/routes/news_articles.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):** Mode: cloud-ok

## Repro steps
1. Seed a tenant with two memberships: `manager@x` (role `manager`) and `resident@x` (role `resident`).
2. As `manager@x`, `POST /api/v1/news` to create an article, then `POST /api/v1/news/{id}/publish` — succeeds (201/200).
3. As `resident@x`, `POST /api/v1/news/{id}/archive` — expected 403, actual 200 (article archived).
4. As `resident@x`, `DELETE /api/v1/news/{id}` — expected 403, actual 200 (article permanently deleted).
5. As `resident@x`, `POST /api/v1/news/{id}/media` with a URL — expected 403, actual 200 (media attached to a manager-authored article).

## Suggested approach
1. At the top of each affected handler (`publish_article` @607, `archive_article` @642, `restore_article` @676, `pin_article` @745), insert the same 2-line guard used by `create_article` (lines 446–447):
   ```rust
   let is_manager = user.role.as_ref().map(|r| r.is_manager()).unwrap_or(false);
   if !is_manager { return Err(forbidden("Manager role required")); }
   ```
   Rename `_user: AuthUser` → `user: AuthUser` in the same signatures. Prefer a single small helper `require_manager(user)?` in this file if you touch more than four handlers, but do not scatter it into a new crate — keep the change local.
2. For `delete_article` @710 and the two media handlers (`add_media` @794, `delete_media` @831), decide the policy — the sibling `create_article` gate is `is_manager` only, and PRs #2314/#2316 treat these as manager-only ops. Apply the same guard; add a SECURITY comment on `delete_article` referencing this plan.
3. For `update_article` (line 542) the existing check is `is_manager || is_author` — keep that shape. Do NOT tighten it here.
4. Update the two `#[utoipa::path]` `responses` blocks per handler to document `403 FORBIDDEN` (mirroring #2317-era additions in other route files).
5. Return `forbidden(…)` via the existing `api-server` `common::forbidden(msg)` helper (grep for its call sites to confirm signature).

## Alternatives considered
- **Move the guard into an axum middleware / extractor (`ManagerOnly`)** — rejected because the crate has no such extractor today, adding it would ripple 20+ route files and put a cross-cutting refactor on the critical path of a small correctness fix. Do the local guard now; extract when a second file needs it.
- **Enforce authorship in the DB via an `authored_by` column on the repo methods** — rejected because it duplicates the JWT-derived role check into a schema change (migration + backfill) and doesn't cover the media handlers which have no author link.

## Root-cause trace
1. Symptom: A `resident`-role JWT can `POST /api/v1/news/{id}/archive` and receive `200 OK` on a `manager`-authored article.
2. ← `archive_article` @642 destructures `_user: AuthUser` (leading underscore = intentionally unused) and calls `repo.archive(id, tenant.tenant_id).await`.
3. ← `create_article` @446 established the manager gate pattern; `publish/archive/restore/pin/delete/media` handlers were added in the same file over Story 59.4 landing but never adopted the guard — plausibly because they extract `AuthUser` only to satisfy the tenant-extractor ordering and the reviewer took the presence of the extractor as evidence of the check.
4. Origin: introduced during Story 59.4 handler expansion; verified this file (see `git log -p -- backend/servers/api-server/src/routes/news_articles.rs`) for the exact commit that added `publish_article` — the missing guard has been present since first landing.

## Test plan
- [ ] `backend/servers/api-server/tests/news_articles_rbac_tests.rs` (new) — style follows `report_schedule_rbac_tests.rs`: seed one manager JWT + one resident JWT in the same org, one manager-authored article, and assert
      `publish/archive/restore/pin/delete/add_media/delete_media` all `403` for resident and `200` for manager.
- [ ] Regression scenarios (all in the same file):
      - resident-authored article: manager can still `update_article` (existing behaviour preserved).
      - unknown `id`: manager gets `404` (tenant scoping unchanged).
      - unauthenticated: still `401`/`403` from the outer JWT gate (verify test doesn't mask the RBAC branch).
- [ ] IG3 gate: land the failing test *first* on `dev`, confirm it fails, then land the handler fix.
- [ ] Run: `SQLX_OFFLINE=true cargo test -p api-server news_articles_rbac_tests` (add to `.sqlx/` snapshots if needed) and `cargo fmt -p api-server`.

## Out of scope
- Adding an `authored_by` column to `news_articles` (would touch the repo + schema + backfill — see Alternative 2).
- Refactoring role checks into a shared `ManagerOnly` extractor (see Alternative 1).
- The `update_article` handler (already gated correctly at lines 542–544).
- News-articles read-side scoping (already fixed in PR #2316).

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-missing-authz-article.md`
- Mark the matching `backlog.json` row as `status: "done"`
