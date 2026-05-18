# `portal_users` Removal Plan (D1.3 follow-up)

**Status:** *still deferred — re-audited 2026-05-15 (E1.2)*
**Owner:** identity unification track
**Decision date:** 2026-05-15 (Phase 2 D1.3 audit)
**Re-audit date:** 2026-05-15 (Phase 2.5 E1.2 — drop blocked by FK constellation)

## Decision

After D1.1 (`OptionalRequestPrincipal`) and D1.2 (sweep of the 17
reality-server endpoints to `RequestPrincipal` / `OptionalRequestPrincipal`),
the audit found that **`portal_users`, `portal_sessions` and
`portal_password_reset_tokens` cannot yet be safely dropped.**

Multiple write call-sites remain that touch these tables outside (or in
coordination with — but not via) `UnifiedPortalUserRepo`. Most are
intentional dual-write mirrors that the unified path orchestrates; some are
legacy fallbacks that exist to handle the unmerged-collision case; one is
the still-active session storage layer used by the SSO/login flow.

Better to leave the tables in place and document the remaining call-sites
than to drop a table that something writes to in prod.

## Audit results — table by table

### `portal_users`

| File:Line | Call site | Category | Action needed before drop |
|-----------|-----------|----------|---------------------------|
| `backend/crates/db/src/repositories/portal.rs:70` | `PortalRepository::create_user` (INSERT) | Legacy writer; **not called from any production handler** (registration goes through `UnifiedPortalUserRepo::create` via `UserHandler::register`). | Verify no test/CLI tooling depends on it; then delete the method along with the table. |
| `backend/crates/db/src/repositories/portal.rs:162` | `PortalRepository::update_user` (UPDATE) | Called from `UserHandler::update_profile` (`backend/servers/reality-server/src/handlers/users/mod.rs:426`) and from `UserService::upsert_sso_user` (`backend/servers/reality-server/src/state.rs:175`) as the final mirror step **after** the unified write. | When `users` becomes the sole source of truth for profile fields, replace these mirror calls with reads from `users`. |
| `backend/crates/db/src/repositories/portal.rs:190` | `PortalRepository::update_password_hash` (UPDATE) | Called from `UserHandler::confirm_password_reset` (`backend/servers/reality-server/src/handlers/users/mod.rs:615`) as a **fallback** when `unified.update_password_hash` finds no `users` row (the unmerged-collision case). | First close the unmerged-collision backlog (see `user_merge_collisions` queue); then this fallback path can go. |
| `backend/crates/db/src/repositories/portal.rs:861` | `PortalRepository::upsert_sso_user` (INSERT … ON CONFLICT) | Legacy SSO writer; **not called from any production handler** (SSO goes through `UnifiedPortalUserRepo::sso_upsert` via `UserService::upsert_sso_user` / `UserHandler::upsert_sso_user`). | Same as `create_user` — verify no remaining caller, delete with the table. |
| `backend/crates/db/src/repositories/unified_portal_user.rs:133, 216, 267, 468, 506` | `UnifiedPortalUserRepo` (INSERT/UPDATE × 5 sites) | **Authoritative dual-writer.** Goes away when the table is dropped. | Drop these statements as part of the same migration that drops the table. |
| `backend/crates/db/src/seed/factories.rs:391` | `Factories::create_portal_user` (INSERT, test/seed) | Test factory used by integration tests and dev seeds. | Migrate the factory to write to `users` directly with `principal_kind='public'`; no production impact. |
| `backend/crates/db/src/seed/factories.rs:988` | `Factories::cleanup` (DELETE) | Cleanup hook; will continue to work as a no-op once the table is dropped (DELETE on a non-existent table errors — switch to a `DROP TABLE IF EXISTS` style or delete the line). | Update the cleanup helper before the migration runs. |
| `backend/crates/db/tests/portal_user_merge_tests.rs:20` | Test fixture (INSERT) | Verifies the `users` ⇄ `portal_users` merge migration; pre-merge data must exist in `portal_users`. | Re-target the test against a fixture that simulates "pre-Phase-2 schema" instead, or retire the test once Phase 3.0 ships. |

### `portal_sessions`

The session-storage layer for reality-server. The `RequestPrincipal`
extractor now validates JWTs directly via `JWT_SECRET`, but the
`SessionService` still creates a `portal_sessions` row on every login and
the `users::logout` handler still invalidates by token hash. Until JWT
revocation moves to a different mechanism (denylist / short-TTL +
refresh), the table is **load-bearing** and must not be dropped.

| File:Line | Call site | Category |
|-----------|-----------|----------|
| `backend/crates/db/src/repositories/portal.rs:752` | `PortalRepository::create_session` (INSERT) | Active write, called by `SessionService::create_session` and `create_mobile_session`. |
| `backend/crates/db/src/repositories/portal.rs:808` | `PortalRepository::refresh_session` (UPDATE) | Active write, called by `SessionService::refresh_session` (SSO refresh flow). |
| `backend/crates/db/src/repositories/portal.rs:824` | `PortalRepository::delete_session` (DELETE) | Active write, called by `SessionService::invalidate_session` (logout). |
| `backend/crates/db/src/repositories/portal.rs:834` | `PortalRepository::delete_user_sessions` (DELETE) | Used by admin / password-reset flows to bulk-revoke a user's sessions. |
| `backend/crates/db/src/repositories/portal.rs:844` | `PortalRepository::cleanup_expired_sessions` (DELETE) | Periodic cleanup. |

**Removal precondition:** revocation moves to a denylist (Redis) or to a
short-TTL access token + a separate refresh table that is NOT
`portal_sessions`. That is a separate phase decision; not in scope here.

### `portal_password_reset_tokens`

Active write path (D1.2 did not touch the password-reset flow).

| File:Line | Call site | Category |
|-----------|-----------|----------|
| `backend/crates/db/src/repositories/portal_password_reset.rs:33` | INSERT (request reset) | Active. |
| `backend/crates/db/src/repositories/portal_password_reset.rs:69, 85` | UPDATE … SET used_at (mark consumed / invalidate batch) | Active. |
| `backend/crates/db/src/repositories/portal_password_reset.rs:103` | DELETE (cleanup) | Active. |

**Removal precondition:** password-reset tokens migrate to a
`users`-keyed table (e.g. `password_reset_tokens` keyed by `users.id`
instead of `portal_users.id`), the legacy table is back-filled, and the
handlers cut over.

## Phase 3.0 entry criteria

Before opening a "drop portal_users" PR, verify ALL of:

- [ ] `user_merge_collisions` queue is drained (no unmerged rows remain).
      The fallback path at `handlers/users/mod.rs:615` exists *because*
      of unmerged collisions; closing that gap removes the last
      production caller of `PortalRepository::update_password_hash`.
- [ ] `PortalRepository::create_user` and `PortalRepository::upsert_sso_user`
      have no callers outside tests/dev-tooling. Delete the methods.
- [ ] All read paths that join through `portal_users.id` (favorites,
      saved-searches, inquiries, comments, sessions, reset tokens) are
      either re-keyed on `users.id` or accept the back-pointer view.
- [ ] Replace the test factory at `seed/factories.rs:391` with a
      `users`-only factory and update the test cleanup to drop without
      `portal_users` references.
- [ ] Decide what replaces `portal_sessions` (denylist vs new refresh
      table) and ship that change first; otherwise reality-server logout
      becomes a no-op.
- [ ] Replace `portal_password_reset_tokens` with a `users`-keyed
      `password_reset_tokens` table, cut over, and remove the legacy
      table reference.

Only then does a `DROP TABLE portal_users, portal_sessions,
portal_password_reset_tokens` migration become safe.

## Why we are NOT writing the drop migration today

The orchestrator's decision tree said:

> If any non-migrated writes remain: DO NOT drop. Add
> `docs/multitenancy/follow-ups/portal-users-removal-plan.md` listing
> remaining write call-sites with file:line and what each needs.

The audit above documents 8 production write call-sites across the three
tables. Every one of them is intentional and currently load-bearing.
Dropping any of these tables in this PR would break the registration,
login, SSO, password-reset, or logout flow on reality-server. The doc
exists; the migration does not.

## E1.2 re-audit (2026-05-15)

When E1.2 attempted to graduate the write call-sites and ship the
`DROP TABLE` migration in this PR, a deeper audit surfaced a much
larger blocker: **`portal_users.id` is the FK target for ~16 columns
across 9 tables**. Migrating writes alone is not sufficient — every one
of those FK-bearing tables would have to either back-fill its column
with `users.id`, change its FK reference, and have its read paths
re-pointed, *before* `portal_users` can drop without leaving dangling
foreign keys.

### FK constellation blocking the drop

Grep: `grep -rn "REFERENCES portal_users" backend/crates/db/migrations/`

| File | Table.column | FK behavior |
|------|--------------|-------------|
| `00063_reality_portal_professional.sql` | `portal_sessions.user_id` | CASCADE |
| `00063_reality_portal_professional.sql` | `portal_favorites.user_id` | CASCADE |
| `00063_reality_portal_professional.sql` | `portal_saved_searches.user_id` | CASCADE |
| `00063_reality_portal_professional.sql` | `portal_inquiries.user_id` (et al.) | CASCADE |
| `00063_reality_portal_professional.sql` | `portal_alert_subscriptions.user_id` | CASCADE |
| `00063_reality_portal_professional.sql` | `agency_invites.invited_by` | (no action) |
| `00063_reality_portal_professional.sql` | `portal_user_settings.user_id` | CASCADE UNIQUE |
| `00063_reality_portal_professional.sql` | `agencies.realtor_id` | (no action) |
| `00063_reality_portal_professional.sql` | `realtors.realtor_id` | (no action) |
| `00063_reality_portal_professional.sql` | `listing_inquiries.user_id` | SET NULL |
| `00063_reality_portal_professional.sql` | `listing_inquiries.realtor_id` | (no action) |
| `00063_reality_portal_professional.sql` | `portal_import_jobs.user_id` | (no action) |
| `00104_create_portal_password_reset_tokens.sql` | `portal_password_reset_tokens.portal_user_id` | CASCADE |
| `00105_create_compare_lists.sql` | `compare_lists.user_id` | CASCADE |
| `00106_create_listing_reports.sql` | `listing_reports.reporter_user_id` | SET NULL |
| `00107_create_realtor_reviews.sql` | `realtor_reviews.reviewer_user_id` | CASCADE |
| `00109_create_articles.sql` | `reality_articles.author_user_id` | SET NULL |
| `00109_create_articles.sql` | `reality_article_comments.author_user_id` | CASCADE |
| `00132_merge_portal_users_into_users.sql` | `users.portal_origin_id` | SET NULL (the merge back-pointer) |

### Why this matters for the dual-write

The "stop dual-writing to `portal_users`" cleanup the original audit
proposed is **not safe today** because of the constellation above. If
`UnifiedPortalUserRepo::create` and `sso_upsert` stopped writing
`portal_users` rows for newly-registered public users, those users would
have NO `portal_users.id` to be referenced by — favoriting a listing,
saving a search, opening an inquiry, or commenting on an article would
all fail with FK violations. The dual-write *is* the correct behavior
until those nine tables migrate their FKs to `users.id`.

### Read paths that join through `portal_users`

The reality-server still has `LEFT JOIN portal_users` reads for author
display name / avatar in two route modules:

| File:line | Read |
|-----------|------|
| `backend/servers/reality-server/src/routes/articles.rs:159, 258, 390, 480, 494` | Article + comment author display |
| `backend/servers/reality-server/src/routes/agent_reviews.rs:128, 276` | Reviewer display |

These read paths could swap to `JOIN users ON users.id = …` (the
unified table now carries every public user via the merge migration
plus the dual-write), but only after the FK columns themselves are
re-pointed at `users.id`. Doing the read swap without the FK swap is
fine in isolation but does not unblock the drop.

### What was actually shipped in E1.x

* **E1.1** — Skipped. Removing the dual-writes would leave new public
  users without the `portal_users.id` row that nine tables' FKs still
  require. The mirror writes stay.
* **E1.2** — Skipped. Drop blocked by the FK constellation above.
  Migration `00144_drop_portal_users_tables.sql` was NOT created.
* **E1.3** — *Shipped.* `AuthenticatedUser` / `OptionalAuth` removed
  from `extractors::auth`; `extract_session_token` /
  `extract_session_cookie` consolidated as the only legacy auth-path
  surface, used solely by the logout flow.

## Phase 3.0 (or later) — what unblocks the drop

A multi-PR sequence is required, in order:

1. **Re-key the FK-bearing tables.** For each of the 16 columns above:
   add a `<col>_user_id` UUID NOT NULL referencing `users(id)`
   alongside the existing `<col>` column, back-fill it from
   `portal_users.pm_user_id` ⇄ `users.portal_origin_id`, swap the
   application reads/writes, then drop the old column.
2. **Re-key `portal_password_reset_tokens` to `users.id`.** Either
   create a new `user_password_reset_tokens` table or reuse the
   existing `password_reset_tokens` (api-server side already references
   `users.id`; the schemas are identical apart from the column name).
3. **Re-key `portal_sessions` to `users.id`.** Cleanest is a fresh
   `portal_user_sessions` table with the same shape but
   `user_id REFERENCES users(id)`. Reusing api-server's `refresh_tokens`
   would entangle two different session lifecycles (refresh-token
   rotation + login-attempt rate-limit vs. opaque-session-JWT
   invalidation) and is not recommended.
4. **Cut over `UnifiedPortalUserRepo`** to write to `users` only.
5. **Drop the legacy tables** in migration `00144_drop_portal_users_tables.sql`
   in this order: `portal_password_reset_tokens` → `portal_sessions` →
   the back-pointer column `users.portal_origin_id` → `portal_users`.
6. **Delete the legacy code surface:** `PortalRepository::create_user`,
   `update_user`, `update_password_hash`, `upsert_sso_user`,
   `find_user_by_*`, the entire `portal::PortalUser` model, and the
   seed factory's `create_portal_user`.

That sequence is well outside the scope of the multitenancy
integration PR. It belongs in its own dedicated "portal_users
retirement" track.
