# refactor-accounting-metrics-named-struct

**Vector:** refactor
**Score:** 3
**Source:** Issue #2122
**Confidence:** high

## Hypothesis
`get_accounting_metrics` in `backend/crates/db/src/repositories/regional_compliance.rs` returns a 6-tuple `(i32, i32, Decimal, Option<Decimal>, Decimal, Option<Decimal>)` whose four monetary fields (revenue / expenses / receivables / payables) are type-identical and interleaved. The handler destructures positionally at `routes/regional_compliance.rs:431-437` and then feeds `SlovakAccountingExportInput` by name — so swapping revenue↔receivables (or the two `Option`s) at either the return tuple or the destructure compiles cleanly and silently ships wrong accounting figures through the now-safe smart constructor. Convert the return to a named struct (mirroring the `SlovakAccountingExportInput` shape) so field identity is compiler-enforced end-to-end. Verified by two independent reviewers (Phase 1.5 static review + post-merge review issue #2122).

## Evidence
- `backend/crates/db/src/repositories/regional_compliance.rs:493-553` — `get_accounting_metrics` returns the positional 6-tuple.
- `backend/servers/api-server/src/routes/regional_compliance.rs:431-447` — handler destructures `let (invoice_count, payment_count, total_revenue, total_expenses, total_receivables, total_payables) = …;` then passes into `SlovakAccountingExportInput { total_revenue, total_expenses, total_receivables, total_payables, … }` by field-shorthand.
- Issue #2122 confirms: "the PR body's claim that transposing revenue↔receivables 'now fails to compile' is only true for the ctor call, not for this repo→handler seam."
- Class parallel: PR #2117 (`SlovakAccountingExport::new` → `SlovakAccountingExportInput`) killed exactly this transposition class inside the constructor; this plan closes the last hop.

## Files
- `backend/crates/db/src/repositories/regional_compliance.rs:493`
- `backend/servers/api-server/src/routes/regional_compliance.rs:431`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (derived from ticks above — C4 / C5 both unticked):**

Mode: cloud-ok

## Repro steps
1. In `backend/servers/api-server/src/routes/regional_compliance.rs:431-437`, swap two adjacent tuple destructure fields locally (e.g. exchange `total_revenue` and `total_receivables`).
2. Run `cargo check -p api-server`.
3. Expected (after fix): the compiler rejects the swap (fields no longer exist positionally). Actual (today): the code compiles cleanly and the export ships with the two Decimals mislabeled.

## Suggested approach
1. Define `pub struct AccountingMetrics { pub invoice_count: i32, pub payment_count: i32, pub total_revenue: Decimal, pub total_expenses: Option<Decimal>, pub total_receivables: Decimal, pub total_payables: Option<Decimal> }` in `backend/crates/db/src/repositories/regional_compliance.rs` (module-local `pub`; not part of the models crate — this is a repository return shape).
2. Change `get_accounting_metrics` signature from returning the 6-tuple to returning `Result<AccountingMetrics>`. Update the `sqlx::query!(…).fetch_one(&pool).await` block at `regional_compliance.rs:499-553` to construct `AccountingMetrics { … }` by field-shorthand from the SQL result row (SQLx maps column order to field order under-the-hood; the SELECT list stays unchanged).
3. Update the sole call site at `routes/regional_compliance.rs:431-437`: replace the tuple destructure with `let metrics = repositories::regional_compliance::get_accounting_metrics(...)?;` then pass into the `SlovakAccountingExportInput { total_revenue: metrics.total_revenue, total_expenses: metrics.total_expenses, ... }` block by field-shorthand-equivalent.
4. Since the field names match 1:1 with `SlovakAccountingExportInput`'s monetary fields, consider optionally impl-ing `From<AccountingMetrics> for SlovakAccountingExportInput` (with the non-metric fields filled by the handler after conversion) — collapses steps 2+3's boilerplate. Keep or skip based on how many other export types share the shape.
5. Verify: `cargo fmt --all -- --check`, `cargo clippy -p db -p api-server --all-targets -- -D warnings`, `cargo check --workspace`.
6. No SQL change, no migration, no wire-format change — the JSON response body is byte-identical.

## Alternatives considered
- **Newtype-wrap each `Decimal` (`TotalRevenue(Decimal)`, `TotalReceivables(Decimal)` …)** — rejected because it forces every downstream Decimal operation to unwrap or use `.0`, spraying the domain-typing wrapper across arithmetic. The named-struct approach gets equivalent transposition safety at the repository→handler seam with lower call-site overhead.
- **Leave the tuple, add a `#[deny(clippy::…)]` lint or a `debug_assert_eq!` cross-check on field ranges** — rejected because there's no lint that catches type-identical positional confusion; a runtime `debug_assert!` fires only in debug builds and only if the swap produces impossible values, which for money values often is not the case.

## Root-cause trace
1. Symptom: silently mislabeled `total_revenue` / `total_receivables` (or the two `Option`s) in the wire response when either side of the tuple/destructure is edited.
2. ← `routes/regional_compliance.rs:431-437` destructures the tuple positionally; a swap here or in step 3 compiles fine.
3. ← `repositories/regional_compliance.rs:499` returns the tuple with the same shape. The `#[allow(clippy::too_many_arguments)]`-style pattern (implicit here — no explicit annotation, but the same code smell) means the compiler has nothing to check.
4. Origin: introduced when `SlovakAccountingExport` first landed as a positional constructor + positional metrics tuple. PR #2069 / #2099 / #2117 progressively fixed the export ctor; the metrics tuple was the last positional link and #2122 named it explicitly.

## Test plan
- [ ] `backend/crates/db/tests/accounting_metrics_return_shape_tests.rs` — new `#[sqlx::test]`:
  - `accounting_metrics_returns_named_struct_with_distinct_values` — seed org / building / invoices / payments with distinct `Decimal` values (e.g. `1000` revenue, `500` receivables, `2000` expenses, `750` payables); call `get_accounting_metrics`; assert `metrics.total_revenue == 1000`, `metrics.total_receivables == 500`, `metrics.total_expenses == Some(2000)`, `metrics.total_payables == Some(750)`. Field-name assertion is the pin — the test fails at the type level if the return shape reverts to tuple.
- [ ] Existing `SlovakAccountingExport` tests remain green (no wire change).
- [ ] Command: `cargo test -p db accounting_metrics && cargo test -p api-server regional_compliance` (CI provides Postgres via `backend.yml` sqlx-test lane).
- [ ] Static: `cargo clippy -p db -p api-server --all-targets -- -D warnings` — must be zero.

## Out of scope
- Fake-minutes / fake-vote-fallback fixes (`bug-regional-compliance-fake-*`). Different code paths; separate plans.
- Wider audit of other positional multi-argument constructors in the same crate (`SlovakVoteMinutes`, `CzechUsneseni` — those are being tackled by the fake-minutes plan; other returns can wait).
- Any wire-format / OpenAPI change — this refactor is invisible on the API surface.

## After-merge
- Move this file to `plans/_archive/refactor-accounting-metrics-named-struct.md`
- Mark the matching `backlog.json` row (`code-review-api-handlers-metric-tuple-swap`) as `status: "done"`
