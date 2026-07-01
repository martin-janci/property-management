# code-review-backend-financial-payment-race

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 static review 2026-07-01
**Confidence:** high

## Hypothesis
`FinancialRepository::allocate_existing_payment` reads the payment and invoice rows outside a transaction, then computes the remaining allocatable amount inside the tx. Two concurrent callers targeting the same payment can both read the same `allocated_total` before either commits, and independently allocate up to that "remaining" figure. The end state is a payment allocated beyond its `amount` — a silent over-allocation bug in a cash-handling path. The smallest fix is to lock the payment row (`SELECT ... FOR UPDATE`) at the top of the transaction and recompute allocated_total from the same tx, before deciding how much this caller can allocate.

## Evidence
- `backend/crates/db/src/repositories/financial.rs:940-1004` — `allocate_existing_payment` body
- Pre-tx reads at `financial.rs:949-969` — payment row fetch and invoice fetch happen on the pool, not the tx
- Allocation math at `financial.rs:971-990` — remaining = payment.amount − sum(allocations); computed after the pre-tx reads, still without a row lock
- Phase 1.5 static review 2026-07-01 (see `.research/signals/2026-07-01.json` entry `code-review-backend-financial-payment-race`)
- Precedent — `fix(finance): record_reserve_transaction concurrency + FOR UPDATE` (#1416) applied the identical `SELECT … FOR UPDATE` pattern to a sibling money-moving path

## Files
- `backend/crates/db/src/repositories/financial.rs:940`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug + concurrency)
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (Postgres for sqlx::test)
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

**Execution mode:**

Mode: cloud-ok

(Postgres-backed sqlx tests run in CI + via ppt-bridge; no Chrome / device needed.)

## Repro steps
1. Seed a payment with `amount = 100`, `allocated_total = 0`, and two open invoices each with `remaining = 100` in the same tenant/org.
2. Spawn two tokio tasks each calling `allocate_existing_payment(payment_id, invoice_a_id, 100)` and `allocate_existing_payment(payment_id, invoice_b_id, 100)` concurrently, with a small delay before the transaction begins to widen the race window.
3. Expected — one call succeeds with 100 allocated, the other fails with "insufficient balance" (or similar).
4. Actual — both succeed, `payment_allocations` shows sum = 200 against a 100-unit payment.

## Suggested approach
1. Inside `allocate_existing_payment`, open the transaction first, then re-fetch the payment row with `SELECT amount, allocated_total FROM payments WHERE id = $1 FOR UPDATE` in the tx. Drop the pre-tx `payment` fetch on the pool.
2. Compute `remaining_allocatable = payment.amount - payment.allocated_total` from the locked row; if `< requested` → return `Insufficient balance` error (same shape as today's error).
3. Re-fetch the invoice inside the tx if invoice mutation depends on its state (a lighter `SELECT ... FOR UPDATE` is fine — invoice sum bookkeeping is the second half of the race).
4. Insert the `payment_allocations` row and update `payments.allocated_total` in the same tx, then commit.
5. Add a `#[sqlx::test]` in `backend/crates/db/tests/financial_allocation_concurrency_tests.rs` that reproduces the repro above using two `tokio::spawn` handles + an explicit `tokio::sync::Barrier` to force overlap; assert exactly one call returns `Ok` and `SELECT SUM(amount) FROM payment_allocations WHERE payment_id = ?` equals `payment.amount`.
6. Do NOT widen scope — leave `record_payment` and `record_reserve_transaction` alone (already hardened in #1361/#1416).

## Alternatives considered
- **Serialisable isolation on the tx** — rejected because most callers set the default `READ COMMITTED`; forcing `SERIALIZABLE` on this one path leaks a transaction-attribute contract into the repository API and would need retry logic for `40001` serialization failures. `FOR UPDATE` on the single row is the local, minimal invariant.
- **App-level mutex keyed on payment_id** — rejected because it doesn't survive across api-server instances; the invariant belongs at the DB level, and `FOR UPDATE` is the standard pattern already used elsewhere in this crate.

## Root-cause trace
1. Symptom: two concurrent allocations against the same payment can both succeed → sum(allocations) > payment.amount (invariant violation, silent).
2. ← `allocate_existing_payment` reads `payment.allocated_total` from the pool (`financial.rs:949-969`) before opening the tx; the value read is stale by the time the tx commits.
3. ← The tx doesn't lock the payment row, so a peer session's UPDATE of `allocated_total` is invisible until commit-vs-commit ordering resolves.
4. Origin: `financial.rs` allocation code — this path pre-dates the `FOR UPDATE` pattern introduced by #1416 for `record_reserve_transaction`; the sibling fix never rippled to `allocate_existing_payment`.

## Test plan
- [ ] `backend/crates/db/tests/financial_allocation_concurrency_tests.rs` — new test `concurrent_allocate_cannot_overspend_payment` fails on `main` and passes after the fix (2 tokio tasks + `Barrier`; asserts one Ok + one Err + `SUM(allocations) == payment.amount`).
- [ ] `cargo test -p db --test financial_allocation_concurrency_tests` → 1/1 pass locally against Postgres 16.
- [ ] `cargo clippy -p db -- -D warnings` clean.
- [ ] Existing `record_payment` regression tests (#1361 lineage) still green.

## Out of scope
- Broader money-moving refactor (record_payment, journal entries, invoice sums)
- API-level idempotency keys on `POST /allocations` (a different layer)
- Anything under `backend/servers/`

## After-merge
- Move this file to `plans/_archive/code-review-backend-financial-payment-race.md`
- Mark the matching `backlog.json` row as `status: "done"`
