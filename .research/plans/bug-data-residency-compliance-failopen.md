# bug-data-residency-compliance-failopen

**Vector:** bug
**Score:** 3
**Source:** signals `code-review-api-handlers-residency-compliance-failopen` (+2) and `code-review-api-handlers-residency-chain-valid-failopen` (+1) — Tier-1d dispatcher generator, 2026-08-10
**Confidence:** medium

## Hypothesis
The data-residency dashboard and audit-log handlers in `backend/servers/api-server/src/routes/data_residency.rs` fail OPEN in two spots: (1) the dashboard's top-level `compliance_status` uses `.unwrap_or(true)` when the org has never been verified, so a never-audited organization reads back as `Compliant` — the summary field consumers use to decide whether to act; and (2) the audit-log route reads the audit-chain tamper-evidence flag as `verification["chain_valid"].as_bool().unwrap_or(true)`, so if the chain field is ever absent (missing, renamed, or on a future repo variant) the API reports the chain as valid. Both are one-line policy inversions: for a *compliance* / *tamper-evidence* signal the safe default is fail CLOSED (`NonCompliant`, `false`), never fail OPEN. The fix is small, mechanical, and testable.

## Evidence
- `backend/servers/api-server/src/routes/data_residency.rs:1040-1048` — dashboard handler: `compliance_status: if last_verification.as_ref().map(|v| v.is_compliant).unwrap_or(true) { Compliant } else { NonCompliant }`. When `last_verification` is `None` (org never verified), the branch evaluates to `Compliant`.
- `backend/servers/api-server/src/routes/data_residency.rs:936-941` — `last_verification` is `None` exactly when `data_residency_repo.get_latest_verification_result(org_id)` returns `None`, i.e. the organization has never had a compliance check run.
- `backend/servers/api-server/src/routes/data_residency.rs:736` — audit-log route: `let chain_valid = verification["chain_valid"].as_bool().unwrap_or(true);` — if the repo ever omits the `chain_valid` key, the endpoint reports the tamper-evidence chain as valid.
- `backend/crates/db/src/repositories/data_residency.rs:552-596` — today the repo always emits `chain_valid`, so the missing-key branch is currently unreachable in production; but the fail-open default is a latent trap if the repo shape changes or a future migration introduces a null.
- Consumers who key off `compliance_status` (dashboards, audit exports, regional-compliance drill-downs) treat a `Compliant` return as "no action needed", which is exactly wrong for an unverified org.

## Files
- `backend/servers/api-server/src/routes/data_residency.rs`
- `backend/crates/db/src/repositories/data_residency.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug — trace which downstream views key off `compliance_status`)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):** `Mode: cloud-ok`

## Repro steps
1. Bring up api-server against a database containing an org that has no `data_residency_verifications` rows (never verified).
2. Call `GET /api/v1/data-residency/organizations/{org_id}/dashboard` as that org.
3. Expected: `compliance_status: "Unverified"` (or `NonCompliant`, or explicit `null` — anything but `Compliant`), so a dashboard tile / alert can fire.
4. Actual: `compliance_status: "Compliant"` — the org is presented to the caller as fully in-region and audited when nothing has been checked.
5. Independent second repro (chain-valid): construct a JSON value where `chain_valid` is absent (`{"other": true}`) and pass it through the `verification["chain_valid"].as_bool().unwrap_or(true)` expression — the result is `true`, i.e. the chain is claimed valid despite the field being missing.

## Suggested approach
1. In `data_residency.rs:1040-1048`, replace the fail-open ternary with `compliance_status: last_verification.as_ref().map(|v| if v.is_compliant { ComplianceStatus::Compliant } else { ComplianceStatus::NonCompliant }).unwrap_or(ComplianceStatus::Unverified)`; add a new `Unverified` variant to `ComplianceStatus` if it doesn't exist, and expose it in the OpenAPI schema (regenerate the TS client alongside).
2. In `data_residency.rs:736`, change `unwrap_or(true)` to `unwrap_or(false)` and add `tracing::warn!(?verification, "audit_chain verify result missing chain_valid — defaulting to false")` above the flip so the fallback is observable.
3. Add a unit test in `data_residency.rs` (below the handler impls) that constructs a `DataResidencyDashboard` for an org with `last_verification: None` and asserts `compliance_status == ComplianceStatus::Unverified` (fails on main today because it comes back `Compliant`).
4. Add a second unit test for the audit-log path that runs `verification["chain_valid"].as_bool().unwrap_or(false)` against a `serde_json::json!({})` value and asserts `false` (fails on main today because current code returns `true`).
5. Regenerate the OpenAPI spec + TS client if `ComplianceStatus::Unverified` was added (per repo convention: run `pnpm --filter @ppt/api-client build` after TypeSpec regen).
6. Grep any frontend/mobile client for a hard-coded `if (compliance_status === "Compliant") { hide }` to make sure the new `Unverified` state doesn't silently look like "Compliant" downstream; add a follow-up backlog row if a UI adjustment is out of scope for the fix PR.

## Alternatives considered
- **Leave `unwrap_or(true)` and rely on the client to check `last_verification: null`** — rejected because the API contract explicitly exposes `compliance_status` as the summary field; requiring every consumer to also key off `last_verification` inverts the abstraction and re-introduces the same bug in every new client. The server owns the "safe default when I don't know" decision.
- **Return HTTP 409 / 404 when an org has never been verified** — rejected because the dashboard is intentionally a "show me the current state" endpoint that must succeed even on day 0; the fix is to surface an honest "Unverified" state in the payload, not to make the endpoint fail.

## Root-cause trace
1. Symptom: an unverified organization's data-residency dashboard shows `compliance_status: Compliant`.
2. ← `data_residency.rs:1040-1048` — `if last_verification.as_ref().map(|v| v.is_compliant).unwrap_or(true)` evaluates `Compliant` on `None`.
3. ← `last_verification` is `None` because `data_residency.rs:936-941` calls `data_residency_repo.get_latest_verification_result(org_id)`, which returns `Ok(None)` for organizations with no verification rows.
4. ← The dashboard author reached for `unwrap_or` to make the type check compile and chose `true` (Compliant) instead of `false` (NonCompliant / Unverified) — the language required a default, and the safest default for a compliance summary was not picked.
5. Origin: introduced with the data-residency dashboard route (git blame `data_residency.rs:1040` — pre-existing when the tier1d review fired; not from any specific recent PR).

## Test plan
- [ ] `cargo test -p api-server routes::data_residency::` — two new unit tests (dashboard-none-is-unverified, chain-valid-missing-is-false), both failing on `dev` today and green after the fix.
- [ ] `cargo clippy -p api-server --all-targets -- -D warnings` — the `unwrap_or(true)` swap must not trip any dead-code / unused-variant warnings once `ComplianceStatus::Unverified` lands.
- [ ] Regression pin: assert the OpenAPI spec's `ComplianceStatus` enum contains the new `Unverified` value (a snapshot test under `backend/tools/*openapi*` or a grep on the generated JSON is fine).
- [ ] Local run command: `cd backend && cargo test -p api-server routes::data_residency && cargo clippy -p api-server -- -D warnings`

## Out of scope
- Frontend UI treatment of the new `Unverified` state (surface a follow-up if the ppt-web dashboard needs a new tile — not part of this backend-only fix).
- Migrating existing DB rows or backfilling verification records (this plan changes read-side default; write-side / seed-side is a separate story).
- Reworking the audit-chain repo shape to make `chain_valid` non-optional at the type level (worth doing later; today's minimal fix is at the read call site).
- Any other fail-open pattern in `data_residency.rs` outside the two lines cited above.

## After-merge
- Move this file to `plans/_archive/bug-data-residency-compliance-failopen.md`
- Mark `code-review-api-handlers-residency-compliance-failopen` in `backlog.json` as `status: "done"` (the consolidated row already covers `code-review-api-handlers-residency-chain-valid-failopen` — leave the latter's `dropped/consolidated` status alone)
