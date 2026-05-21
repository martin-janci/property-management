# reality-server-security-route-tests

**Vector:** test-gap
**Score:** 4
**Source:** PR #316 | PR #317
**Confidence:** high

## Hypothesis
PRs #316 and #317 shipped a batch of reality-server security fixes — open-redirect allowlisting on SSO login, missing-auth/IDOR closures on agency routes, and request-body length caps + URL-scheme validation — but added no route-level regression tests (only `url_validator.rs` carries inline unit tests). Without a test pinning the security contract at the handler boundary, a future refactor can silently reintroduce an open redirect, a cross-agency IDOR, or an unbounded input. The smallest durable fix is one integration test module that drives the real router and asserts each fix's reject path.

## Evidence
- PR #316 file list: `sso.rs`, `agencies.rs`, `agency_imports.rs`, `compare.rs`, `reports.rs`, `util/url_validator.rs` — no `*test*` file in the diff (`+550/-32`).
- PR #317 file list: `agency_branding.rs`, `articles.rs`, `inquiries.rs`, `reports.rs`, `users.rs` — no `*test*` file in the diff (`+280/-5`).
- `backend/servers/reality-server/src/routes/sso.rs:124` — `fn ensure_redirect_uri_allowed(state, raw)` is the open-redirect guard; grep of `tests/` + `routes/` for `ensure_redirect_uri_allowed` / `invalid_redirect` / `ALLOWED_REDIRECT` returns nothing.
- `backend/servers/reality-server/tests/raw_pool_audit_tests.rs` is the only integration test file in the crate (covers tenant-host isolation, not these fixes).
- `backend/servers/reality-server/src/util/url_validator.rs` has 11 inline `#[cfg(test)]` cases — the SSRF parser is covered, so the gap is the *route-handler application*, not the parser.

## Files
- `backend/servers/reality-server/src/routes/sso.rs:124`
- `backend/servers/reality-server/src/routes/agencies.rs`
- `backend/servers/reality-server/src/routes/reports.rs`
- `backend/servers/reality-server/src/routes/inquiries.rs`
- `backend/servers/reality-server/tests/raw_pool_audit_tests.rs`

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [x] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** no C4/C5 ticked → cloud-ok.

Mode: cloud-ok

## Repro steps
1. `cd backend && grep -rn "ensure_redirect_uri_allowed\|invalid_redirect" servers/reality-server/tests/` → no matches (the guard is untested).
2. Confirm only one integration test file exists: `ls servers/reality-server/tests/` → `raw_pool_audit_tests.rs`.
3. Expected (today): there is no automated test that fails if `ensure_redirect_uri_allowed` is deleted from `sso.rs`, if an agency IDOR check is removed, or if an input length cap is dropped. Actual: regression of any of these three fixes ships green.

## Suggested approach
1. Add `backend/servers/reality-server/tests/security_regression_tests.rs`, modelled on the existing `raw_pool_audit_tests.rs` harness (minimal Axum router + `#[sqlx::test]` migrated pool, `tower::ServiceExt::oneshot`).
2. **Open-redirect**: build the SSO router, GET `/api/v1/sso/login?redirect_uri=https://evil.example.com` against a state whose allowlist excludes that origin; assert `400` + `invalid_redirect_uri` (the reject path at `sso.rs:78-82`). Add a positive case: an allowlisted origin returns the redirect.
3. **IDOR / missing-auth**: pick one agency-scoped route hardened in #316 (e.g. an `agencies.rs` or `reports.rs` handler) and assert a request resolving to agency A cannot read agency B's resource (expect `403`/`404`), mirroring the contract style in `raw_pool_audit_tests.rs`.
4. **Input caps (#317)**: POST an over-long body to one capped endpoint (e.g. `inquiries.rs` create) and assert `400`/`413` rather than acceptance.
5. Reuse any `common`/test-app helper if reality-server has one; otherwise seed the minimal org/agency fixtures the chosen routes need (C2).
6. Keep coverage to the three contracts above — one assertion per fix is enough to pin the regression; do not attempt exhaustive per-handler coverage.
7. Run the suite and confirm each new test fails when its guard is temporarily reverted, then passes with the guard in place.

## Alternatives considered
- **Inline `#[cfg(test)]` in each route module** — rejected because the fixes span 8+ handlers across two PRs; a single integration module that drives the real router gives higher-fidelity contract coverage than scattered unit tests and matches the crate's existing `tests/` convention.
- **Snapshot/HTTP-record tests** — rejected because they pin response *shape*, not the security *decision*; a regression that flips a `403` to `200` with the same body shape would slip through.

## Root-cause trace
N/A — test-gap doesn't need backward tracing. (The fixes are correct; the gap is the absence of tests guarding them.)

## Test plan
- [ ] `backend/servers/reality-server/tests/security_regression_tests.rs` — open-redirect reject + allow, one IDOR/missing-auth denial, one input-cap rejection.
- [ ] Regression scenario: each test must fail if its corresponding guard (`ensure_redirect_uri_allowed`, the agency ownership check, the length cap) is removed.
- [ ] `cd backend && cargo test -p reality-server --test security_regression_tests`

## Out of scope
- Changing any production behaviour in `reality-server` routes (this is tests-only).
- Exhaustive per-handler coverage of every route touched by #316/#317.
- api-server auth tests (#332/#335 already added `ai_auth_tests.rs` / `faults_tests.rs`).
- The `url_validator.rs` parser (already covered by inline unit tests).

## After-merge
- Move this file to `plans/_archive/reality-server-security-route-tests.md`
- Mark the matching `backlog.json` row as `status: "done"`
