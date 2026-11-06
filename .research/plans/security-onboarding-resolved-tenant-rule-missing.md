# security-onboarding-resolved-tenant-rule-missing

**Vector:** security
**Score:** 3
**Source:** PR #3332 body — "Still open" item 3
**Confidence:** high

## Hypothesis
`routes/onboarding.rs::resolve_user_roles` performs its own ad-hoc tenant resolution — compares the `X-Tenant-ID` header against the `org_id` JWT claim, falls back to "all active memberships" when either is missing, and reads memberships directly from `org_member_repo` — instead of consulting the canonical `ResolvedTenant` extractor that the rest of api-server was moved onto in PR #3332. A login token (which carries no `org_id`) combined with an attacker-chosen `X-Tenant-ID` header silently bypasses the tenant-isolation contract: the handler returns the full membership list without ever constraining to the host-resolved tenant. The fix routes role selection through `ResolvedTenantExtractor` so onboarding gets the same 403-on-foreign-org-header behaviour as `features.rs` and every other migrated handler.

## Evidence
- `backend/servers/api-server/src/routes/onboarding.rs:119-125` — hand-rolled `header_org = headers.get("X-Tenant-ID")` / `claim_org = claims.org_id` comparison; only enforces when **both** are present.
- `backend/servers/api-server/src/routes/onboarding.rs:134-136` — fallback `org_scope = header_org.or(claim_org)` leaves `org_scope == None` for a login-token request → membership filter accepts every org.
- `backend/servers/api-server/src/routes/onboarding.rs:150-162` — role iteration reads directly from `org_member_repo`, bypassing the host-tenant isolation that `ResolvedTenant` enforces.
- `backend/crates/api-core/src/middleware/host_tenant.rs:77,684` — `ResolvedTenant` + `ResolvedTenantExtractor` are the canonical extraction path adopted by PR #3332 for features.rs, data residency, subscriptions, etc.
- PR #3332 body — "Still open" item 3: "onboarding doesn't apply the `ResolvedTenant` rule".

## Files
- `backend/servers/api-server/src/routes/onboarding.rs:65`
- `backend/servers/api-server/src/routes/onboarding.rs:110`
- `backend/servers/api-server/src/routes/onboarding.rs:119`
- `backend/servers/api-server/src/routes/onboarding.rs:134`
- `backend/servers/api-server/src/routes/onboarding.rs:150`
- `backend/crates/api-core/src/middleware/host_tenant.rs:77`

## Dependencies
<!-- no dependencies — can land independently -->

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (to spin up the integration test)
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Spin up api-server; create two orgs (A and B) and a user who is a member of org A only.
2. Log in as the user → get a login token (no `org_id` claim).
3. `GET /api/v1/onboarding/status` with header `X-Tenant-ID: <org-B-uuid>`.
4. Expected (after fix): `403 TENANT_MISMATCH` (or, if the user has zero memberships in org B, the empty-memberships 404). Observed (before fix): 200 with the full active-memberships list from **all** orgs (because `org_scope == None` after the fallback) — the handler silently ignores the hostile header.

## Suggested approach
1. Change the signature of `resolve_user_roles` in `backend/servers/api-server/src/routes/onboarding.rs:65` from `headers: HeaderMap, claims: Claims` to `ResolvedTenantExtractor(resolved_tenant): ResolvedTenantExtractor, claims: Claims` (follow the pattern in `features.rs` from PR #3332).
2. Replace the hand-rolled `header_org`/`claim_org` block at lines 119-133 with a single comparison: if `resolved_tenant.organization_id` is present and disagrees with `claims.org_id`, return 403 `TENANT_MISMATCH`.
3. Use `resolved_tenant.organization_id` as the `org_scope` directly — no `.or(claim_org)` fallback (the extractor already resolves via host + header + claim in the documented precedence).
4. Keep the "no org scope → all active memberships" behaviour only when `resolved_tenant` is the explicitly-public variant (if applicable); otherwise require a resolved org.
5. Update each caller site that passes `HeaderMap` to `resolve_user_roles` (`get_onboarding_status`, `post_onboarding_progress`, etc.) to pass the extractor; the router stays the same because axum resolves the extractor from the request parts.
6. Delete the now-dead `X-Tenant-ID` import / use if no other code in `onboarding.rs` reads it directly.
7. Rebuild openapi via `pnpm generate-api` — the request shape is unchanged (extractors don't appear in OpenAPI), but regeneration catches any accidental schema drift.

## Alternatives considered
- **Keep the hand-rolled block but fix the fallback** — i.e., require org_scope to be non-None in the final membership filter. Rejected because it leaves onboarding on a divergent tenant-resolution path. #3332's whole point was consolidation; re-introducing bespoke logic undoes that guarantee.
- **Reject requests without an X-Tenant-ID header entirely** — rejected because the onboarding client legitimately sends no header on first-login flows (it's exploring which orgs the user has). ResolvedTenant handles that case via its documented precedence.

## Root-cause trace
1. Symptom: a login-token request with `X-Tenant-ID: <foreign-org>` returns role suggestions from the user's own memberships without the handler having consulted any host/tenant resolution.
2. ← At `backend/servers/api-server/src/routes/onboarding.rs:134`, `org_scope = header_org.or(claim_org)` evaluates to `None` for a login token, so the membership filter at line 150-151 passes every row.
3. ← The handler never called `ResolvedTenantExtractor`; it predates the Round-5 consolidation.
4. Origin: pre-ResolvedTenant era (onboarding was written before the host-tenant middleware existed); PR #3332 identified the gap but deferred the migration.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/onboarding_resolved_tenant_tests.rs` — new integration test: user in org A, login token (no org_id), `X-Tenant-ID: <org-B>` → 403.
- [ ] Same test file — user in org A, login token (no org_id), no X-Tenant-ID → 200 with org-A memberships only (public/unresolved path).
- [ ] Same test file — user in org A, org-scoped token (`org_id == org-A`), `X-Tenant-ID: <org-B>` → 403 TENANT_MISMATCH.
- [ ] `cargo test -j 1 -p api-server --test suite_7 onboarding_resolved_tenant_tests` locally.
- [ ] `check-rls-enforcement.sh --strict` (should remain clean).

## Out of scope
- Rewriting `resolve_user_roles` into a shared utility used by other role-selection handlers — carry as a follow-up refactor if the pattern repeats.
- Auditing remaining `X-Tenant-ID` reads across the codebase (there are legitimate ones in deploy/health paths) — survey separately.

## After-merge
- Move this file to `plans/_archive/security-onboarding-resolved-tenant-rule-missing.md`.
- Mark the matching `backlog.json` row `security-onboarding-resolved-tenant-rule-missing` as `status: "done"`.
