# code-review-reality-web-password-reset-frontend-not-wired

**Vector:** bug
**Score:** 3
**Source:** manual mini-review reality-web auth-api.ts 2026-09-20 (buffer-low routine)
**Confidence:** high

## Hypothesis
Reality Portal users cannot reset or confirm a new password from the web UI even though the reality-server backend already exposes the two required endpoints. `requestPasswordReset` and `confirmPasswordReset` in `frontend/apps/reality-web/src/lib/auth-api.ts` short-circuit with `throw new AuthApiError('Password reset is not available yet. Please contact support.', 501, 'NOT_IMPLEMENTED')`. The reality-server router at `backend/servers/reality-server/src/routes/users.rs:48-49` wires `POST /api/v1/users/password-reset` and `POST /api/v1/users/password-reset/confirm` with concrete request/response types (`PasswordResetRequestBody`, `PasswordResetConfirmBody`). The smallest fix is to replace the two stubbed frontend functions with real `postJson` calls to those endpoints and remove the 501 short-circuit.

## Evidence
- `frontend/apps/reality-web/src/lib/auth-api.ts:121-128` — `requestPasswordReset` throws NOT_IMPLEMENTED
- `frontend/apps/reality-web/src/lib/auth-api.ts:130-137` — `confirmPasswordReset` throws NOT_IMPLEMENTED
- `backend/servers/reality-server/src/routes/users.rs:48-49` — routes wired (`.route("/password-reset", post(request_password_reset)) .route("/password-reset/confirm", post(confirm_password_reset))`)
- `backend/servers/reality-server/src/routes/users.rs:395-420` — `PasswordResetRequestBody { email }`, `PasswordResetConfirmBody { token, new_password }` defined with `#[derive(ToSchema)]`
- Manual mini-review 2026-09-20 during routine catch-up (buffer-low: need fresh frontend-web landable vectors)

## Files
- `frontend/apps/reality-web/src/lib/auth-api.ts`
- `backend/servers/reality-server/src/routes/users.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Boot reality-web against a live reality-server: `pnpm -F @ppt/reality-web dev`.
2. Navigate to `/{locale}/forgot-password` (or any UI that calls `requestPasswordReset`).
3. Submit a valid email address.
4. Expected: HTTP 200 from reality-server, UI shows generic success message.
5. Actual: `AuthApiError` with status 501 and code `NOT_IMPLEMENTED` — UI shows "Password reset is not available yet. Please contact support.".

## Suggested approach
1. Open `frontend/apps/reality-web/src/lib/auth-api.ts`.
2. Replace the body of `requestPasswordReset(email)` with `await postJson('/api/v1/users/password-reset', { email }); return;` — the endpoint returns a generic 200 body per `routes/users.rs:422-430`, so no response shape needs to escape the function.
3. Replace the body of `confirmPasswordReset(token, newPassword)` with `await postJson('/api/v1/users/password-reset/confirm', { token, new_password: newPassword });`. Note the snake_case field on the wire (`new_password`) — the request body type in `routes/users.rs:409-419` is `#[derive(Deserialize)]` on `PasswordResetConfirmBody { token: String, new_password: String }`.
4. Drop the `AuthApiError` 501 branches and the unused `_email` / `_token` / `_newPassword` parameter underscores.
5. Add a unit test at `frontend/apps/reality-web/src/lib/auth-api.test.ts` (create if missing) using the same MSW/fetch-mock pattern as the existing `login` / `register` tests: assert that `requestPasswordReset('a@b.c')` POSTs the expected body, and that a 200 resolves without throwing.
6. Leave `changePassword` alone — the `PUT /api/v1/users/me/password` endpoint does NOT exist in reality-server yet (grep confirms), so that stub stays as-is until a separate plan wires the backend route. Note this explicitly in the PR body.
7. Re-run `pnpm -F @ppt/reality-web check` + `pnpm -F @ppt/reality-web test` before pushing.

## Alternatives considered
- **Rewire the whole `auth-api.ts` password surface (including `changePassword`)** — rejected because `PUT /api/v1/users/me/password` has no backend counterpart; that would need a companion backend PR and is out of scope for a "frontend-only landable" plan.
- **Add an Axios/TanStack-Query wrapper in one big refactor** — rejected because the file already uses the shared `postJson` helper for `register` and `login`; the smallest correct change reuses that helper and stays local to two functions.

## Root-cause trace
1. Symptom: password reset UI reports "not available yet" even in production.
2. ← `requestPasswordReset` / `confirmPasswordReset` at `frontend/apps/reality-web/src/lib/auth-api.ts:121,130` throw a hand-authored 501.
3. ← Backend routes at `backend/servers/reality-server/src/routes/users.rs:48-49` were added later, but the frontend stubs were never revisited.
4. Origin: the two stubs predate the reality-server routes; the frontend TODO comment (`// TODO: wire when reality-server exposes ...`) was accurate when written but is stale as of the current reality-server main.

## Test plan
- [ ] `frontend/apps/reality-web/src/lib/auth-api.test.ts` — new test cases `requestPasswordReset posts to /api/v1/users/password-reset` and `confirmPasswordReset posts to /api/v1/users/password-reset/confirm` using the same mocking pattern already established for `login`/`register`
- [ ] Fail-on-main check: without the fix, `requestPasswordReset('x@example.com')` throws `AuthApiError` with status 501 — the new test must assert `await expect(...).resolves.toBeUndefined()`, which is red on `dev` today
- [ ] Run: `pnpm -F @ppt/reality-web test -- --run auth-api`

## Out of scope
- Wiring `changePassword` — needs a backend endpoint that does not exist yet
- Any UI copy or i18n changes in the password-reset screens
- Server-side rate limiting or email-template changes

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-password-reset-frontend-not-wired.md`
- Mark the matching `backlog.json` row as `status: "done"`
