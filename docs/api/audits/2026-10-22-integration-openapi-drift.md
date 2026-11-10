# Integration audit — OpenAPI / TypeSpec drift (window 2026-10-17 → 2026-10-22)

- **Author:** m.janci@32bit.sk (pm-integration)
- **Date:** 2026-10-22
- **Task:** `pm-integration-audit-integration-prs-openapi-drift`
- **Scope:** every integration-touching change merged to `dev` since 2026-10-17 —
  did any Rust server handler / route change without a matching TypeSpec +
  regenerated OpenAPI + generated-client update (or vice versa)?

## TL;DR

**No new actionable contract drift was introduced by the window's changes.**
Every integration-touching change in the window either (a) left the HTTP wire
shape byte-for-byte identical, or (b) touched an endpoint that is *already*
outside the TypeSpec contract pipeline entirely. One dep/security hardening pass
and one docs change carried no API surface.

The audit did surface two **pre-existing** (not window-caused) drift findings in
the AML/DSA (`compliance`) domain and one **systemic** gap, all recorded below
for follow-up. None is a trivial doc/spec sync that can be applied inside this
docs-only task (they require frontend client-type or route changes with their
own tests), so they are logged rather than fixed here.

## How the contract pipeline works (what "drift" means here)

There are two independent OpenAPI representations in this repo; only the first
feeds the generated clients:

1. **TypeSpec → OpenAPI → TS clients (the contract of record).**
   `docs/api/typespec/**` is compiled by `@typespec/openapi3`
   (`docs/api/typespec/tspconfig.yaml`, emitter output `../generated`) to
   `docs/api/generated/openapi.yaml`. Both `@ppt/api-client` and
   `@ppt/reality-api-client` run `openapi-ts -i ../../../docs/api/generated/openapi.yaml`
   (see each package's `package.json` `generate` script) to produce
   `src/generated/{types,sdk,client}.gen.ts`. The `api-validation.yml` CI gate
   guards *this* path — but **only triggers when TypeSpec/OpenAPI files change.**

2. **utoipa runtime spec (not consumed by clients).** `api-server`'s
   `#[derive(OpenApi)] ApiDoc` (`backend/servers/api-server/src/main.rs:254`) is
   served at runtime (`/api-docs/openapi.json`, Swagger UI) and is **not dumped
   to a committed file and not consumed by client generation.** `ToSchema`
   derives on handler DTOs feed only this runtime doc.

3. **Hand-maintained domain modules (outside both guards).** A large set of
   endpoints are **not in TypeSpec at all**. Their TS clients are hand-written
   under `frontend/packages/api-client/src/<domain>/{api,hooks,types}.ts`
   (e.g. `compliance/`, `migration/`). Changes to these endpoints' Rust handlers
   are invisible to `api-validation.yml` (no TypeSpec file changes), so drift
   here is caught by nothing but review. **This is where the window's
   integration-touching changes landed.**

> Consequence: for the window's PRs, "regenerated OpenAPI + generated client
> update" was **not required** — the touched endpoints are not in the TypeSpec
> pipeline. The relevant question becomes whether the *hand-written* client
> module still matches the server wire shape.

## Per-change verdict

| PR | Commit | Area | API surface? | In TypeSpec? | Wire shape change? | Verdict |
|----|--------|------|--------------|--------------|--------------------|---------|
| #3129 | `8c099a3` | `routes/aml_dsa/aml.rs` — type `list_aml_assessments` response | **Yes** (response typing) | No (AML not in TypeSpec) | **No** (byte-identical, test-pinned) | **In sync** — no contract change; see Finding 1 |
| #3127 | `7feacdc` | `routes/migration.rs` — persist import upload | **Yes** (new 503/400 statuses) | No (migration not in TypeSpec) | **No** (`ImportJobResponse` unchanged) | **In sync** — success shape unchanged |
| #3092 | `33982b7` | `reality-server` price-map cache bound | No (internal cache only) | n/a | No | **In sync** — `GET /api/v1/price-map` request/response untouched |
| #3123 | `12be5f4` | `api-server` TOTP CSPRNG error path | No (internal service; callers unchanged) | n/a | No | **In sync** — confirms context hint |
| #3135 | `0663a68` | bump `rustls-webpki` (RUSTSEC-2026-0382) | No | n/a | No | **In sync** — dependency only |
| #3125 | `025ce59` | bump `rust_xlsxwriter`/`zip` 9.x | No | n/a | No | **In sync** — dependency only |
| #3130 | `3ee947c` | bump `ammonia` (RUSTSEC-2026-0373) | No | n/a | No | **In sync** — dependency only |
| #2902 | `301fe81` | reality saved-searches screen-map doc | No | n/a | No | **In sync** — docs only |

### #3129 detail (the one that looks like drift but isn't)

`refactor(api-server): type AML assessments list endpoint response`. Replaced the
untyped `Json<serde_json::Value>` payload of `list_aml_assessments` with typed
`ListAmlAssessmentsResponse` / `AmlAssessmentListItem` DTOs (deriving serde +
utoipa `ToSchema`). The commit touched **only** `aml.rs` — no TypeSpec, no
`openapi.yaml`, no generated client. That is correct here because:

- the endpoint `GET /api/v1/aml-dsa/aml/assessments` is **not in TypeSpec**, so
  there is nothing in the `openapi.yaml → src/generated` path to regenerate;
- the DTO mirrors the prior wire shape exactly (same field names, same
  serialized form), and the PR adds serialization tests pinning the field set,
  the boolean flags, the nullable + populated `risk_factors`, and the pagination
  envelope. So no consumer sees a different payload.

The `ToSchema` derives feed only the runtime utoipa doc (representation #2),
which is not committed and not client-generating, so no regeneration artifact is
expected in the diff.

### #3127 detail

`POST /api/v1/migration/import/upload` now threads the uploaded bytes through
`state.storage_service.upload(...)` and uses the returned key as `file_path`
(previously it discarded the bytes and fabricated a key). It adds **503
SERVICE_UNAVAILABLE** (no storage configured) and **400 BAD_REQUEST**
(unsupported content type). These are new *status codes*, not schema changes —
the success body (`ImportJobResponse`) is unchanged, and the hand-written client
(`src/migration/api.ts`) already surfaces non-2xx via `ApiError`. Migration is
not in TypeSpec, so no regeneration was due.

## Findings (pre-existing — recorded for follow-up, not window regressions)

### Finding 1 — AML list client type diverges from the server wire shape

**Severity:** medium · **Status:** latent (frontend reads defensively) · **Owner:** pm-integration / pm-frontend

The hand-written `AmlRiskAssessment` interface in
`frontend/packages/api-client/src/compliance/types.ts` (lines ~102-133) does not
match the now-formalized server wire shape in
`backend/servers/api-server/src/routes/aml_dsa/aml.rs` (`AmlAssessmentListItem`,
post-#3129):

- **Field naming:** frontend `subject_id` / `subject_type` / `subject_name` vs
  server `party_id` / `party_type` (server emits no `subject_name`).
- **Risk-factor flag name:** frontend `AmlRiskFactor.triggered: boolean` vs
  server `RiskFactor.mitigated: bool`.
- **Nullability:** frontend `risk_factors: AmlRiskFactor[]` (required array) vs
  server `Option<Vec<RiskFactor>>` (nullable) — a `null` would break the
  non-optional frontend type. (This is exactly the PR #3117 symptom #3129 cites.)
- **Unmodeled wire fields:** server emits `organization_id`, `transaction_id`,
  `transaction_amount_cents`, `currency`, `country_code`, `country_risk`,
  `is_pep`, `sanctions_match`, `assessor_notes`, `assessed_by`, `assessed_at`,
  `review_reason` — none present on the frontend type.

This is **pre-existing** (the endpoint was an untyped `serde_json::Value` before
#3129; the frontend type was always an approximation). #3129 did not cause it; it
*enables the fix* by giving the endpoint a real, test-pinned schema.
**Follow-up:** either lift `AmlAssessmentListItem` into
`docs/api/typespec/domains/compliance.tsp` so the client is generated, or
reconcile `compliance/types.ts` by hand (rename `triggered`→`mitigated`, make
`risk_factors` optional, align `subject_*`→`party_*`).

### Finding 2 — AML client calls a path the server does not serve

**Severity:** medium · **Status:** needs confirmation · **Owner:** pm-integration

`frontend/packages/api-client/src/compliance/api.ts` sets
`API_BASE = '/api/v1/compliance'` and calls `GET ${API_BASE}/aml/assessments`
(i.e. `/api/v1/compliance/aml/assessments`). The backend mounts the AML router
at `/api/v1/aml-dsa` (`backend/servers/api-server/src/lib.rs:322`), so the
handler lives at `/api/v1/aml-dsa/aml/assessments`. `routes/compliance.rs`
(mounted at `/api/v1/compliance`, lib.rs:243) exposes only audit-log / GDPR /
security-report routes — no `aml/assessments`. The same `API_BASE` is used for
`/aml/thresholds`, `/aml/country-risks`, `/aml/assessments/{id}/edd`, and
`/aml/assessments/{id}/review`.

This is **pre-existing** and unrelated to the window, but is a textbook example
of why hand-maintained clients (outside the TypeSpec + `api-validation.yml`
guard) drift from the server. **Follow-up:** confirm the live path and align
`API_BASE` (or add a backend alias); a generated client would have made this a
compile-time impossibility.

### Finding 3 (systemic) — integration-heavy domains sit outside the contract guard

The AML/DSA (`compliance`) and `migration` domains — both touched this window —
are absent from `docs/api/typespec/**` and `docs/api/generated/openapi.yaml`,
and their clients are hand-maintained. `api-validation.yml` only runs when
TypeSpec/OpenAPI files change, so handler changes to these domains (#3129,
#3127) can never trip the "generated clients up to date" check. **Follow-up:**
prioritize bringing `compliance`/`migration` into TypeSpec (the typed DTOs from
#3129 are a ready starting point), which would convert Findings 1 and 2 into CI
failures instead of latent runtime risks.

## Method / reproduction

```bash
# window changes (first-parent, since 2026-10-17)
git log --since=2026-10-17 --first-parent --pretty=oneline origin/dev
# client generation source
grep -R "openapi-ts -i" frontend/packages/*/package.json
# TypeSpec coverage of the touched domains (empty = not in contract)
grep -rniE 'aml|assessment|migration|price-?map' docs/api/typespec/ docs/api/generated/openapi.yaml
# server wire shape vs hand-written client
sed -n '179,320p' backend/servers/api-server/src/routes/aml_dsa/aml.rs
sed -n '92,133p' frontend/packages/api-client/src/compliance/types.ts
```
