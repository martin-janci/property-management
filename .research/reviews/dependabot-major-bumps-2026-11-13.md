# Dependabot major-version bump review — 2026-11-13

Review of the three major-version Dependabot PRs open since 2026-11-12. These are
**major** bumps and need a human merge decision — this note is the breaking-change
surface + per-PR recommendation so a human (or a follow-up implementer task) can act
quickly. Minor-group PRs can auto-merge after; these three cannot.

| PR | Bump | Kind | Recommendation |
|----|------|------|----------------|
| [#3444](https://github.com/martin-janci/property-management/pull/3444) | `jsonwebtoken` 10.4.0 → 11.1.0 | backend, **auth-critical** | **Safe to land** pending human read + green backend CI |
| [#3447](https://github.com/martin-janci/property-management/pull/3447) | `vitest` 4.1.11 → 5.1.0 | frontend, dev-only | **Hold** — needs companion changes (see below) |
| [#3443](https://github.com/martin-janci/property-management/pull/3443) | `utoipa-swagger-ui` 9.0.2 → 10.0.2 | backend | **Hold** — will not compile as-is; needs coordinated `utoipa` 5→6 migration |

---

## #3444 — jsonwebtoken 10.4.0 → 11.1.0  (SAFE TO LAND, pending human read + CI)

**Auth-critical.** v11's breaking changes (from the changelog):

- `Validation.insecure_disable_signature_validation` removed → use `dangerous::insecure_decode`
- `EncodingKey.inner` renamed → `as_bytes`; `try_get_hmac_secret` removed
- `DecodingKey.as_bytes` / `try_get_hmac_secret` removed → use `try_get_as_bytes`
- `Algorithm`, `KeyAlgorithm`, `EllipticCurve`, `ThumbprintHash` are now `#[non_exhaustive]`
- `Jwk.thumbprint` now returns `Result<_>`
- `Header.extras` is now a struct
- Implicit features from optional crates removed

**Repo usage (full surface):** the workspace only touches the stable API —
`DecodingKey::from_secret` / `EncodingKey::from_secret`, `Validation::default()`,
`Header::default()`, and `decode` / `encode`. Call sites:

- `backend/crates/api-core/src/extractors/auth.rs` — `decode`, `DecodingKey::from_secret`, `Validation::default()`
- `backend/crates/api-core/src/extractors/principal.rs` — `DecodingKey::from_secret`, `Validation::default()`
- `backend/crates/api-core/tests/{principal_token_type,portal_principal_token_type,optional_principal}_tests.rs` — `encode`, `EncodingKey::from_secret`, `Header::default()`

None of the removed/renamed/`non_exhaustive` symbols are used. No `Jwk`, no
`insecure_*`, no `.inner`/`try_get_hmac_secret`, no `Algorithm` exhaustive match.

**Feature flags:** the PR keeps `features = ["rust_crypto"]` (the pure-Rust crypto
backend). The "implicit features removed" breaking change is already covered because
the workspace sets the crypto backend explicitly (see the existing comment in
`backend/Cargo.toml`). Cargo.lock keeps `ed25519-dalek` + `base64 0.22.1`.

**Recommendation:** API-safe. Let backend CI (`cargo test` — exercises the
encode→decode round-trip and the `exp`/`token_type` gate) confirm green, then a human
merges. Do **not** auto-merge: it is auth code and policy requires a human read.

---

## #3447 — vitest 4.1.11 → 5.1.0  (HOLD — companion changes required)

Dev-only test runner, but **do not merge as Dependabot opened it.** Two concrete
problems visible in the PR's `pnpm-lock.yaml`:

1. **`@vitest/coverage-v8` left at 4.1.11.** The PR bumps `vitest` to 5.1.0 in every
   `package.json` but leaves `@vitest/coverage-v8` at `^4.1.11` (ppt-web, reality-web).
   The lockfile resolves it as `@vitest/coverage-v8@4.1.11(vitest@5.1.0)`, and
   `vitest@5.1.0` declares a peer of `@vitest/coverage-v8: 5.1.0`. Coverage providers
   are version-locked to the runner's internals — a v4 provider under a v5 runner is a
   peer mismatch and very likely fails at runtime. Both ppt-web and reality-web run
   `coverage: { provider: 'v8' }`, so `pnpm test` with coverage is at high risk.
   → A merge must also bump `@vitest/coverage-v8` to `^5.1.0`.

2. **Node engine bump.** `vitest@5.1.0` sets `engines: { node: "^22.12.0 || ^24.0.0 || >=26.0.0" }`,
   dropping Node 20. The repo baseline is **Node ≥ 20** (CLAUDE.md frontend toolchain).
   If CI runners or local dev are on Node 20, vitest 5 won't run.
   → Needs a Node ≥ 22.12 baseline decision before (or with) this bump.

Config surface is otherwise clean: the vitest configs use only stable options
(`globals`, `environment: 'jsdom'`, `setupFiles`, `include`, `coverage`), and there is
no `vitest.workspace.*` file using the deprecated workspace API.

**Recommendation:** Hold. Reopen as a small follow-up that (a) bumps
`@vitest/coverage-v8` to `^5.1.0` alongside vitest, and (b) raises the Node engine
baseline to ≥ 22.12 (and updates CI node-version). Then let frontend CI gate it.

---

## #3443 — utoipa-swagger-ui 9.0.2 → 10.0.2  (HOLD — does not compile as-is)

**This PR will fail the backend build.** `utoipa-swagger-ui@10` depends on
`utoipa@6`, but the workspace pins `utoipa = { version = "5", ... }` directly (for the
`#[derive(OpenApi)]` / `#[derive(ToSchema)]` macros). The PR's Cargo.lock confirms
**two** utoipa majors coexisting: `utoipa 5.5.0` (direct) **and** `utoipa 6.0.0`
(pulled by swagger-ui 10, with `utoipa-gen 6.0.1` on `syn 3.0`).

All three servers wire the UI as:

```rust
SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi())
```
- `backend/servers/api-server/src/main.rs:200`
- `backend/servers/reality-server/src/main.rs:521`
- `backend/servers/accounting-server/src/main.rs:330`

`ApiDoc::openapi()` returns a `utoipa::openapi::OpenApi` from the **v5** derive, but
`SwaggerUi::url()` in swagger-ui 10 expects the **v6** `OpenApi` type. These are
distinct types from distinct crate versions → a type-mismatch compile error. The
Dependabot compatibility badge does not catch this.

The PR does **not** touch `utoipa = "5"` in `backend/Cargo.toml` (it only bumps
`utoipa-swagger-ui`), so the mismatch is unresolved.

MSRV is not the blocker: swagger-ui 10 requires Rust 1.88 and the toolchain is 1.97.1
(`backend/rust-toolchain.toml`).

**Recommendation:** Hold / close this Dependabot PR. Adopting swagger-ui 10 requires a
coordinated **`utoipa` 5 → 6 major migration** across the workspace — every
`ToSchema`/`OpenApi` derive, the generated OpenAPI spec, and the generated Rust + TS
clients (which the `api-validation.yml` CI gate checks). That is a dedicated task, not
a dependency bump. Keep swagger-ui at 9 until a `utoipa 6` migration is planned.

---

## Suggested follow-up tasks

1. **`jsonwebtoken` 11** — human-review + merge #3444 once backend CI is green (no code change needed).
2. **`vitest` 5** — new PR: bump `vitest` + `@vitest/coverage-v8` together to `^5.1.0`, raise Node baseline to ≥ 22.12, update CI node-version. Supersedes #3443→#3447 (close #3447).
3. **`utoipa` 6 migration** — dedicated backend task: bump workspace `utoipa` 5→6 and `utoipa-swagger-ui` 9→10 together, regenerate OpenAPI + clients, run `api-validation.yml`. Supersedes #3443.
</content>
</invoke>
