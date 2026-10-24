# pm-scrum-master — 2026-10-19

_Always-on synthesis run. 3-day lag since 2026-10-16 — on schedule._

## Summary

Mild delivery window: **2 PRs merged** (both dispatcher-authored code-review follow-ups on existing surfaces), **1 draft PR open** (#3123 api-core TOTP RNG), **1 critical new issue** (#3124 cargo-deny RUSTSEC-2026-0368/0369 on zip 8.6.0 via rust_xlsxwriter). The merge gate is now doubly locked: #2966 (swagger-ui egress) still blocks builds, #3124 blocks merges even on green builds.

## Shipped since last run (2 PRs)

- **#3121** reality-web auth-context transient-logout cushion — SSO consumer side; resolves `code-review-reality-web-auth-ctx-transient-logout` (+159/-2 with tests).
- **#3122** ppt-web-core useAmlDashboard AML flags — compliance view; resolves `code-review-ppt-web-core-aml-checks-hardcoded-false` + `code-review-ppt-web-core-aml-risk-factors-unguarded-map` (+117/-6 with tests).

## Sprint progress

- **Sprint:** _Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth_
- **Epics done:** 3/5 at sprint level (8A done, 10A done, cross-cutting unknowns ignored); coverage-level all 13 epics complete at 49/49 done.

## Next actions (top 3)

| # | Priority | Dependency | Action | Definition of done |
|---|---|---|---|---|
| 1 | high | none | Resolve #3124 cargo-deny advisories (RUSTSEC-2026-0368/0369 on zip 8.6.0 via rust_xlsxwriter) — unblock every backend PR merge | cargo-deny passes on dev HEAD |
| 2 | high | none | Carry on #2966 utoipa-swagger-ui egress unblock — gates backend cloud builds; now compounded by #3124 blocking merges on green builds | backend.yml green end-to-end in cloud runner |
| 3 | high | #2966 + #3124 | Land the 3 open IDOR retries (#2946/#2944/#2945) once the twin merge-gate clears | three merged PRs with cross-tenant repro tests |

## Risks

| Risk | P | I | Mitigation |
|---|---|---|---|
| **#3124 cargo-deny RUSTSEC-2026-0368/0369:** every backend PR merge is now gated on the advisory check; compounds with #2966 which already blocks builds. Builds green PRs cannot merge. | high | high | Either bump rust_xlsxwriter to a version with safe zip transitive dep, OR add a bounded cargo-deny exception for RUSTSEC-2026-0368/0369 with a deadline captured in dispatcher backlog and surfaced in CI gate description. |

(All other risks carried from 2026-10-16 roster — see `risks.md`.)

## Blockers

- **all backend PR merges** → #3124 cargo-deny FAILED (RUSTSEC-2026-0368/0369 zip 8.6.0 via rust_xlsxwriter) — owner: pm-security.
- **backend cloud builds** → #2966 utoipa-swagger-ui egress 403 — owner: pm-devops.
- **KMP cloud builds** → #2652 — owner: pm-devops.

## Open questions

(none this run — the merge-gate is unambiguous and the fix path is a known pattern.)

## Decisions needed

- Pin `rust_xlsxwriter` at a version with safe zip transitive dep, OR exempt RUSTSEC-2026-0368/0369 in `cargo-deny.toml` with a deadline captured in dispatcher — owner: pm-security.
