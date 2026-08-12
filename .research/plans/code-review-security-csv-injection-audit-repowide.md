# code-review-security-csv-injection-audit-repowide

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review-pm-qa (2026-08-12); prior PR #2731
**Confidence:** medium

## Hypothesis
PR #2731 sanitized user-authored text in CSV export for **vote titles only**, using an opt-in `sanitize_csv_cell` helper called at that one export site. The same formula-injection vector (leading `=`, `+`, `-`, `@`, tab, CR) applies to every other backend CSV export that writes user-authored strings — notably GDPR data export (`routes/gdpr.rs`), reports (`routes/reports/mod.rs`), and market-pricing (`routes/market_pricing.rs`) — because sanitization is per-call-site rather than centralised at the CSV writer. A malicious tenant can inject a spreadsheet formula (e.g. `=HYPERLINK(evil,x)` in an address, `=cmd|'/C calc'!A1` in a message field) that fires when a landlord or admin opens the export in Excel / LibreOffice / Google Sheets. Fix: audit every `text/csv` producer, route free-text cells through `sanitize_csv_cell`, and add a regression test per fixed field.

## Evidence
- PR #2731 (2026-08-11) sanitized only vote titles in reports CSV export; commit landed as `code-review-api-handlers-reports-csv-injection`.
- `backend/servers/api-server/src/routes/gdpr.rs` — GDPR data-portability export builds CSV rows including user-authored fields (names, messages, addresses); needs audit.
- `backend/servers/api-server/src/routes/reports/mod.rs` — sibling to the fixed vote-title site; other report categories reuse the same builder pattern.
- `backend/servers/api-server/src/routes/market_pricing.rs` — listing titles and free-text notes flow into CSV output.
- `sanitize_csv_cell` (added by #2731) is opt-in per call site, not enforced at the CSV writer — the pattern will regress on the next new export handler.

## Files
- `backend/servers/api-server/src/routes/gdpr.rs`
- `backend/servers/api-server/src/routes/reports/mod.rs`
- `backend/servers/api-server/src/routes/market_pricing.rs`
- `backend/servers/api-server/src/routes/reports`
- `backend/servers/api-server/src`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (grep-based audit across every CSV producer)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode:**

Mode: cloud-ok

## Repro steps
1. In an integration test, POST a vote whose title starts with `=HYPERLINK("http://evil/",X)`. (Blocked by #2731.)
2. Repeat for a GDPR export: create a user whose full name is `=cmd|'/C calc'!A1`, request `/api/v1/gdpr/export`, save the returned CSV, open in LibreOffice.
3. Expected after fix: the leading `=` is prefixed with a `'` (or otherwise neutralised) in every CSV export path — never fires as a formula.
4. Actual today for GDPR / reports / market_pricing: cell renders as an executable formula on open.

## Suggested approach
1. Grep for every CSV writer: `grep -rn "text/csv\|Content-Type.*csv\|write_record\|WriterBuilder\|csv::Writer" backend/ --include="*.rs"` — build the call-site inventory.
2. For each producer, list every column whose source is user-authored (any `String` that entered the DB via a POST/PATCH body).
3. Route each such column through `sanitize_csv_cell(...)` at the call site. If the helper isn't accessible from the crate, promote it to a shared util (`backend/servers/api-server/src/util/csv.rs` or the `util::errors`-style shared module).
4. **Preferred structural fix:** wrap the CSV `Writer` in a thin `SanitizingCsvWriter` that always calls `sanitize_csv_cell` on `write_field(String)` unless the caller opts out for a numeric/typed column. This makes future exports safe-by-default.
5. Add a regression test per fixed field: seed a row whose value starts with each of `=`, `+`, `-`, `@`, `\t`, `\r`; call the export handler; assert the CSV bytes have the value prefixed / escaped.
6. Update `sanitize_csv_cell`'s doc comment to name the CVE class ("CWE-1236 Formula Injection") and link the audit PR.
7. Run `cargo test -p api-server` and `cargo clippy --workspace --all-targets -- -D warnings`; confirm the whole workspace still builds.

## Alternatives considered
- **Client-side escaping (Excel-side)** — rejected because we cannot control which spreadsheet application the user opens the export in; the mitigation has to be server-side.
- **Content-Type change (`text/tab-separated-values`)** — rejected because it doesn't remove the injection vector (Excel still auto-executes leading `=` in TSV), and existing consumers depend on `text/csv`.

## Root-cause trace
1. Symptom: user-authored cell whose text starts with `=` executes as a formula on export open (arbitrary hyperlink, command-line invocation, cell exfiltration).
2. ← Immediate cause: CSV writer emits the raw string verbatim (no `'`-prefix or wrap) at each per-handler call site.
3. ← Upstream cause: `sanitize_csv_cell` (added by PR #2731) is opt-in and applied at a single site; the CSV writer has no default-on sanitisation layer.
4. Origin: the CSV export code long predates #2731. The class of bug is well-known (CWE-1236). PR #2731 fixed one manifestation only.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/csv_injection_tests.rs` — new integration suite that seeds a row per export handler with a formula-prefixed value, calls the export, asserts the returned CSV bytes are neutralised.
- [ ] Regression case per fixed field (GDPR full name, GDPR message, GDPR address, report title, report note, market_pricing listing title).
- [ ] Failing-on-main assertion: today's `gdpr::export` returns `=HYPERLINK(...)` unmodified — the new test fails against `origin/dev` before the fix.
- [ ] `cargo test -p api-server --test csv_injection_tests` — exact local command.
- [ ] `cargo test -p api-server` — full suite must stay green.

## Out of scope
- Frontend CSV import parsers (this plan targets server-side export only).
- Non-CSV export formats (`text/tab-separated-values`, XLSX) — separate audit.
- Changing the export column set or CSV dialect.

## After-merge
- Move this file to `plans/_archive/code-review-security-csv-injection-audit-repowide.md`
- Mark the matching `backlog.json` row as `status: "done"`
