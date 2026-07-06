# security-jwt-role-vs-db-role-mismatch-mutating-handlers

**Vector:** security
**Score:** 3
**Source:** PR #2120 (partial fix) + Phase 1.6 pm-security lens 2026-07-06
**Confidence:** high

## Hypothesis
PR #2120 fixed the JWT-role/DB-role mismatch in `outages.rs` only, but the identical anti-pattern still gates ~22 mutating handlers across `documents/*.rs`, `announcements/*.rs`, `templates.rs`, and `granular_notifications.rs`. Production login tokens carry `org_id`+`roles` (not the legacy `role` claim), so `TenantExtractor::role` resolves to `TenantRole::Guest` and every `tenant.role.is_manager()` gate 403's real managers — this is fail-closed today, but the exact pattern flipped to fail-open would grant privilege escalation. Same fix as #2120: gate on `rls.role().is_manager()` (DB-derived via `ValidatedTenantExtractor`) and add a real-login regression test.

## Evidence
- PR #2120 (`e6b5093`) — fixed outages.rs six mutating handlers, documented the JWT-vs-DB-role mismatch as the root cause
- `backend/servers/api-server/src/routes/documents/{core,folders,versions,shares}.rs` — still contain 14 `tenant.role.is_manager()` gates on mutating paths (Phase 1.6 pm-security grep)
- `backend/servers/api-server/src/routes/announcements/{crud,lifecycle,engagement,stats,comments,ai_draft}.rs` — 12 more call sites on Epic 6 endpoints
- `backend/servers/api-server/src/routes/{templates,granular_notifications}.rs` — 5 additional call sites
- Precedent: `backend/servers/api-server/tests/outages_happy_path_tests.rs` (PR #2120) shows the correct real-login test harness (`create_authenticated_user_with_org`) that surfaces the bug

## Files
- `backend/servers/api-server/src/routes/documents/core.rs`
- `backend/servers/api-server/src/routes/documents/folders.rs`
- `backend/servers/api-server/src/routes/documents/versions.rs`
- `backend/servers/api-server/src/routes/documents/shares.rs`
- `backend/servers/api-server/src/routes/announcements/crud.rs`
- `backend/servers/api-server/src/routes/announcements/lifecycle.rs`
- `backend/servers/api-server/src/routes/announcements/engagement.rs`
- `backend/servers/api-server/src/routes/announcements/stats.rs`
- `backend/servers/api-server/src/routes/announcements/comments.rs`
- `backend/servers/api-server/src/routes/announcements/ai_draft.rs`
- `backend/servers/api-server/src/routes/templates.rs`
- `backend/servers/api-server/src/routes/granular_notifications.rs`
- `backend/servers/api-server/tests/outages_happy_path_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug/security)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (multi-file security touch)

**Execution mode (auto-derived):** Mode: cloud-ok

## Repro steps
1. Register a fresh user via the real login flow (`create_authenticated_user_with_org` from `outages_happy_path_tests.rs`) — the token carries `org_id`+`roles`, no legacy `role` claim.
2. Add them to an org as `org_admin` (a DB-level manager).
3. Call any mutating endpoint on the listed handlers, e.g. `POST /api/v1/documents/{doc_id}/share`.
4. Expected: 200/201 (the DB role authorizes). Actual on today's `dev`: 403 (`TenantExtractor::role` resolves to `Guest` because the login flow doesn't set the `role` claim).

## Suggested approach
1. In each listed file, replace `tenant.role.is_manager()` with `rls.role().is_manager()`. The handler already holds an `RlsConnection` (built on `ValidatedTenantExtractor`), so no extra DB round-trip is needed — mirrors PR #2120 line-for-line.
2. Drop the now-unused `TenantExtractor` parameter from mutating handlers; keep it on read-only handlers.
3. For each file, add 1–2 regression tests using `create_authenticated_user_with_org` that mutate through the handler and assert 200/201 (not just any 2xx). Model on `create_outage_via_real_login_succeeds` in `outages_happy_path_tests.rs`.
4. In `documents/core.rs:618` and `announcements/comments.rs:474`, `is_manager` is computed as a *boolean used in business logic* (not just gate-then-403) — audit the surrounding logic to confirm the DB role swap doesn't silently flip response shape or side-effects.
5. Sequence: land documents (4 files) → announcements (6 files) → templates + granular_notifications (2 files) as three PRs to keep review surface bounded.
6. After all three land, follow the pm-security recommendation and add a CI grep-gate that fails on any new `tenant.role.is_manager()` in a mutating handler.

## Alternatives considered
- **Add `role` claim to production login tokens** — rejected because it perpetuates trusting a JWT claim over DB state (spoofable via forged tokens; the existing `outages_happy_path_tests.rs` demonstrates the DB-role check catches forged claims).
- **New `require_manager()` extractor that reads `ValidatedTenantExtractor` internally** — a good idea for the future but adds an abstraction to review at the same time as the fix; PR #2120's inline pattern is already reviewed and merged, so mirror it first, refactor later.

## Root-cause trace
1. Symptom: real manager gets 403 on documents/announcements/templates mutating endpoints (documented for outages in the PR #2120 body).
2. ← `tenant.role.is_manager()` evaluates false because `tenant.role == TenantRole::Guest` at `<file>:<line>` (see individual call sites).
3. ← `TenantExtractor::role` reads `AuthUser.role` which is populated from `Claims.role` — a claim the production `JwtService::generate_access_token` never sets (it emits `org_id`+`roles` instead).
4. Origin: PR #1979 established the fabricated-token test harness for outages, then #2107/#2120 traced it back to the shared `TenantExtractor` pattern used across 22+ handlers.

## Test plan
- [ ] `documents/core.rs`: add `share_document_via_real_login_succeeds` — assert 201, response body shape unchanged (`documents/core.rs:618` `is_manager` boolean audit)
- [ ] `announcements/crud.rs`: add `publish_announcement_via_real_login_succeeds` — assert 200 + `status: "published"`
- [ ] `announcements/comments.rs`: add `pin_comment_via_real_login_succeeds` — assert 200 + audit `is_manager` boolean at line 474
- [ ] `templates.rs`, `granular_notifications.rs`: add one real-login mutating test each
- [ ] Command: `cargo test -p api-server --tests documents_ announcements_ templates_ granular_` (backend.yml is the verifying gate — the sandbox lacks Postgres)

## Out of scope
- Refactoring `TenantExtractor` / `AuthUser` into a single `ValidatedManagerExtractor` helper — deferred to a follow-up.
- Auditing the ~12 *other* route files (agencies, rentals, registry, listings) noted in the PR #2120 body — separate plan.
- Removing the `role` claim from `JwtService::generate_access_token` entirely — an even bigger refactor; the DB-role check is the belt+suspenders.

## After-merge
- Move this file to `plans/_archive/security-jwt-role-vs-db-role-mismatch-mutating-handlers.md`
- Mark the matching `backlog.json` row (`id=security-jwt-role-vs-db-role-mismatch-mutating-handlers`) as `status: "done"`
