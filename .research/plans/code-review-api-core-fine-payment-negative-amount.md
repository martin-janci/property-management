# code-review-api-core-fine-payment-negative-amount

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review 2026-10-31 (api-core); commit cdca04e (HEAD at review time)
**Confidence:** medium

## Hypothesis

`POST /api/v1/violations/{id}/actions/{action_id}/payments` (`record_payment`)
accepts a `RecordFinePayment { amount: Decimal }` body with no sign or zero
check at the handler, model, or repository layer. The repository runs
`paid_amount = COALESCE(paid_amount, 0) + $2` so a **negative amount decreases
`paid_amount`**, while the status / `paid_at` `CASE` arms only ever flip
forward to `'paid'` / `NOW()` and never back. An attacker with a legitimate
org-member session (or a manager who can `record_payment` on an own-org
violation) can therefore silently rewrite `enforcement_actions.paid_amount`
downward while audit history keeps the row at `status='paid'`. The smallest
fix is a `req.amount > 0` (strictly positive) check at the handler before
the repo call, with a 400 and a validator attribute (`#[validate(range)]` or
hand-rolled serde) on the model for defence in depth.

## Evidence

- `backend/servers/api-server/src/routes/violations.rs:558` — `record_payment` passes `req: RecordFinePayment` straight to `enforcement_repo.record_payment(action_id, org_id, req, user.user_id)` with no amount validation.
- `backend/crates/db/src/models/violations.rs:388` — `pub struct RecordFinePayment { pub amount: Decimal, ... }` with no `#[validate(...)]` attribute, no `Deserialize` guard.
- `backend/crates/db/src/repositories/violations.rs:961` — `record_payment` method; see line ~1022 for the `COALESCE(paid_amount, 0) + $2` arithmetic and the forward-only status / `paid_at` arms.
- `decide_appeal` has the mirrored sign-inversion flaw: `fine_adjustment = GREATEST(0, fine_amount - $2)` at `backend/crates/db/src/repositories/violations.rs:854` means a negative adjustment INCREASES the fine (tracked separately as `code-review-api-core-decide-appeal-adjustment-sign`).

## Files

- `backend/servers/api-server/src/routes/violations.rs:558`
- `backend/crates/db/src/repositories/violations.rs:961`
- `backend/crates/db/src/models/violations.rs:388`

## Dependencies

<!-- none -->

## Required capabilities

- [x] C1 — Systematic debugging (bug / security — reach-the-call-site before fix)
- [x] C2 — Seed data (needed to spin up a violations+action+appeal fixture for the regression test)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion (cargo test + fmt + clippy on api-server + db crate)
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. Create a violation + enforcement_action (`#[sqlx::test]` fixture seeds org, user, violation, action with `fine_amount=100.00` and `paid_amount=0`).
2. `POST /api/v1/violations/{v}/actions/{a}/payments` with body `{"amount": "-50.00", "payment_method": "cash"}` as an org manager.
3. Expected: `400 BAD_REQUEST` with message naming the amount constraint.
4. Actual (today): `201 CREATED`; the row's `paid_amount` goes to `-50.00`; `status` stays whatever it was (and the next legitimate payment that *reaches* `fine_amount` flips `status='paid'` + `paid_at=NOW()` even though the actual net received is wrong).
5. Second repro: `POST ... {"amount": "0.00", ...}` — expect 400; today it's accepted (creates a zero-amount `fine_payments` audit row that still flips status if `paid_amount >= fine_amount` by coincidence).

## Suggested approach

1. In `backend/servers/api-server/src/routes/violations.rs:558`, after `require_manager(...)` and before the repo call, add a strict guard:
   ```rust
   if req.amount <= Decimal::ZERO {
       return Err(bad_request_error("amount must be a positive decimal"));
   }
   ```
   Reuse `bad_request_error` from the same module (used elsewhere in `violations.rs`).
2. Belt-and-braces: add `#[validate(range(exclusive_min = 0))]` to `RecordFinePayment::amount` in `backend/crates/db/src/models/violations.rs:388` (if the project already uses the `validator` crate — check `Cargo.toml`; if not, keep the guard in step 1 only — don't add a new dep in a security hotfix).
3. Add a regression test file `backend/servers/api-server/tests/suites/violations_record_payment_amount_guard_tests.rs` with cases: negative amount → 400, zero amount → 400, positive amount → 201 + `paid_amount` increments correctly, two positive partial payments that total >= `fine_amount` → status flips to 'paid'.
4. Keep the `record_payment` repository method unchanged — the invariant lives at the handler + model layer; the repo is a passthrough for other callers (if any). Verify with `grep -rn 'enforcement_repo\.record_payment\|repositories::violations::.*record_payment\b' backend/`.
5. Audit sibling monetary paths in the same handler file for the mirrored flaw: `update_action` (sets `fine_amount` directly, see line 802's `.bind(req.fine_adjustment)` from `UpdateEnforcementAction`) and `decide_appeal` (see `code-review-api-core-decide-appeal-adjustment-sign`). This plan does NOT touch those — they are tracked separately — but note the audit in the PR description so the reviewer knows the scope boundary.

## Alternatives considered

- **DB CHECK constraint on `fine_payments.amount > 0`** — rejected because it protects the row-insert path but doesn't surface a meaningful HTTP error to the caller; the client gets a 500 from a constraint violation. A handler guard gives a clean 400 and the DB constraint can land as a follow-up migration.
- **Validate at the repository layer only** — rejected because the model + repo are reused by other binaries (sqlx test fixtures, admin tools); the handler is the HTTP boundary and that's where RFC 7807 400s belong. Repo-only validation also makes unit-testing the handler contract harder.

## Root-cause trace

1. Symptom: `enforcement_actions.paid_amount` can be rewritten downward through the public API; status column may stay at `'paid'` while balance goes wrong.
2. ← `record_payment` handler at `backend/servers/api-server/src/routes/violations.rs:558` passes `req` through unchecked.
3. ← `enforcement_repo.record_payment` at `backend/crates/db/src/repositories/violations.rs:961` runs `paid_amount = COALESCE(paid_amount, 0) + $2` with no sign guard.
4. ← `RecordFinePayment { amount: Decimal }` at `backend/crates/db/src/models/violations.rs:388` has no validator; serde happily deserialises `"-50.00"` into a valid `Decimal`.
5. Origin: initial addition of the fine-payment surface — pre-dates the recent IDOR hardening work on this file (PRs #3171/#3180/#3183/#3188).

## Test plan

- [ ] New `backend/servers/api-server/tests/suites/violations_record_payment_amount_guard_tests.rs` — negative, zero, positive, overpay cases.
- [ ] Red-before: run the new test file at origin/dev — the negative + zero cases return 2xx (fail). After fix: all cases pass.
- [ ] Regression: existing `violations_role_authz_tests.rs` + `violations_mutation_cross_org_idor_tests.rs` suites stay green.
- [ ] `cd backend && cargo test -p api-server --test suite_8 -- violations_record_payment_amount_guard` (or the test name under whichever `tests/suite_*.rs` includes the new module — register it next to the sibling violations tests).
- [ ] `cd backend && cargo fmt --all -- --check && cargo clippy -p api-server --all-targets -- -D warnings`.

## Out of scope

- `decide_appeal` sign inversion (tracked as `code-review-api-core-decide-appeal-adjustment-sign`).
- `update_action` free-form fine_amount write (same model, different method — audit noted; separate plan if confirmed).
- Nested-route parent-id ignored defect (tracked as `code-review-api-core-nested-parent-id-ignored`).
- DB CHECK constraint migration (follow-up after the handler guard lands).
- Any sqlx offline-data regeneration — only needed if the new test adds a new query, which it does not (it hits the public HTTP layer).

## After-merge

- Move this file to `plans/_archive/code-review-api-core-fine-payment-negative-amount.md`
- Mark the matching `backlog.json` row (`code-review-api-core-fine-payment-negative-amount`) as `status: "done"`
