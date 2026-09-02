# pm-security — 2026-09-02

_Rotating role this run (pm_cursor idx 5). Last ran 2026-07-21 — 6 weeks stale; refreshed against current churn. Static read; no compile/run._

## Summary

This window ships **security-hygiene reinforcement** rather than fresh vulnerabilities: #2925 patched a raw-sqlx-error leak class in `compliance.rs` (audit-log count now goes through `db_error`, masking internals in 500 bodies), and #2926 deleted the decommissioned FCM legacy send path (attack-surface reduction). Reviewed `data_export` + `gdpr.rs` while adjacent — GDPR download / status handlers correctly gate on `export_request.user_id != user.user_id`, so no cross-user PII leak on that surface. The **standing gh-issue-2797 (RUSTSEC-2026-0258 h2 empty-DATA-frame DoS)** remains the single biggest security debt — 15+ days unresolved and blocks every backend PR.

## next_actions

- **[high]** Land the h2 bump for RUSTSEC-2026-0258 (gh-issue-2797) — every backend PR runs into it in cargo-deny. Confirm patched h2 version present in `backend/Cargo.lock`; run cargo-deny locally / in CI to green. DoD: cargo-deny advisories clean on `dev` — **owner: pm-security**. dependency: none.
- **[high]** Grep-sweep for the `#2925` raw-db-leak class across `backend/servers/api-server/src/routes/**`: every secondary DB call (`.count()`, aggregate helpers, `.map_err(...)` that stringifies sqlx) that does NOT go through a `db_error(...)` helper is a candidate. Rank hits, land the top 3 in one code-review PR. DoD: no `.map_err(|e| e.to_string())` or `.map_err(sqlx errors → StatusCode)` idiom left in compliance / reports / audit routes — **owner: pm-backend**. dependency: none.
- **[high]** Add integration test for the compliance audit-log DB-error path (#2925 regression guard): drop the audit_logs table permission or inject a sqlx failure, assert 500 body contains NO connection-string / SQL fragments. DoD: red-then-green test committed alongside the fix or in a follow-up — **owner: pm-qa**. dependency: pm-backend (test scaffolding).
- **[medium]** Reality-server saved-search typed-error enum (#2922) sets a good pattern — extend to `inquiries.rs` and `reports.rs` on the same server, which still have unwrap-to-500 error paths. Reduces the chance of leaking domain internals via panic-derived 500 bodies. DoD: 1 more route migrated + regression test — **owner: pm-backend**. dependency: none.
- **[medium]** Audit the `push_fanout.rs` receipt-processing paths for `user_id` in structured logs (`user_id = %user_id`) — after the #2926 legacy-path deletion, the module still emits `user_id` at info+ level in ~10 sites; verify the log-scrubber policy scrubs UUIDs or acknowledge this as accepted PII exposure. DoD: written policy note in `docs/api/README.md` under "Log scrubbing" OR add a scrubber rule — **owner: pm-devops**. dependency: pm-security (policy call).
- **[medium]** Close carried risks `risk-layout-webhook-replay-2026-07-23` (#2485) and `risk-mobile-layout-cache-cross-tenant-2026-07-23` (#2486) — both open 6+ weeks, no PR movement. DoD: PR merged for each or explicit "accepted, tracked in Q4" — **owner: pm-security**. dependency: pm-backend / pm-mobile.

## risks

- **Standing h2 DoS (RUSTSEC-2026-0258) (high/high):** every backend PR fails cargo-deny; delivery loop routes around by disabling the gate locally, which erodes the advisory-check discipline. Mitigation: bump h2 crate.
- **Raw-sqlx-error leak class (medium/high):** #2925 fixed one site in compliance.rs; the same idiom (secondary count/aggregate calls returning `Result<_, sqlx::Error>` mapped ad-hoc to 500 string) very likely repeats across reports / audit / analytics routes — a targeted grep is cheap and finds unknown-count sites. A leaked connection URL or SQL fragment in a 500 body is a real infoleak.
- **`push_fanout.rs` `user_id` in structured logs (low/medium):** 10+ `user_id = %user_id` sites at info/warn level. If the org's log-shipper doesn't scrub UUIDs, every send/receipt event exports a user identifier to the log store. Mitigation: policy decision + optional scrubber rule.
- **Aging: layout-webhook-replay (#2485) and mobile-layout-cross-tenant-cache (#2486) (medium/high):** two open post-merge review findings from 2026-07-23, no PR movement in 6 weeks. Mitigation: land or explicitly de-prioritize.
- **OAuth 10a-* has no e2e security tests (medium/high):** carried from 2026-05-27 run — introspection / refresh-rotation / PKCE-S256 enforcement untested; a refactor could silently reintroduce revoked-token acceptance. Mitigation: dedicated security-test PR (already queued as `pm-qa-oauth-security-tests`).

## open_questions

- Does the org's log-shipper scrub UUID-shaped fields, or do `user_id = %user_id` structured-log sites in `push_fanout.rs` export identifiers verbatim to the log store?
- Is #2652 (mobile-native/KMP cloud-runner) the reason `pm-mobile-android-sso-csrf-half-wired-2026-07-30` (#2574 — SsoStateStore.mint() has no call site so every reality://sso callback is rejected) hasn't moved? A never-called mint is a **hard availability bug on Android SSO**, not just a security-hardening item.
- The compliance-tier gate (require_super_admin vs require_platform_admin) is enforced in-handler, not via an axum extractor — is that a deliberate architectural choice, or should we extract a `RequireTier` extractor to prevent an accidentally-unguarded new handler in this file?

## decisions_needed

- Adopt or reject a repo-wide "no raw sqlx error text in HTTP bodies" lint / clippy check — owner: pm-tech-lead.
- Log-scrubbing policy for `user_id` in structured logs — either scrub at shipper OR downgrade info→debug in `push_fanout.rs` — owner: pm-devops (with pm-security concurrence).
