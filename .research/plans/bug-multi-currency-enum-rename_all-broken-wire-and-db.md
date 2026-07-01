# bug-multi-currency-enum-rename_all-broken-wire-and-db

**Vector:** bug
**Score:** 3
**Source:** Issue #2000 (PR #1991 follow-up)
**Confidence:** high

## Hypothesis
`CountryCode` and `SupportedCurrency` in `backend/crates/db/src/models/multi_currency.rs` are annotated with `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` and `#[sqlx(type_name = "...", rename_all = "SCREAMING_SNAKE_CASE")]`. Because every variant is already an ISO acronym (`SK`, `CZ`, `EUR`, `CZK`), serde's `SCREAMING_SNAKE_CASE` treats each capital letter as a word boundary and emits `"S_K"`, `"C_Z"`, `"E_U_R"`, `"C_Z_K"`. The public wire contract now demands non-ISO codes, and the sqlx encode/decode label `'S_K'` does not exist in the Postgres enum (`00101_create_multi_currency.sql` seeds real ISO codes `'SK'`, `'EUR'`, `'CZK'`). PR #1991 masked this by bending finance test payloads to the broken form instead of fixing the attribute. Switching both `rename_all` to `"UPPERCASE"` restores the ISO round-trip and lets the test payloads revert to the intended `"SK"`/`"EUR"` form.

## Evidence
- `backend/crates/db/src/models/multi_currency.rs` ~L19 `SupportedCurrency` + ~L144 `CountryCode` — both carry `rename_all = "SCREAMING_SNAKE_CASE"` on serde and sqlx.
- `backend/crates/db/migrations/00101_create_multi_currency.sql` — `CREATE TYPE country_code AS ENUM ('SK','CZ',...)` and `supported_currency AS ENUM ('EUR','CZK',...)` — labels are the correct ISO codes; sqlx encoding `variant SK → label 'S_K'` cannot find the row.
- `backend/servers/api-server/tests/multi_currency_happy_path_tests.rs` (PR #1991) — payloads use `country: "S_K"`, `tenant_country: "C_Z"`, `property_currency: "E_U_R"` and the compliance path `/compliance/S_K`.
- Issue #2000 (2026-07-01) — post-merge review of PR #1991 with the same root-cause diagnosis.
- `docs/api/typespec/domains/accounting.tsp` — currency documented as plain `string, "e.g. CZK"`; TypeSpec and generated clients currently advertise ISO codes while the running server rejects them.

## Files
- `backend/crates/db/src/models/multi_currency.rs`
- `backend/servers/api-server/tests/multi_currency_happy_path_tests.rs`
- `backend/crates/db/migrations/00101_create_multi_currency.sql`
- `docs/api/typespec/domains/accounting.tsp`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode:**

Mode: cloud-ok

## Repro steps
1. Start the api-server and hit any endpoint that accepts `CountryCode` or `SupportedCurrency` on the wire, e.g. `POST /api/v1/regions` with body `{"country": "SK", ...}`.
2. Expected: request succeeds and DB row stores `country = 'SK'`.
3. Actual: request rejects with 422 (serde deserialize fails); the only accepted form is the broken `"S_K"`. If the code path binds the column as the enum type (rather than text), sqlx encode of `CountryCode::SK` also fails at the DB boundary because label `'S_K'` is not in the `country_code` enum.

## Suggested approach
1. In `backend/crates/db/src/models/multi_currency.rs`, change both attributes on `CountryCode` — `#[serde(rename_all = "UPPERCASE")]` and `#[sqlx(type_name = "country_code", rename_all = "UPPERCASE")]`.
2. Same change on `SupportedCurrency` — `#[serde(rename_all = "UPPERCASE")]` and `#[sqlx(type_name = "supported_currency", rename_all = "UPPERCASE")]`.
3. Grep the rest of `backend/crates/db/src/models/` for other all-caps-acronym enums using `SCREAMING_SNAKE_CASE` and repeat if any match.
4. Revert the test payloads in `backend/servers/api-server/tests/multi_currency_happy_path_tests.rs` back to ISO codes: `"SK"`, `"CZ"`, `"EUR"`, `"CZK"`, `.../compliance/SK`.
5. Optional: add a `serde_json::to_value(SupportedCurrency::EUR)` unit assertion in a `#[cfg(test)]` block to lock the wire form as `"EUR"` and prevent regression.
6. Optional stretch: model `CountryCode`/`SupportedCurrency` explicitly in `docs/api/typespec/domains/accounting.tsp` so generated Rust + TS clients enforce the ISO values instead of accepting `string`.

## Alternatives considered
- **Leave rename_all alone, keep the ugly `"S_K"` on the wire** — rejected because it silently breaks the ISO 3166-1 / 4217 contract with frontend + mobile clients and forces every consumer to hand-craft non-standard codes; the DB enum labels still say `'SK'`/`'EUR'`, so any code path that binds the enum type (as opposed to text) breaks against the real schema regardless.
- **Manual `impl Serialize` / `#[serde(rename = "SK")]` per variant** — rejected because it duplicates the mapping and drifts easily; `rename_all = "UPPERCASE"` collapses it into a single attribute per enum.

## Root-cause trace
1. Symptom: `POST /api/v1/regions` with body `{"country":"SK"}` returns 422 (serde reject); the multi-currency happy-path tests only pass with `country: "S_K"`.
2. ← Deserialization of `CountryCode` from `"SK"` fails because the derived `Deserialize` impl only accepts `"S_K"` at `backend/crates/db/src/models/multi_currency.rs:~144`.
3. ← `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` expands to `heck::AsShoutySnakeCase("SK")` which treats the two capitals as separate words → `"S_K"`.
4. Origin: whichever commit first added the multi-currency model with `SCREAMING_SNAKE_CASE` chosen instead of `UPPERCASE` (mechanical mis-copy from a mixed-case enum where the policy DID work); latent since introduction, only surfaced when PR #1991 wrote the first tests that actually exercise the wire form.

## Test plan
- [ ] Un-quarantine or run `backend/servers/api-server/tests/multi_currency_happy_path_tests.rs` after payload revert — with the fix, ISO-code payloads must succeed. With ISO-code payloads on the current `SCREAMING_SNAKE_CASE` code, tests must fail (IG3 evidence).
- [ ] New unit tests near the enum defs in `models/multi_currency.rs`: `assert_eq!(serde_json::to_value(CountryCode::SK).unwrap(), json!("SK"))` and the same for `SupportedCurrency::EUR`.
- [ ] `cargo test -p db --lib` for the model-level unit tests + `cargo test -p api-server --test multi_currency_happy_path_tests` for the integration surface.

## Out of scope
- Fixing the BIT-351 quarantine harness so all `#[ignore]`d finance tests run — tracked separately under `test-gap-bit-351-quarantine-hides-regression-guards`.
- Reworking `docs/api/typespec/domains/accounting.tsp` to model the enums explicitly — noted as an optional stretch; TypeSpec parity is a follow-on issue.

## After-merge
- Move this file to `plans/_archive/bug-multi-currency-enum-rename_all-broken-wire-and-db.md`
- Mark the matching `backlog.json` row as `status: "done"`
