# `portal_users` Removal Plan (D1.3 follow-up)

**Status:** *deferred to Phase 3.0*
**Owner:** identity unification track
**Decision date:** 2026-05-15 (Phase 2 D1.3 audit)

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
