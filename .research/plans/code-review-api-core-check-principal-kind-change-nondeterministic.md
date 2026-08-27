# code-review-api-core-check-principal-kind-change-nondeterministic

**Vector:** security
**Score:** 2
**Source:** api-core segment review 2026-08-27; PR #2864 (sibling fix); GH issue #2861
**Confidence:** high

## Hypothesis
`AuthPolicyEnforcer::check_principal_kind_change` in `auth_policy.rs` still calls `MembershipRepository::list_for_user(...).await?; memberships.first()` before `self.policy_for(m.organization_id)` — the exact non-deterministic-`first()` anti-pattern that PR #2864 eradicated from `check_capability_grant_for_user` two commits earlier. Because `list_for_user` has no `ORDER BY` (already documented in the file's own docstring at lines 213-217), the outcome depends on Postgres's row-visit order. When a subset of the grantee's org rows has a corrupt/unloadable `org_auth_policies` row, the check succeeds if a healthy org is surfaced first and fails otherwise — the same fail-open-by-row-order regression class issue #2857 was security-labeled for. The smallest fix is to mirror `check_capability_grant_for_user`: fold `platform_default_policy()` in and iterate over every org (as `strictest_policy_across` already does), removing the `.first()` shortcut entirely.

## Evidence
- `backend/servers/api-server/src/services/auth_policy.rs:256-259` — the offending block: `let memberships = mem_repo.list_for_user(target_user_id).await?; if let Some(m) = memberships.first() { let _policy = self.policy_for(m.organization_id).await?; }`
- `backend/servers/api-server/src/services/auth_policy.rs:213-217` — file docstring already warns `list_for_user` returns rows in arbitrary order (that warning is what PR #2864 landed).
- PR #2864 (merged 2026-08-27, closes issue #2861) — fixed the same anti-pattern in `check_capability_grant_for_user` and added two sqlx regression tests at `auth_policy.rs:442-491`.
- Issue #2857 — security label on the parent bug class ("Fail-open non-deterministic email-verification gate").
- `backend/servers/api-server/src/services/auth_policy.rs:107` — the sibling `strictest_policy_across` seeds `effective = self.platform_default_policy()` before unioning per-org policies; that is the correct pattern to mirror.

## Files
- `backend/servers/api-server/src/services/auth_policy.rs`
- `backend/crates/db/src/repositories/membership.rs`
- `backend/crates/db/src/repositories/membership_test.rs`

## Dependencies
_None — sibling fix PR #2864 already merged._

## Required capabilities
- [x] C1 — Systematic debugging (bug/security vector: always tick)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks — C4/C5 unticked → cloud-ok):**

Mode: cloud-ok

Backend Rust unit + sqlx tests only; no Chrome, no ADB — the cloud dispatcher runs `cargo test -p api-server` via the ppt-bridge MCP.

## Repro steps
1. Seed a user with two active `user_memberships` rows (`org-A`, `org-B`); mark `org-B.org_auth_policies` as absent or unloadable (e.g. delete the row).
2. Call any code path that invokes `check_principal_kind_change` for that `target_user_id`.
3. Repeat the call in a fresh transaction (or use `pg_prewarm` / index-hint tricks) to swap Postgres's visit order for the two membership rows.
4. **Expected:** the check either consistently errors (strict-across-orgs) or consistently passes with `platform_default_policy`. **Actual:** it flips between `Ok(())` (when `org-A` is surfaced first) and an error (when `org-B` is surfaced first) — same fail-open-by-row-order class as #2857.

## Suggested approach
1. In `auth_policy.rs:256-259`, delete the `if let Some(m) = memberships.first()` block.
2. Replace with the same pattern `check_capability_grant_for_user` now uses post-#2864: fold `platform_default_policy()` in first, then iterate every membership row to short-circuit as soon as any org's `policy_for` disagrees with the incoming kind change (or, if the method's semantics only need liveness, just call `platform_default_policy()` and drop the per-org lookup entirely — verify with the caller in `routes/admin/capabilities.rs`).
3. Update the docstring above the method to mirror the one on `check_capability_grant_for_user` (post-#2864 wording) so future readers see the invariant.
4. Add one sqlx test in `auth_policy.rs`'s `#[cfg(test)]` block modelled on `capability_grant_rejected_when_any_org_requires_verification` (lines 442-491): seed the two-org setup from *Repro steps*, assert the outcome is stable regardless of `INSERT` order of the two `user_memberships` rows.
5. Add a mirror test in `backend/crates/db/src/repositories/membership_test.rs` if the repository-level invariant needs pinning (mirroring what PR #2864 added there).
6. `cd backend && cargo test -p api-server auth_policy` to run the new tests locally / in the impl agent's verify step.

## Alternatives considered
- **Add ORDER BY to `list_for_user`** — rejected because PR #2864's approach (iterate all rows / fold platform default) is stronger: even a deterministic order can pick the wrong (lax) org. The consensus fix is ordering-independent.
- **Suppress the finding as latent** — rejected because the sibling anti-pattern was security-labeled by issue #2857 and the fix is trivial (< 10 lines). Waiting for a live incident to promote this signal recreates the exact "auth policy fail-open in production" story #2857 already burned a cycle on.

## Root-cause trace
1. Symptom: non-deterministic `check_principal_kind_change` outcome when at least one grantee org has an unloadable `org_auth_policies` row.
2. ← `memberships.first()` at `backend/servers/api-server/src/services/auth_policy.rs:257` picks an arbitrary member of an unordered `Vec`.
3. ← `MembershipRepository::list_for_user` (in `backend/crates/db/src/repositories/membership.rs`) has no `ORDER BY` clause; Postgres returns rows in physical-heap order which shifts with `VACUUM`/`REINDEX`/write patterns.
4. Origin: the anti-pattern was never introduced by a specific commit — it's the *same* pattern PR #2864 fixed in `check_capability_grant_for_user`; PR #2864 (2026-08-27, closes #2861) only cleaned the one call site, leaving `check_principal_kind_change` (this method) and any future `.first()` call on `list_for_user` still exposed.

## Test plan
- [ ] `backend/servers/api-server/src/services/auth_policy.rs` — sqlx test: `principal_kind_change_stable_when_any_org_membership_unloadable` (two-org seed as in *Repro steps*; assert `check_principal_kind_change` outcome is stable across insertion orders).
- [ ] Command: `cd backend && cargo test -p api-server auth_policy::tests::principal_kind_change` — new test must fail on `main` and pass after the fix (IG3).

## Out of scope
- Refactoring `list_for_user` to always `ORDER BY organization_id` — separate churn-hotspot follow-up (see `churn-hotspot-backend/servers/api-server/src/services/auth_policy.rs`); we intentionally solve the fail-open one call site at a time to keep the diff reviewable.
- Adding a lint that flags any `.first()` on `list_for_user` — nice-to-have but the file is small enough that reviewers catch this now.
- The other two api-core findings surfaced in the same review (`check_login-no-platform-default`, `check_password_change-strictest-untested`) — separate backlog rows.

## After-merge
- Move the plan to `.research/plans/_archive/` and set the backlog row to `status=done`.
- Update `docs/api/README.md`'s auth-policy section if the docstring change materially changes the invariant description.
- Re-run the api-core segment review next cycle to confirm no other `.first()` on `list_for_user` slipped through.
