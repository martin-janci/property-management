---
name: ppt-dev-stack
description: Bring up the local PPT dev stack declaratively via `stack up pm-local`, with a Docker Compose fallback.
when_to_use: A plan needs the full stack (postgres + redis + minio + api-server + reality-server + ppt-web + reality-web) running locally for repro or integration tests.
mode: local
capabilities: [C3]
tags: [infra]
---

# PPT Dev Stack

Local dev orchestration. Primary path is the `stack` CLI from
`~/dotfiles/claude/.claude/skills/dev-stack`, which reads the declarative
manifest at `~/dotfiles/dev-stacks/pm-local.yml`. Fallback is raw
`docker compose -f docker-compose.dev.yml`.

## When to invoke

Plan ticks C3 (run a dev instance) or requires reproducing a flow
end-to-end on `localhost`. Skip if you're in cloud mode — use
[`ppt-bridge-mcp`](../ppt-bridge-mcp/SKILL.md) instead.

## What it gives you

- One-command up / down / logs via `stack`
- Per-service URLs (ports)
- When to drop to raw compose
- Cloud equivalent pointer

## Services and ports

From `~/dotfiles/dev-stacks/pm-local.yml`:

| Service | Port | Notes |
|---|---|---|
| `postgres` | 5432 | Primary DB |
| `redis` | 6379 | Cache / queue |
| `minio` | 9000 (API), 9001 (console) | S3-compatible store (documents + images) |
| `api-server` | 8080 | Rust — Property Management API |
| `reality-server` | 8081 | Rust — Reality Portal API |
| `ppt-web` | 3000 | Vite + React — PPT web (SPA) |
| `reality-web` | 3001 | Next.js — Reality web (SSR) |

Dependencies: `api-server`/`reality-server` need `postgres + redis + minio`;
the web apps need their respective API server.

## Steps

1. **Bring up the whole stack:**
   ```bash
   stack up pm-local
   ```
2. **Subset:**
   ```bash
   stack up pm-local postgres redis minio   # infra only
   stack up pm-local api-server             # implicitly brings up deps
   ```
3. **Inspect:**
   ```bash
   stack list                           # all known stacks
   stack ps pm-local                    # service status
   stack logs pm-local api-server       # tail logs
   ```
4. **Tear down:**
   ```bash
   stack down pm-local                  # leaves volumes
   stack down pm-local --volumes        # wipes DB / minio data
   ```
5. **Fallback to raw compose** when `stack` is unavailable or you need a
   compose-only flag:
   ```bash
   docker compose -f docker-compose.dev.yml up -d postgres redis minio
   docker compose -f docker-compose.dev.yml logs -f api-server
   docker compose -f docker-compose.dev.yml down
   ```
6. **Frontend foreground dev** (skip Docker for hot reload):
   ```bash
   just web            # ppt-web on :3000 via pnpm
   just reality-web    # reality-web on :3001 via pnpm
   ```
   Requires `api-server` reachable on `:8080` (Docker or `just api`).

## Cloud equivalent

When running in a cloud routine (no local Docker), use the bridge —
[`ppt-bridge-mcp`](../ppt-bridge-mcp/SKILL.md):

```
ppt_dev_up host=mefistos
ppt_dev_logs service=api-server tail=200 host=mefistos
ppt_dev_down host=mefistos
```

## Deterministic verification

```bash
# 1. stack CLI on PATH
command -v stack >/dev/null && echo OK
# expected: OK

# 2. pm-local manifest registered
stack list 2>/dev/null | grep -qE '(^|\s)pm-local(\s|$)' && echo OK
# expected: OK

# 3. docker daemon reachable (needed for the compose fallback)
docker info >/dev/null 2>&1 && echo OK
# expected: OK (Docker Desktop or colima running)

# 4. compose file present in the repo
test -f docker-compose.dev.yml && echo OK
# expected: OK
```

## Smoke check (single command)

```bash
stack list 2>/dev/null | grep -qE '(^|\s)pm-local(\s|$)'
```

## After-task verification

After bringing the stack up for a flow test:

```bash
curl -fsS http://localhost:8080/health >/dev/null && echo api-ok
curl -fsS http://localhost:3000/ >/dev/null && echo web-ok
```

## Cross-references

- `~/dotfiles/claude/.claude/skills/dev-stack/skill.md` — the `stack` CLI itself
- `~/dotfiles/dev-stacks/pm-local.yml` — manifest
- [`ppt-bridge-mcp`](../ppt-bridge-mcp/SKILL.md) — cloud equivalent
- [`ppt-db-migrations`](../ppt-db-migrations/SKILL.md) — apply migrations
  after `postgres` is up
