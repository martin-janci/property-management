# pm-security — 2026-06-26

_Rotating role this run (pm_cursor idx 5). Last ran 2026-05-27 — first re-assessment in 30 days. Static read; no compile/run._

## Summary

Five test-hardening issues from the May 25 batch remain open (#480, #481, #482, #483, #487). OAuth refresh-token revocation (#481) appears already fixed at the OAuth layer (`revoked_at IS NULL` present in both `backend/servers/api-server/src/routes/oauth.rs:414` and `backend/crates/api-core/src/session.rs:55`), but sprint-status.yaml still marks the gate open — blocking 10a-1 + 10a-3 unnecessarily. Phase 1.5 surfaced two panic-path findings in reality-server (`backend/servers/reality-server/src/handlers/users/mod.rs:551` rng `.expect()`, `src/state.rs:759` http-client `.expect()`) that are not yet tracked as issues. The IoT WS token-logging risk from #480 has a suppression comment at `iot/realtime.rs:118` but no test asserting absence of the token in logs.

## Next actions

1. **[high] Close or downgrade issue #481** — Audit OAuth and session refresh-token paths; confirm both carry `revoked_at IS NULL` guard. Mark closed so 10a-1 and 10a-3 story gates clear. _DoD: #481 closed; sprint-status.yaml story-gate entries for 10a-1/10a-3 cleared._
2. **[high] Fix reality-server panic paths** — Replace `.expect('OS rng failed')` at `handlers/users/mod.rs:551` with returned 500; replace `.expect('Failed to create HTTP client')` at `state.rs:759` with startup `Result` propagation. _DoD: both sites use `?` or `map_err`; `cargo clippy` clean; no `.expect()` on fallible OS/network calls in request path._
3. **[high] Resolve issue #480** — Verify `iot/realtime.rs:118` suppression is enforced by a structured-log test (assert token string absent from tracing output); confirm session re-validation after JWT expiry is implemented for WS keep-alive. _DoD: #480 has linked test proving token absent; session expiry evicts WS connection; issue closed._
4. **[medium] Fix issue #482** — Trace `user.role` in `ProtectedRoute.tsx:120` back to `deriveActiveRole` — if it still reads `tenants[0]` for multi-tenant users, replace with active-tenant-context-aware role derivation and add unit tests. _DoD: `deriveActiveRole` does not blindly use `tenants[0]`; `ProtectedRoute.test.tsx` covers multi-tenant non-primary tenant; #482 closed._
5. **[medium] Add MFA brute-force / rate-limit integration test coverage** — Per issue #487; blocks 10a-1 story gate. Verify `mod common` compiles in nested module context. _DoD: rate-limit path exercised in `#[sqlx::test]`; `cargo test -p api-server` green; #487 closed._
6. **[medium] Review PR #1809** — Grep for residual `provider_secret`/`api_key` fields in accounting response structs that lack `#[serde(skip_serializing)]`; confirm 404-not-500 body is tenant-neutral (no leak via timing or body). _DoD: no credential fields serializable in responses; 404 body tenant-neutral._

## Risks (added to risks.json)

| Risk | P×I | Mitigation |
|---|---|---|
| reality-server rng `.expect()` panics on OS entropy failure (password-reset, untracked) | low×high | Propagate error → 500; file as release blocker |
| #481 stale tracker state may block 10a-1/10a-3 or cause accidental revert | medium×high | Verify and close #481; add regression test |
| ProtectedRoute role check depends on `user.role` possibly derived from `tenants[0]` (multi-tenant elevation) | medium×high | Trace `deriveActiveRole`; fix before 7A document permissions ship |
| #480 WS JWT logging suppression is human-only comment | medium×high | Add automated test assertion; consider header-exchange handshake |
| #483 voice device IDOR (list-commands existence leak) — no story gate | high×medium | Assign rust-backend next sprint; normalise empty-vs-403 to 403; add IDOR test |

## Open questions

- Is #481 actually still open, or did the OAuth fix already resolve it? Code says fixed, tracker says open — needs human confirmation before closing story gates.
- Does `deriveActiveRole` still fall back to `tenants[0]` for multi-tenant users, or was it patched in a #459 follow-up?
- Does the IoT WS upgrade path re-validate the JWT on a schedule, or is a mid-session-expired token silently kept alive?
- Did PR #1768/#1756 inadvertently serialize attachment pre-signing secrets or full thread participant lists into the camelCase response fields?
- Will Epic 7A document-download (7a-4) use pre-signed S3 URLs scoped per-tenant?

## Decisions needed

- Are reality-server `.expect()` panics release blockers for the current sprint, or deferred to a hardening sprint? — owner: pm-security + rust-backend + PM lead
- Should issue #481 be formally closed on code evidence, or must an integration test land first as condition of closure? — owner: rust-backend
