# pm-security — 2026-07-06

_Rotating role this run (pm_cursor idx 5 → 6). Static read; no compile/run._

## Summary

Authz-mismatch class of bugs is actively being fixed (outage mutations PR #2120, dependency-pin hardening #2096/#2111), but four high/medium severity items from test-hardening batch thb-2026-05-25 remain open and are blocking three OAuth/notification stories from being marked done. WS auth token-in-query-param exposure (#480) is only partially mitigated — exp re-validation was added but the raw JWT still travels in the URL/query string and will land in access logs.

## next_actions

- **[high]** Verify oauth.rs revoked_at filtering (find_access_token_by_hash/refresh token lookups) end-to-end against issue #481's claim that revocation is bypassed; sprint-status still lists #481 open but current code shows `revoked_at IS NULL` present on the hash-lookup queries — reconcile and close or re-open with root cause. DoD: Issue #481 closed with regression test proving a revoked refresh token cannot mint new access tokens, or re-confirmed open with exact query/line identified. dependency: rust-backend.
- **[high]** Redirect WS auth off the query string (issue #480): move token to a `Sec-WebSocket-Protocol` header or first-frame auth message so it stops appearing in access/proxy logs; keep the exp re-validation already added. DoD: `ws_notifications.rs` WsQuery.token removed from URL query; token passed via non-logged channel; access-log grep shows no JWTs. dependency: rust-backend.
- **[high]** Land #1797 (auth on OCR endpoints + manager-gate on rental guest PII reads) before release — currently draft. DoD: PR #1797 merged to dev with tests covering unauthenticated OCR access and non-manager PII read attempts. dependency: rust-backend.
- **[medium]** Add IDOR regression tests for the voice-device fix (#483) and fix list-commands empty-vs-403 existence leak. DoD: New tests assert 403 (not empty list) for non-owners; #483 closed. dependency: rust-backend.
- **[medium]** Fix ProtectedRoute multi-tenant role fallback (#482, uses `tenants[0]`) before promoting 10a-2 to done. DoD: ProtectedRoute resolves role from active tenant context, not array index 0; unit tests added. dependency: react-web.
- **[medium]** Add MFA brute-force/rate-limit e2e coverage and fix the nested `mod common` compile risk (#487) ahead of 10a-1. DoD: Rate-limit test added, workspace compiles clean under `cargo test --workspace`. dependency: rust-backend.

## risks

- OAuth Provider Foundation (epic-10a) has zero stories completed while two high-severity gates (#481 revocation, #487 MFA rate-limiting) remain open against it — risk of shipping OAuth without brute-force/replay protection. probability=medium impact=high. Mitigation: Block 10a-1/10a-3 promotion until #481 and #487 explicitly closed with tests, per existing story_gate rule.
- WS JWT-in-query-param (#480) leaks bearer tokens into HTTP/proxy access logs and browser history, enabling session hijack if logs are exposed. probability=medium impact=high. Mitigation: Move token off the query string; short-term ensure access-log middleware redacts the token param.
- Draft PR #1797 leaves OCR endpoints and rental guest PII reads potentially under-authorized in the meantime. probability=medium impact=high. Mitigation: Prioritize review/merge of #1797 before next release cut.
- cargo-deny/XXE hardening (#2096/#2111) is new; other XML/YAML-parsing dependencies in the workspace may not yet be covered by the deny.toml ban list. probability=low impact=medium. Mitigation: Audit deny.toml coverage against all crates that parse untrusted XML/YAML.

## open_questions

- Is issue #481 actually still open, or was it fixed by an untracked commit — current `oauth.rs` shows `revoked_at IS NULL` on token lookups?
- What is the merge/review target date for draft PR #1797?
- Does the #2120 DB-role-derived authz pattern need to be back-ported to other mutation endpoints beyond the six outage ones?

## decisions_needed

- Confirm whether #481 stays open or closes given current oauth.rs state — owner: rust-backend
- Decide WS auth transport mechanism (header vs first-frame vs signed short-TTL ticket) to replace query-param JWT — owner: rust-backend
