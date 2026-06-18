# pm-security — 2026-06-18

_Rotation slot 5 — last run 2026-05-27 (22 days stale)._

## Summary

The sprint's most critical active security gap is the CI gate: `dev` branch protection requires only a single generic `"check"` context (`app_id: 15368`) — `cargo test` is **NOT** a required check, confirming issue **#1538** and meaning auth/RLS regression PRs can merge freely. Seven of eight test-hardening batch issues (#480-#487) remain open, including OAuth refresh-token revocation bypass (#481), WebSocket JWT-in-query-param (#480), and multiple IDOR gaps without regression tests.

## Findings

- `dev` branch protection (`enforce_admins: false`, `strict: false`) lists only `"check"` (app 15368) as a required status check — backend `cargo test` job is advisory only (issue #1538 confirmed).
- `session.rs:55` `find_by_token_hash` still contains `revoked_at IS NULL` guard — the PR #470 regression in issue #481 appears partially remediated, but no integration test asserts it; sprint-status still marks #481 open.
- `ws_notifications.rs:122` carries a `// Never log params.token` comment so the JWT does not appear in handler logs, but it is still transmitted in the `?token=…` query param → access logs / proxy logs / Referer headers (#480 structural exposure persists).
- `backend/crates/db/src/repositories/document.rs` retains many `#[deprecated]` legacy non-RLS methods. Epic 7A is the active sprint with 8-PR churn on this file in 48h — any handler regression to a deprecated method silently bypasses tenant isolation.
- `rental.rs` queries all bind `org_id`; the FORCE-RLS workaround for GH #1363 is inline-documented and legitimate. Cross-org IDOR test scaffolding exists (`reserve_funds_cross_org_idor_tests.rs`).
- Recent landings (positive direction): #1467 (mfa cross-user IDOR test), #1460 (FORCE RLS on developer_oauth_apps/grants), #1561 (PortalPrincipal for imports closes #1300), #1473 (BIT-85 cross-tenant OAuth upsert hazard), #1539 (reject public client supplying client_secret), #1502 (Booking.com credential encryption at-rest IG3). Net direction is positive — but the gate gap negates that net.

## Risks (top 5)

| risk | prob | impact | mitigation |
|---|---|---|---|
| CI test job not required on dev (#1538) | high | high | Add `backend / test` to required_status_checks; freeze backend merges until done |
| OAuth refresh-token revocation bypass (#481) | medium | high | Add integration test asserting revoked tokens rejected; close #481 only when green |
| WebSocket JWT in query param (#480) | high | medium | Migrate to OTT exchange (`POST /ws/ticket` → opaque short-lived token) |
| Legacy non-RLS document.rs methods callable during 7A churn | medium | high | Grep routes for deprecated calls; enforce `clippy -D deprecated` in CI |
| 5 IDOR backlog signals — merge status unconfirmed | medium | high | `gh pr list --search idor`; per-finding confirm patch+test or open tracking issue |

## Next actions

1. **[high]** Add `backend / test` as required status check on dev — close #1538 — owner: pm-security (→ pm-devops landing)
2. **[high]** Close #481: add revoked-OAuth-token rejection integration test; verify guard still in session.rs:55 — owner: pm-security (→ pm-backend)
3. **[high]** Close #480: migrate WebSocket auth to OTT exchange — owner: pm-security (→ pm-backend)
4. **[high]** Audit document.rs route handlers for deprecated method calls; add `clippy -D deprecated` CI gate — owner: pm-security
5. **[medium]** Re-audit 5 IDOR backlog signals (equipment, voice-device, llm-doc, reality-inquiry, reserve-funds); close #483 — owner: pm-security
6. **[medium]** Close #486: replace direct getToken() calls in announcement/fault frontend with axios-interceptor path — owner: pm-security (→ pm-frontend)

## Decisions needed

- Treat #1538 (test gate) as a release blocker: freeze backend feature merges until required check is enforced.
- Does #480 (WS JWT in query) require an OTT protocol change before Epic 8A ships, or is it accepted-risk deferred?
- Should deprecated non-RLS document.rs methods be removed outright (breaking) or kept with a `clippy -D deprecated` CI gate?

## Open questions

- Which of the 5 IDOR backlog signals have merged remediation vs still open? (`gh pr list --search idor` unavailable this run)
- Is the `security-test-gate.yml` workflow enforcing or advisory on dev? (`gh issue list --label security` unavailable)
- Are there deprecated-method callers in the active 7A document route handlers?
- Does the `WS_MAX_SESSION_SECS` (4h) vs JWT lifetime (15m) mismatch in #480 have a wired exp-based close in the session loop?
