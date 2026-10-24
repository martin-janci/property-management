# pm-integration — 2026-10-19

_Rotating-role run (next_index 7 → 0, wrap). Last run: 2026-07-24 — ~3 months stale, so this is a fresh integration lens on the current merge/build-gate stack._

## Summary

Integration-contract surface is quiet this run — the two merged PRs (#3121 reality-web auth-context, #3122 ppt-web useAmlDashboard) are client-side cushions over existing surfaces and do not change any external contract. The signal of the day is a merge-queue failure mode: #3124 cargo-deny advisories (RUSTSEC-2026-0368/0369 on zip 8.6.0 via rust_xlsxwriter) now vetoes every backend merge, independently of #2966 (which already vetoes every backend build). The twin lock means a green PR still cannot land.

## Next actions

| # | Priority | Dependency | Action | Definition of done |
|---|---|---|---|---|
| 1 | medium | none | Audit integration-touching PRs since 2026-10-17 for OpenAPI / TypeSpec drift (draft #3123 is api-core TOTP RNG — no contract impact; recent merged #3121/#3122 are client-side only) | api-validation.yml stays green for the audit window; a one-line note on each PR confirming "no contract delta" or opening a follow-up |
| 2 | medium | #3124 | Surface the RUSTSEC-exception policy in the cargo-deny CI gate description so #3124-class blockers triage faster next time (bounded deadline, approver captured in the exception row) | cargo-deny CI gate description links the exception policy and names the current triage owner |
| 3 | low | none | Document the reality-server SSO consumer transient-401 contract vs reality-web auth-context cushion (PR #3121) in docs/api/README.md — ensure client/server treat transient vs permanent 401 symmetrically | docs/api/README.md gains a short section on "transient 401 vs permanent 401" for the SSO introspect / refresh pair |

## Risks

| Risk | P | I | Mitigation |
|---|---|---|---|
| External-dependency drift surface: cargo-deny can veto every backend merge on any new RUSTSEC advisory landing on a transitive dep (today: rust_xlsxwriter → zip 8.6.0 → RUSTSEC-2026-0368/0369). Integration velocity collapses to zero on advisory bursts; no SLA on how fast exceptions get triaged. | medium | high | Introduce a bounded cargo-deny exception window with a deadline tracked in dispatcher backlog; add CI gate description text that links the exception policy and names the current triage owner. |
| api-client regen drift: PR #3122 touched @ppt/api-client compliance types (AML flags). If api-validation.yml did not regenerate cleanly against the typespec snapshot, downstream frontends could silently diverge on nullable flag defaults. | low | medium | Spot-check api-validation.yml on PR #3122; open a follow-up to regenerate if the client bump was not part of the merge. |

## Open questions

- Is the SSO consumer's 401 handling in reality-web auth-context (PR #3121) symmetric with the api-server /auth/refresh contract? Post-condition should be: both sides treat transient 401 as retriable only when the server includes `WWW-Authenticate: Bearer error="invalid_token"`; a bare 401 remains permanent-logout. The PR adds the client cushion but we have no explicit server-side contract line to point to.

## Decisions needed

- Formalize the RUSTSEC-exception policy (max deadline, approver, surfacing in CI gate description) — owner: pm-security. Trigger: the #3124 cargo-deny gate is the second instance of this failure mode this quarter; without a policy, each occurrence stalls the merge queue until it's hand-triaged under pressure.
