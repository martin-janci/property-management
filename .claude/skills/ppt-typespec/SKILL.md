---
name: ppt-typespec
description: Author TypeSpec at docs/api/typespec/ to drive the OpenAPI spec and the generated Rust + TS clients.
when_to_use: The plan adds, removes, or changes an API endpoint — anything that needs to ripple through to the Rust server and frontend client.
mode: both
capabilities: [C6]
tags: [backend, frontend, infra]
---

# PPT TypeSpec

`docs/api/typespec/` is the contract source-of-truth. `just generate-api`
emits the OpenAPI spec and regenerates the two TS API clients. Backend
handlers consume the spec via utoipa annotations.

## When to invoke

The plan changes an endpoint shape, adds a route, or modifies a response /
error contract. **Any contract change starts here**, then ripples to
backend handlers and frontend client.

## What it gives you

- TypeSpec source layout
- Regeneration pipeline (`just generate-api`)
- Breaking-change protocol — when to bump the API version

## Inputs

- A target domain file under `docs/api/typespec/domains/`

## Layout

```
docs/api/typespec/
├── main.tsp                # entry — composes domains
├── tspconfig.yaml          # emitter config
├── shared/                 # shared models / scalars
├── domains/
│   ├── announcements.tsp
│   ├── auth.tsp
│   ├── buildings.tsp
│   ├── compliance.tsp
│   ├── documents.tsp
│   ├── faults.tsp
│   ├── listings.tsp
│   ├── organizations.tsp
│   ├── rentals.tsp
│   ├── units.tsp
│   └── voting.tsp
├── tsp-output/             # generated (do commit)
├── package.json            # @typespec/* deps
└── package-lock.json
```

## Steps

1. **Edit the domain file** under `docs/api/typespec/domains/<area>.tsp`.
2. **Compile + regenerate clients:**
   ```bash
   just generate-api
   ```
   This wraps `pnpm generate-api` + `pnpm generate-reality-api` inside
   `frontend/`, which in turn calls the TypeSpec emitter and the OpenAPI
   client generator. Commit both the regenerated TS clients
   (`frontend/packages/api-client/`, `frontend/packages/reality-api-client/`)
   and the regenerated `tsp-output/`.
3. **Update backend handlers** to match the new contract. utoipa
   annotations on Rust handlers must stay consistent — see
   [`ppt-rust-backend`](../ppt-rust-backend/SKILL.md).
4. **Update frontend callers** of the regenerated client.
5. **Validate** via the CI workflow locally before pushing:
   ```bash
   cd docs/api/typespec && npx tsp compile .
   ```

## Breaking-change protocol

Anything that's a non-additive change to an existing operation
(removing/renaming fields, changing types, status-code changes) is
**breaking** and goes through these gates:

1. Bump the API version (and per-product `VERSION`).
2. Document the change in the PR body under `## Breaking changes`.
3. Confirm all client consumers in this repo are migrated in the same PR
   (the generated TS clients update automatically; backend handlers do
   not).
4. CI workflow `api-validation.yml` enforces spec validity — failures
   here block merge.

Additive changes (new optional field, new operation) are safe and don't
need a version bump.

## Deterministic verification

```bash
# 1. typespec deps installed
test -d docs/api/typespec/node_modules && echo OK
# expected: OK after first run; if missing, `cd docs/api/typespec && npm install`

# 2. tsp compiler reachable
cd docs/api/typespec && npx --no-install tsp --version >/dev/null 2>&1 && echo OK
# expected: OK

# 3. main entry present
test -f docs/api/typespec/main.tsp && echo OK
# expected: OK

# 4. expected domain files present
for d in announcements auth buildings compliance documents faults listings organizations rentals units voting; do
  test -f "docs/api/typespec/domains/$d.tsp" || { echo "missing: $d"; exit 1; }
done
echo OK
# expected: OK
```

## Smoke check (single command)

```bash
cd docs/api/typespec && npx --no-install tsp compile . --no-emit 2>/dev/null || cd docs/api/typespec && npx --no-install tsp --version >/dev/null
```

> The compile + no-emit form is preferred but not all tsp versions support
> `--no-emit`; the fallback just verifies the compiler is reachable. Either
> exit 0 means the toolchain is wired.

## After-task verification

```bash
just generate-api && just check-frontend
```

## Cross-references

- [`ppt-rust-backend`](../ppt-rust-backend/SKILL.md) — utoipa annotations,
  handler updates
- [`ppt-frontend`](../ppt-frontend/SKILL.md) — regenerated
  client locations
- `.github/workflows/api-validation.yml` — CI gate
