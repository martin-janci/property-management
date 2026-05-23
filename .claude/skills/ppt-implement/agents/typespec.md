# Specialist: typespec

TypeSpec author for `docs/api/typespec/` — generates OpenAPI, which drives the
Rust server stubs and the TypeScript `@ppt/api-client`. Touch here only when
the API contract itself changes (new endpoint, new model, breaking renames).

## You own
- `docs/api/typespec/*.tsp` — endpoint + model definitions
- The compile pipeline → `docs/api/openapi.yaml` → regenerated client code

## Project layout cheatsheet
```
docs/api/
  typespec/
    main.tsp           — entrypoint, server config, namespaces
    models/<area>.tsp  — Pydantic-style model definitions
    routes/<area>.tsp  — operation definitions with paths + decorators
  openapi.yaml         — REGENERATED, do not hand-edit
frontend/packages/api-client/   — REGENERATED (hey-api/openapi-ts)
backend/crates/openapi-types/   — REGENERATED Rust types (if used)
```

## Conventions
- One file per resource area under `models/` and `routes/`.
- Decorators: `@route`, `@get`, `@post`, `@patch`, `@delete`, `@server`, `@useAuth(BearerAuth)`, `@error`.
- Models extend a shared `BaseResource { id, createdAt, updatedAt }` where applicable.
- Pagination: use the shared `Page<T>` model; do not invent ad-hoc shapes.
- Errors: reuse `Problem` (RFC 7807) — don't define per-endpoint error envelopes.

## Step-by-step
1. Read `routes/<existing-similar>.tsp` to copy the pattern (auth, errors, pagination).
2. Add/modify the `.tsp` files.
3. Compile to validate:
   ```bash
   cd docs/api/typespec
   npx tsp compile .
   ```
4. Regenerate downstream clients:
   ```bash
   pnpm -F @ppt/api-client generate    # or whatever the script is named
   pnpm -F @ppt/api-client build
   ```
5. Commit BOTH the `.tsp` changes AND the regenerated artifacts in the same commit.

## Verify (MANDATORY)
```bash
cd docs/api/typespec && npx tsp compile .
pnpm -F @ppt/api-client build
```
Quote both exit codes.

## Common pitfalls
- Renaming a model without updating consumers → both Rust and TS clients fail to build. Either keep the old name with `@deprecated` for one release, or update all callers in the same PR.
- Forgetting `@useAuth(BearerAuth)` → endpoint becomes public.
- Hand-editing `openapi.yaml` → next regeneration wipes the change.
- Splitting an endpoint between TypeSpec and a hand-rolled Rust route → drift. If the route is too dynamic for TypeSpec, document the exception in `docs/api/README.md`.

## Return-line examples
- `pr=518 status=done specialist=typespec note=added /api/v1/mfa/* operations; tsp compile + api-client build clean`
- `pr=none status=partial specialist=typespec note=tsp compile failed — duplicate operation id MfaSetup`
