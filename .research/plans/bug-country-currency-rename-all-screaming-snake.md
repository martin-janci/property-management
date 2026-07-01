# bug-country-currency-rename-all-screaming-snake

**Vector:** bug
**Score:** 3
**Source:** Issue #2000 (post-merge review of PR #1991)
**Confidence:** high

## Hypothesis

`CountryCode` and `SupportedCurrency` in `backend/crates/db/src/models/multi_currency.rs` are annotated with `rename_all = "SCREAMING_SNAKE_CASE"` for both `serde` and `sqlx`, but every variant is an already-uppercase ISO code (`SK`, `CZ`, `EUR`, `USD`, …). serde treats the trailing single-cap boundary as a word split and rewrites `SK`→`"S_K"`, `EUR`→`"E_U_R"`. This breaks two contracts from one attribute: (1) the wire API rejects the standard ISO codes with 422 and requires clients to send `"S_K"`/`"E_U_R"`; (2) sqlx encode/decode tries to bind variant `SK` as label `'S_K'`, which does not exist in the Postgres enum (`CREATE TYPE country_code AS ENUM ('SK','CZ',...)`), so any real DB read/write path with these columns typed as the enum is broken. Change both attributes to `"UPPERCASE"` to match the DB labels and the Rust `Display`/`FromStr` impls, which already assume ISO codes.

## Evidence

- `backend/crates/db/src/models/multi_currency.rs:19-20` — `SupportedCurrency` with `SCREAMING_SNAKE_CASE`
- `backend/crates/db/src/models/multi_currency.rs:144-145` — `CountryCode` with `SCREAMING_SNAKE_CASE`
- `backend/crates/db/migrations/00101_create_multi_currency.sql` — `CREATE TYPE country_code AS ENUM ('SK','CZ','AT',…)` / `supported_currency AS ENUM ('EUR','CZK','CHF',…)` — plain ISO labels, no underscores
- PR #1991 test payloads had to be bent to `country: "S_K"`, `tenant_country: "C_Z"`, `property_currency: "E_U_R"`, path `/compliance/S_K` to compile against the current server — issue #2000 documents this
- `impl Display for SupportedCurrency` and `impl FromStr` in the same file already emit/accept the ISO forms (`"EUR"`, `"CZK"`, …), so Rust internals are inconsistent with the (broken) wire contract

## Files

- `backend/crates/db/src/models/multi_currency.rs:19`
- `backend/crates/db/src/models/multi_currency.rs:20`
- `backend/crates/db/src/models/multi_currency.rs:144`
- `backend/crates/db/src/models/multi_currency.rs:145`
- `backend/servers/api-server/tests/multi_currency_happy_path_tests.rs`

## Dependencies

## Required capabilities

- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. On `dev`: `curl -X POST http://localhost:8080/api/v1/multi-currency/tenants -H 'content-type: application/json' -d '{"tenant_country":"CZ","property_country":"SK","property_currency":"EUR"}'` — expect 200, observe **422** (rejects ISO codes).
2. Repeat with `{"tenant_country":"C_Z","property_country":"S_K","property_currency":"E_U_R"}` — observe 200/2xx: the server currently requires the mangled forms.
3. On the DB side, insert a row with `country_code = 'SK'` via psql and try to decode it through the Rust model — sqlx will fail to match variant `SK` because `rename_all` expects label `'S_K'` which doesn't exist.

## Suggested approach

1. In `backend/crates/db/src/models/multi_currency.rs:19-20`, change `rename_all = "SCREAMING_SNAKE_CASE"` → `rename_all = "UPPERCASE"` on both `#[sqlx(...)]` and `#[serde(...)]` for `SupportedCurrency`.
2. Do the same at lines 144-145 for `CountryCode`.
3. Grep the models tree for any other uppercase-acronym enum with the same wrong `rename_all`: `rg -n 'rename_all = "SCREAMING_SNAKE_CASE"' backend/crates/db/src/models/` and audit each variant list for all-caps acronyms; fix in one pass.
4. In `backend/servers/api-server/tests/multi_currency_happy_path_tests.rs`, revert PR #1991's bent payloads from `S_K`/`C_Z`/`E_U_R` back to ISO codes `SK`/`CZ`/`EUR`, including the `/compliance/{country}` path segment.
5. Add a small serde round-trip unit test in `multi_currency.rs`'s `#[cfg(test)]` module asserting `serde_json::to_string(&SupportedCurrency::EUR) == "\"EUR\""` and `serde_json::from_str::<SupportedCurrency>("\"EUR\"").unwrap() == EUR` for the three most-used variants (EUR, SK/CZ country pair) — this pins the wire contract and would have caught the original regression.
6. Run `cargo check -p db -p api-server` and `cargo test -p api-server --test multi_currency_happy_path_tests` to confirm the corrected payloads pass with the new `rename_all`.
7. If the OpenAPI spec surfaces these enums, regenerate the typespec output (`docs/api/typespec/`) so the generated Rust/TS clients advertise the ISO forms.

## Alternatives considered

- **Leave `SCREAMING_SNAKE_CASE` and update every client + DB migration to `S_K`/`E_U_R`** — rejected because ISO codes are the industry standard for country/currency, we'd break every existing client, and the DB migration to rename enum labels is destructive.
- **Add explicit `#[serde(rename = "SK")]` on every variant** — rejected: repetitive, error-prone (easy to miss a variant), and `rename_all = "UPPERCASE"` expresses the same intent for one attribute per enum.

## Root-cause trace

1. Symptom: PR #1991 tests only compile when payloads use `"S_K"`/`"E_U_R"`; author noted "CountryCode wire form is S_K/C_Z" in the PR body.
2. ← Serde's `SCREAMING_SNAKE_CASE` policy inserts `_` at any case boundary; a 2-letter all-caps variant `SK` has a boundary between the two capitals, so the emitted label is `"S_K"` (`backend/crates/db/src/models/multi_currency.rs:19-20`, `144-145`).
3. ← The `#[sqlx(rename_all = ...)]` attribute mirrors the same wrong policy, so any code path that actually binds/reads these columns as the enum type (as opposed to `String`) tries to encode label `'S_K'`, which is absent from the DB enum.
4. Origin: PR that introduced `SCREAMING_SNAKE_CASE` on these two enums (Epic 145 initial commit of `multi_currency.rs`). Latent: the multi-currency handlers previously ran with `#[ignore]` quarantines, so the encoding mismatch never blew up loudly in CI until PR #1991 un-quarantined the suites and had to bend the payloads to keep them green.

## Test plan

- [ ] Serde round-trip unit test in `backend/crates/db/src/models/multi_currency.rs` `#[cfg(test)]`: `SupportedCurrency::EUR` <=> `"EUR"`, `CountryCode::SK` <=> `"SK"`, `CountryCode::CZ` <=> `"CZ"` — fails on the current `SCREAMING_SNAKE_CASE` code, passes after `UPPERCASE` fix (deterministic IG3).
- [ ] `backend/servers/api-server/tests/multi_currency_happy_path_tests.rs` reverted to ISO payloads still passes end-to-end.
- [ ] Command: `cargo test -p db --lib multi_currency::` and `cargo test -p api-server --test multi_currency_happy_path_tests`.

## Out of scope

- Modelling these enums explicitly in TypeSpec (issue #2000 finding 2 — separate typespec follow-up).
- Fixing the ~59 `finance.md` rows still marked `done` on `#[ignore]`'d tests (issue #2001 — separate follow-up).
- Migrating away from runtime `sqlx::query_as` towards compile-checked `query_as!` for the finance/multi-currency queries.

## After-merge

- Move this file to `plans/_archive/bug-country-currency-rename-all-screaming-snake.md`
- Mark the matching `backlog.json` row as `status: "done"`
