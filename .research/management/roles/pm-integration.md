# pm-integration — 2026-11-19

<sub>Rotating role — last run 2026-10-19 (31 days). Focus: API contracts, generated clients, external integrations, dependency hygiene.</sub>

## Summary

Two contract themes stand out this sprint.
1. **reality-server `/listings`** gained a `slug` field on `ListingSummary` (#3616) — additive, but client-side regeneration (TS `@ppt/reality-api-client`, KMP, iOS) and the canonical-identifier decision are unverified.
2. **Compliance pagination hardening (#3627)** and **voice OAuth error sanitization (#3621)** tightened error contracts, but compliance list responses may still echo an unclamped `limit`, and the voice account-linking error shape hasn't been checked against Alexa/Google (RFC 6749 §5.2) expectations.

`sprint-status.yaml` shows all tracked epics done, so this is a hotspot/PR-driven read. The cargo-deny advisories blocker (#3604, closed via #3608 + #3619) remains the recurring merge-queue risk pattern.

## Next actions

| Priority | Action | Dependency | Definition of done |
|----------|--------|------------|--------------------|
| high | Confirm #3616 slug reaches OpenAPI/TypeSpec, `@ppt/reality-api-client`, KMP + iOS generated clients; document slug as always-populated; state whether detail lookup accepts slug, id, or both. | pm-qa | api-validation.yml green; generated TS/Kotlin clients contain `slug`; deep-link note states lookup identifier. |
| medium | Make compliance list routes echo clamped `limit` (not raw param); add test with `limit=5000`; same pattern audit near line 340. | — | Response `limit` equals the limit actually applied; test with over-limit param; OpenAPI documents max 1000 + 400 on negatives. |
| medium | Verify sanitized `voice_oauth_error_response` bodies follow RFC 6749 §5.2 on Alexa/Google account-linking path. | pm-security | Test asserts body shape + status for `invalid_grant` / `invalid_request` / `server_error`; upstream detail only in logs; voice integration docs updated. |
| low | Document compliance 400 behaviour for negative limit/offset in TypeSpec/OpenAPI; decide: keep `(StatusCode, String)` or move to standard error envelope. | pm-backend | Spec lists 400 for compliance list endpoints; decision recorded. |
| low | Verify #3609 property_type i18n helper keeps filter contract on stable enum codes across sk/cs/de/en. | pm-qa | Test or spec note confirms locale-independent filter query values. |
| high (carry) | Triage #3604 outcome (cargo-deny advisories) and record a bounded exception policy, same failure vetoed merges on 2026-10-19 (#3124). | pm-security | Advisory fixed or exception recorded in `deny.toml` with deadline + approver; cargo-deny gate green on dev. |

## Risks

- **slug client drift (medium / medium)** — additive field may not propagate to generated clients; strict-decode clients could fail; mitigation: run spec/client regen in CI, keep field additive, clients tolerant of unknown fields.
- **compliance `limit` echo mismatch (medium / low)** — paging clients that use `limit` to detect last page could mis-paginate or loop; mitigation: echo effective limit + document semantics in spec.
- **voice OAuth sanitized shape drift (low / medium)** — could break Alexa/Google linking if sanitized message/status drifts; mitigation: contract test + alert on upstream-failure log.
- **cargo-deny advisories recurring (medium / high)** — can block all backend merges; non-required gate so regressions may be silent; mitigation: bounded exception window with owner + deadline, surface triage owner in CI gate description.
- **listings.rs + compliance.rs spec drift (medium / medium)** — churn hotspots receive contract-adjacent edits without evidence that OpenAPI/TypeSpec was updated in the same PRs; mitigation: require `no contract delta` or spec-update line in PR descriptions touching these files.

## Open questions

- Was OpenAPI/TypeSpec + generated clients updated in #3616 (slug) and #3627 (400 on negative pagination)? Diffs not inspected this run.
- Does the reality-server detail endpoint resolve by slug, UUID, or both now that search results expose slug?
- Is the clamped `limit` (max 1000) applied consistently across all compliance list handlers? Only the audit-log handler confirmed.
- Is there a contract test for Alexa/Google account-linking error responses beyond the unit test around voice_webhooks.rs:1906?

## Decisions needed

- Standard error envelope vs plain-text `(StatusCode, String)` for compliance validation errors — **pm-backend**
- RUSTSEC exception policy (max deadline, approver, CI gate surfacing) — **pm-security**
- Whether `slug` becomes canonical public identifier for reality listings (UUID kept for back-compat) — **pm-product**
