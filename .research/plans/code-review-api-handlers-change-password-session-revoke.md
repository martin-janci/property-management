# code-review-api-handlers-change-password-session-revoke

**Vector:** security
**Score:** 2
**Source:** Issue #3352 (surfaced by post-merge review of PR #3348)
**Confidence:** high

## Hypothesis
The authenticated `change_password` handler at `backend/servers/api-server/src/routes/auth/password_reset.rs:396-540` updates the user's password hash but never revokes the user's other active refresh tokens / sessions. Its sibling `reset_password` handler in the same file does — it calls `state.session_repo.revoke_all_user_tokens(user.id, None)` at line 345 right after the hash is persisted, because a password change is a security event and any pre-existing session (incl. the attacker's) must die. Mirroring the same two-line call in `change_password` closes the gap; the existing repository method already handles the `except` parameter to preserve the caller's own current session if that's the chosen semantics.

## Evidence
- `backend/servers/api-server/src/routes/auth/password_reset.rs:396-540` — `change_password` handler body: validates current password, policy, hashes, calls `user_repo.update_password(user.id, &password_hash)` at L522, then returns `ChangePasswordResponse` at L537 — no session-revocation call anywhere.
- `backend/servers/api-server/src/routes/auth/password_reset.rs:345` — sibling `reset_password`'s call: `state.session_repo.revoke_all_user_tokens(user.id, None).await` with comment `// Revoke all refresh tokens for security (force re-login)`.
- `backend/crates/db/src/repositories/session.rs:191` — `pub async fn revoke_all_user_tokens(&self, user_id: Uuid, except: Option<Uuid>) -> …` already exists and is reused by `suspend_cascade.rs`, `auth/sessions.rs`, `auth/mod.rs`.
- Issue #3352 ("Follow-up: change_password does not revoke other sessions after a password change (PR #3348)") — filed by `ppt-review-merged` skill on 2026-11-07, labels `follow-up` + `from-merged-review`, severity: medium security.
- Route is `POST /api/v1/auth/password/change`, Story 1.4 / onboarding P0s, shipped in PR #3348 (merged 2026-11-06).

## Files
- `backend/servers/api-server/src/routes/auth/password_reset.rs:396`
- `backend/servers/api-server/tests/suites/auth_tests.rs`
- `backend/crates/db/src/repositories/session.rs:191`

## Dependencies
<!-- No upstream task dependencies — all evidence files exist on `dev`; the repository method is already shipped. -->

## Required capabilities
- [x] C1 — Systematic debugging (security bug, comparing two sibling handlers)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):** `Mode: cloud-ok`
Mode: cloud-ok

## Repro steps
1. Log in as a user `U` on device A, obtaining a refresh token `RT_A`.
2. Log in as the same user `U` on device B, obtaining a refresh token `RT_B`.
3. On device A, call `POST /api/v1/auth/password/change` with `{ current_password, new_password }`. It returns 200.
4. From device B, call `POST /api/v1/auth/refresh` with `RT_B`.
   - Expected after fix: 401 — the refresh token minted before the password change is no longer honoured.
   - Actual today: 200 — device B stays logged in on a password the user just changed; defeats the purpose of the change.

## Suggested approach
1. In `change_password` (`backend/servers/api-server/src/routes/auth/password_reset.rs:510-539`), after the successful `user_repo.update_password(user.id, &password_hash)` call at L522 and the `tracing::info!` at L535, insert the session revocation before building `ChangePasswordResponse`.
2. Mirror `reset_password`'s pattern exactly (`password_reset.rs:343-355`) for behaviour parity: `state.session_repo.revoke_all_user_tokens(user.id, None).await`. On error, `tracing::error!` and keep the overall operation successful — matches how `reset_password` handles the same call (password is already persisted, revocation is best-effort and must not undo the write).
3. Decide "revoke all" vs "revoke all except current caller": the issue author weighs both. Pick **revoke all (`None`)** for behavioural parity with `reset_password` — the caller's device will see its next refresh fail and transparently prompt a login, which is consistent with "a password change is a security event". The alternative (`except: Some(current_token_id)`) requires plumbing the caller's refresh-token id through `authenticated_user_id`, which is outside the minimal fix shape.
4. Add a regression test in `backend/servers/api-server/tests/suites/auth_tests.rs`, co-located with the existing `change_password` suite: seed a user with two active refresh tokens (`RT_A`, `RT_B`), successfully change the password, then assert `POST /api/v1/auth/refresh` with `RT_B` returns 401. The test must fail on `dev` before the fix.
5. Run `cargo test -p api-server --test suites change_password` (or the specific test name) to confirm the new test is green post-fix.
6. Run `cargo fmt && cargo clippy -p api-server -- -D warnings` to clear style gates.
7. Open PR on `dev` with title `fix(UC-14): revoke other sessions after change_password (closes #3352)`.

## Alternatives considered
- **Preserve the caller's current session via `revoke_all_user_tokens(user.id, Some(current_token_id))`** — rejected because `change_password` currently does not thread the caller's refresh-token id (the `authenticated_user_id` helper returns only the user id). Plumbing that in is a larger surface change for marginal UX benefit; behavioural parity with `reset_password` is simpler and the caller's device will re-authenticate transparently on the next refresh.
- **Soft-revoke: mark tokens as "needs-rotation" instead of hard-revoke** — rejected because the repository has no such state and the attacker model demands immediate invalidation; adding a lifecycle state is scope creep far beyond the one-line fix the issue proposes.

## Root-cause trace
1. Symptom: after `POST /api/v1/auth/password/change`, a refresh token minted before the change still succeeds at `POST /api/v1/auth/refresh`. Observable in the proposed regression test above.
2. ← immediate cause at `backend/servers/api-server/src/routes/auth/password_reset.rs:522-537`: handler writes the new hash and returns success with no call into `state.session_repo`.
3. ← upstream cause at the same file `L396-540`: the handler was authored as a stand-alone flow rather than copying the `reset_password` body's post-write hook that already lives at `L345`. The two handlers diverged on this security-event rule.
4. Origin: PR #3348 (merged 2026-11-06) — "verify e-mail, password routes, buildings, people & residents" — introduced the authenticated `change_password` route (Story 1.4) without mirroring `reset_password`'s existing session-revocation hook.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/auth_tests.rs::change_password_revokes_other_sessions` — new integration test seeding two refresh tokens, changing password, asserting the other refresh 401s.
- [ ] Regression: existing `change_password` suite still passes (no new false-positive on the caller's own current refresh — if `None` is used, the caller's next refresh will 401 and then re-login, which is the documented behaviour; if a direct `/me` call is in the same test that calls change_password, that one call is fine because `change_password` uses the access token, not a refresh — only a subsequent `/auth/refresh` would fail).
- [ ] `cargo test -p api-server --test suites change_password`
- [ ] `cargo fmt --all && cargo clippy -p api-server -- -D warnings`

## Out of scope
- Rewriting `authenticated_user_id` to expose the caller's refresh-token id so the `except: Some(…)` path becomes feasible — that is a separate refactor and not required to close the security gap.
- Adding a user-facing "sign out of all other devices" UI in ppt-web / mobile — a client-side feature, orthogonal to the server-side guarantee that a password change invalidates prior sessions.
- Touching `reset_password` behaviour — it already does the right thing.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-change-password-session-revoke.md`
- Mark `code-review-api-handlers-change-password-session-revoke` in `backlog.json` as `status: "done"`
