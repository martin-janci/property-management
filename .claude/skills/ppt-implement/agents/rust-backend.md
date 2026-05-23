# Specialist: rust-backend

Rust + Axum + SQLx implementer for the PPT backend workspace.

## You own
- `backend/servers/api-server/` — UC-* routes, OAuth, JWT, RBAC
- `backend/servers/reality-server/` — public listings, SSO consumer
- `backend/crates/{db, common, integrations}/` — shared data layer, types, external clients
- NOT migrations (see `db-migration` specialist) and NOT TypeSpec (see `typespec`)

## Project layout cheatsheet
```
backend/
  servers/{api-server,reality-server}/src/
    main.rs           — Axum app, route mount
    routes/           — handler functions, request/response types
    services/         — business logic
    state.rs          — AppState (repos, services)
  crates/
    db/src/
      models/         — domain structs + sqlx::FromRow
      repositories/   — DB access (RLS-aware)
      migrations/     — DO NOT touch here — use db-migration specialist
    common/src/       — error types, utils, notifications, RBAC
    integrations/     — external API clients (S3, Airbnb, Booking, etc.)
```

## Conventions (CLAUDE.md + repo-specific)
- Routes return `Result<Json<T>, AppError>` where `AppError` impls `IntoResponse`.
- Always use `RequireCapability` extractor for protected routes (RBAC enforcement).
- Tenant context: extract via `OrgContext`; all repo calls pass `org_id` so RLS holds.
- Logging: `tracing::info_span!` per request; never `println!`.
- Don't `unwrap()` or `expect()` outside tests — use `?` with `AppError::Internal`.
- DB calls: use `sqlx::query!` / `query_as!` (compile-time checked); regenerate offline data with `cargo sqlx prepare` when the SQL changes.

## Step-by-step
1. Read `routes/<existing-similar>.rs` to learn the local pattern (Json extractor, AppError, AppState fields).
2. Add new handler in the right module; mount in `main.rs`.
3. Add a unit test next to the handler (`#[tokio::test]`) using the test harness from `crate::testing` if you need a DB.
4. If a new repo method is needed: add to `crates/db/src/repositories/<area>.rs` with both the impl and a `#[cfg(test)]` round-trip test.

## Verify (MANDATORY)
```bash
cd backend
cargo check -p api-server        # or reality-server / db / common / integrations
cargo test  -p api-server -- <test_name_filter>
```
Quote both exit codes in the PR body.

## Common pitfalls
- Forgetting `RequireCapability(Capability::X)` → route is public. Always check the matching `pm-security` story.
- Adding `Send + Sync + 'static` workarounds on handlers → it usually means a non-`Send` type is being held across `.await`. Refactor instead of adding bounds.
- Touching migrations from a route PR → split into a separate `db-migration` PR.

## Return-line examples
- `pr=512 status=done specialist=rust-backend note=added /api/v1/foo + 2 tests; cargo check + test clean`
- `pr=none status=partial specialist=rust-backend note=cargo check failed: missing trait impl on FooRepo`
