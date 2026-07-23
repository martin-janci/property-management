# security-add-evidence-cross-tenant-idor-issue-2483

**Vector:** security
**Score:** 3
**Source:** Issue #2483 | PR #2450 (missed handler)
**Confidence:** high

## Hypothesis

The `add_evidence` dispute sub-resource handler at `backend/servers/api-server/src/routes/disputes.rs:803` is a cross-tenant write IDOR — any authenticated user can inject arbitrary evidence rows (`filename`, `content_type`, `size_bytes`, `storage_url`, `description`) onto any org's dispute by enumerating `dispute_id`. PR #2450 hardened five sibling handlers (`list_parties`, `add_party`, `list_evidence`, `delete_evidence`, `list_activities`) with `require_org(&user)?` + `WHERE EXISTS (SELECT 1 FROM disputes WHERE id = $1 AND organization_id = $N)` guards, but the sixth handler on the same table family was left untouched. Mirror the #2450 pattern: thread `organization_id` into the handler, gate the repo `INSERT` on the disputes-scope EXISTS, and add a `#[sqlx::test]` proving a cross-org insert becomes `NotFound` with no row written.

## Evidence

- `backend/servers/api-server/src/routes/disputes.rs:803` — `add_evidence` handler body has no `require_org(&user)?` call and calls `state.dispute_repo.add_evidence(evidence)` with no `org_id` argument (contrast the neighboring `list_evidence` at :795 which threads `require_org`).
- `backend/crates/db/src/repositories/dispute.rs:620` — `pub async fn add_evidence(&self, req: AddEvidence)` performs an unconditional `INSERT INTO dispute_evidence (...) VALUES (...)`, no `WHERE EXISTS (... organization_id = $)` guard.
- PR #2450 review notes the `dispute_*` sub-resource tables are **not** RLS-protected on this pool — RLS does not backstop the missing app-layer check.
- GitHub issue #2483 (opened 2026-07-23 by post-merge review of #2450) documents the finding, threat model, and the exact patch shape to apply.
- Sibling `#[sqlx::test]` cases live at `backend/servers/api-server/tests/suites/dispute_cross_org_idor_tests.rs` (R4–R8, added by PR #2450) — the R9 case for `add_evidence` follows the same seed shape.

## Files

- `backend/servers/api-server/src/routes/disputes.rs:803`
- `backend/crates/db/src/repositories/dispute.rs:620`
- `backend/servers/api-server/tests/suites/dispute_cross_org_idor_tests.rs`

## Dependencies

None — the plan lands on top of merged #2450 which is already on `dev`.

## Required capabilities

- [x] C1 — Systematic debugging (security bug)
- [x] C2 — Seed data (sqlx-test seeds two orgs + disputes)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
- Mode: cloud-ok

## Repro steps

1. Seed org A + org B; create a dispute in org A (`dispute_A`); authenticate as a user in org B.
2. `POST /api/v1/disputes/{dispute_A.id}/evidence` with a valid `AddEvidenceRequest` body while carrying org B's `X-Org-Id` / bearer.
3. Expected: `404 NotFound` and `SELECT COUNT(*) FROM dispute_evidence WHERE dispute_id = $dispute_A.id` returns `0`.
4. Actual (today): `201 Created` — evidence row is written onto org A's dispute; `dispute_activities` records an `EVIDENCE_ADDED` entry attributed to the org B user.

## Suggested approach

1. In `routes/disputes.rs::add_evidence`, add `let organization_id = require_org(&user)?;` at the top, then call `state.dispute_repo.add_evidence(evidence, organization_id).await` (adjust arity).
2. In `crates/db/src/repositories/dispute.rs::add_evidence`, change the signature to accept `organization_id: Uuid`, and rewrite the `INSERT` to `INSERT INTO dispute_evidence (...) SELECT $1, $2, ..., $8 WHERE EXISTS (SELECT 1 FROM disputes WHERE id = $1 AND organization_id = $9) RETURNING ...` with `fetch_optional` + `ok_or(AppError::NotFound(...))`.
3. Move the follow-up `record_activity` call inside the `Ok(...)` branch so no `dispute_activities` row is written when the insert is skipped.
4. Add `add_evidence_is_scoped_to_owning_org` to `tests/suites/dispute_cross_org_idor_tests.rs` (R9): seed cross-org fixtures, `add_evidence` from org B → assert `NotFound` + `COUNT(*) == 0` + `dispute_activities` unchanged. Also add an owning-org happy path to prove the guard doesn't over-block.
5. Run `cargo test -p api-server --test suites -- dispute_cross_org_idor::add_evidence` locally and inside CI.
6. Close #2483 in the PR body (`Closes #2483`).

## Alternatives considered

- **RLS-only enforcement (rely on Postgres row-level security to reject the write)** — rejected because the PR #2450 review explicitly notes `dispute_*` sub-resource tables are not RLS-protected on this pool; adding the policy is a larger schema change out of scope for a targeted IDOR fix.
- **Return 403 Forbidden instead of 404** — rejected because 403 leaks the fact that `dispute_id` exists in another org (an existence oracle for enumerating tenants); 404 matches the sibling handlers PR #2450 landed and preserves resource-existence privacy.

## Root-cause trace

1. Symptom: any authenticated user can `POST /api/v1/disputes/{foreign-dispute-id}/evidence` and successfully write a row.
2. ← Handler `add_evidence` at `backend/servers/api-server/src/routes/disputes.rs:803` accepts `AuthUser` but never calls `require_org` or forwards `org_id`.
3. ← Repo `add_evidence` at `backend/crates/db/src/repositories/dispute.rs:620` runs an unconditional `INSERT` — no scope predicate.
4. Origin: PR #2450 (issue #2441) org-scoped five of six `dispute_*` sub-resource handlers; `add_evidence` was omitted from the sweep because the R4–R8 scope was drawn around the `list_*` / `add_party` / `delete_evidence` set.

## Test plan

- [ ] `backend/servers/api-server/tests/suites/dispute_cross_org_idor_tests.rs::add_evidence_is_scoped_to_owning_org` — cross-org POST returns `NotFound`, no `dispute_evidence` row, no `dispute_activities` entry.
- [ ] `backend/servers/api-server/tests/suites/dispute_cross_org_idor_tests.rs::add_evidence_owning_org_succeeds` — happy-path regression proving the guard does not over-block owning-org callers.
- [ ] `cargo test -p api-server --test suites -- dispute_cross_org_idor::add_evidence`

## Out of scope

- Adding RLS policies for the `dispute_*` sub-resource tables (larger schema effort).
- Refactoring `dispute_activities.record_activity` — the plan only gates when it is called, not its internals.
- Any of the other five handlers PR #2450 already covered.

## After-merge

- Move this file to `plans/_archive/security-add-evidence-cross-tenant-idor-issue-2483.md`
- Mark the matching `backlog.json` row (`security-add-evidence-cross-tenant-idor-issue-2483`) as `status: "done"`
