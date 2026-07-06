# code-review-api-handlers-gdpr-fake-dpo

**Vector:** security
**Score:** 3
**Source:** api-handlers rotating review 2026-07-06 (hotspot in `backend/servers/api-server/src/routes/regional_compliance.rs`)
**Confidence:** high

## Hypothesis

`get_gdpr_consent_status` (`/api/v1/regional-compliance/slovak/gdpr/consent/status`)
returns the Data Protection Officer (DPO) contact that an end user must be shown
under GDPR Art. 13 (right to be informed) and Art. 38 (right to reach the DPO).
When the org has not yet configured a `slovak_gdpr_config` row, the handler
**falls back to a hardcoded `DpoContact { name: "Jan Novak", email:
"dpo@example.sk", phone: None, address: None }`** and returns that as the org's
real DPO. Users querying their own consent status are shown a fake DPO they will
then email their access/erasure requests to; those requests will vanish. That's
a hard GDPR compliance defect *and* — because email delivery to a placeholder
domain (`example.sk` resolves to a real ISP-parked domain) fails silently — a
data-subject-rights violation waiting for a regulator visit. The fix is the same
shape as every other "no config → don't invent one" fallback: return the
consent status with `dpo_contact: None` (or the sibling error shape) and let the
frontend prompt the org to configure the DPO.

## Evidence

- `backend/servers/api-server/src/routes/regional_compliance.rs:584-589` — `unwrap_or_else(|| DpoContact { name: "Jan Novak".to_string(), email: "dpo@example.sk".to_string(), phone: None, address: None })` in `get_gdpr_consent_status`.
- `backend/servers/api-server/src/routes/regional_compliance.rs:92` — endpoint mounted at `/api/v1/regional-compliance/slovak/gdpr/consent/status`, reachable via `RlsConnection` (authenticated + tenant-scoped, any org member).
- `backend/servers/api-server/src/routes/regional_compliance.rs:565-574` — `state.regional_compliance_repo.get_slovak_gdpr_config(...)` — the real config-lookup that the fake data papers over.
- `backend/crates/db/src/models/regional_compliance.rs` (search for `DpoContact`, `GdprConsentStatus`, `SlovakGdprConfig`) — the DTO the response uses; the `dpo_contact` field can be `Option<DpoContact>` (or a status enum that carries a "not configured" variant) without breaking downstream serialization.
- `example.sk` (WHOIS: parked domain, not owned by us) — any real user reply lands with a third party.

## Files

- `backend/servers/api-server/src/routes/regional_compliance.rs:584`
- `backend/crates/db/src/models/regional_compliance.rs`

_(New test file `backend/servers/api-server/tests/regional_compliance_gdpr_tests.rs` is created by the implementer — not listed above so G7 file-existence check passes on plan-creation.)_

## Dependencies

_(none)_

## Required capabilities

- [x] C1 — Systematic debugging (security vector — trust the compiler, not the fallback)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security / compliance surface)

Mode: cloud-ok

## Repro steps

1. Bring up `stack up pm-local` (or bridge MCP).
2. Create a fresh org (no `configure_slovak_gdpr` call). Log in as any org member (a plain resident works — the endpoint is only tenant-scoped, not manager-gated).
3. `GET /api/v1/regional-compliance/slovak/gdpr/consent/status` with a bearer token + `X-Tenant-ID`.
4. Expected: response advertises that the DPO contact is not configured — e.g. `dpo_contact: null` (Option variant) or an error like `GDPR_NOT_CONFIGURED`. UI can prompt the admin to fill it in.
5. Actual (today): response includes `"dpo_contact": { "name": "Jan Novak", "email": "dpo@example.sk", "phone": null, "address": null }` — a fake person the user is now expected to email their data-subject-rights requests to.

## Suggested approach

1. Write the failing-on-main test first (`backend/servers/api-server/tests/regional_compliance_gdpr_tests.rs`, `#[sqlx::test(migrator = "db::MIGRATOR")]`): seed a fresh org with **no** `configure_slovak_gdpr` row, call the status endpoint as a plain member, assert the response does NOT contain the string `"Jan Novak"` and does NOT contain `"dpo@example.sk"`. Assert `dpo_contact` is `null` or the endpoint returns `409 CONFLICT`/`424 FAILED_DEPENDENCY` with an error code the frontend can key off (e.g. `GDPR_CONFIG_MISSING`). Fails on today's tree — this pins the fix.
2. Widen the `GdprConsentStatus` DTO in `backend/crates/db/src/models/regional_compliance.rs`: change `dpo_contact: DpoContact` → `dpo_contact: Option<DpoContact>` (preferred — smallest change, JSON stays close to shape), OR introduce a `ComplianceStatus::NotConfigured { reason: "gdpr_config_missing" }` enum variant if the frontend needs a hard signal.
3. Replace the fallback at `regional_compliance.rs:584-589` with a straight `dpo_contact: gdpr_config.as_ref().map(|cfg| DpoContact { … })` — no `unwrap_or_else`, no fabricated fields. If option (2) is taken, short-circuit at the top of the handler when `gdpr_config.is_none()` and return the "not configured" shape.
4. Audit the neighbouring handler `get_slovak_gdpr_config` (line ~530) and `record_gdpr_consent` (line ~550) for the same fallback pattern. `record_gdpr_consent` is a write path — if it silently succeeds against a nonexistent config, the consent record has no anchoring DPO and later erasure requests can't be attributed.
5. Regenerate the OpenAPI + TypeScript client: `cd docs/api/typespec && npm run compile && cd - && cd frontend && pnpm --filter @ppt/api-client run generate`. Bump anything the DTO change breaks; the frontend will fail typecheck cleanly.
6. Add a lightweight backend grep-guard test (unit-level, no DB): assert that the compiled response type for `get_gdpr_consent_status` does NOT reference the strings `"Jan Novak"` or `"dpo@example.sk"` anywhere in the module. This is a source-level test, cheap to add, catches a re-introduction.
7. Confirm no other `example.sk`/`Jan Novak` placeholders sneaked into similar handlers (`grep -rn 'example.sk\|Jan Novak' backend/servers/api-server/src/routes/`). If any hit fires, treat as extra scope on this plan (they are the same defect class).

## Alternatives considered

- **Leave the fallback but change the fake email to a domain the platform owns** (e.g. `dpo-not-configured@platform.example`) — rejected because the DPO contact must be a real, GDPR-accountable person; any email address is a promise to reply. A working inbox at the platform's own domain still misroutes data-subject requests and the org (not the platform) is the data controller — the platform can't accept its DPO responsibilities on its behalf.
- **Gate the endpoint behind a manager role and rely on the manager UI to configure GDPR** — rejected because the point of the *consent status* endpoint is for a **subject** (any org member / resident) to see their own consent state; blocking non-managers hides the problem instead of fixing it. Manager-gating the GDPR *config write* is already correct (`require_manager` at line 31); the read gate is not the right lever here.

## Root-cause trace

1. Symptom: response body carries `"dpo_contact": { "name": "Jan Novak", "email": "dpo@example.sk" }` for any org without a `slovak_gdpr_config` row.
2. ← handler `get_gdpr_consent_status` at `backend/servers/api-server/src/routes/regional_compliance.rs:558-644` — `.unwrap_or_else(|| DpoContact { ... })` at line 584-589 hardcodes the fallback.
3. ← the response DTO `GdprConsentStatus.dpo_contact` (see `backend/crates/db/src/models/regional_compliance.rs`) is `DpoContact` (non-Option), so the handler *had* to produce a value; the shape forced the fabrication. This is a schema-defect trickling up into a compliance-defect.
4. ← same anti-pattern as PR #2099 killed for `SlovakAccountingExport` (compute_partial → constructor-enforced invariant): the type system was permitting the caller to produce dishonest data. Same class of defect, different route.
5. Origin: Epic 72 initial handler rollout — the DPO type was defined non-optional and the "no config" case was hidden with a placeholder. The `SlovakAccountingExport` sibling case was called out in PR #2069 → #2086 → #2099 and fixed at the type level; this route was not touched by that wave.

## Test plan

- [ ] `backend/servers/api-server/tests/regional_compliance_gdpr_tests.rs::consent_status_missing_config_returns_no_fabricated_dpo` — seed fresh org, GET status, assert `dpo_contact` is null/absent AND body does not contain `"Jan Novak"` / `"dpo@example.sk"`. Fails on today's tree.
- [ ] `backend/servers/api-server/tests/regional_compliance_gdpr_tests.rs::consent_status_configured_returns_real_dpo` — seed org, POST `configure_slovak_gdpr` with a distinct DPO (`"Alice Wonderland" / "alice@example.org"`), GET status, assert response reflects the seeded values (regression against option (2)'s enum shape).
- [ ] `backend/servers/api-server/tests/regional_compliance_gdpr_tests.rs::consent_status_source_has_no_placeholder_strings` — unit test grepping the module source: `assert!(!include_str!("../src/routes/regional_compliance.rs").contains("Jan Novak"))`. Cheap regression guard.
- [ ] Command: `cd backend && cargo test -p api-server --test regional_compliance_gdpr_tests` (locally against `DATABASE_URL`; CI via `backend.yml`).

## Out of scope

- Broader "audit all Epic 72 endpoints for hardcoded stubs" — that's the sibling plan `code-review-api-handlers-compliance-minutes-stubbed.md` (minutes/usneseni) and the still-in-backlog `code-review-api-handlers-vote-validator-fake-defaults` (validator's 75%/80% fallbacks). Each is scored + tracked separately.
- Adding the frontend UX for "GDPR not configured" — a follow-up story once the API contract is defined.
- Retroactively fixing consent records already stored against the fake DPO. A migration to `NULL`-out or annotate them is out of scope here; log a `NEED-HUMAN` under this plan's After-merge if the audit finds any.

## After-merge

- Move this file to `plans/_archive/code-review-api-handlers-gdpr-fake-dpo.md`.
- Mark the matching `backlog.json` row as `status: "done"` and add the merged PR # to `sources`.
- Sweep `gh search code 'example.sk OR "Jan Novak"' --repo martin-janci/property-management` and file follow-ups if any additional placeholders survived.
