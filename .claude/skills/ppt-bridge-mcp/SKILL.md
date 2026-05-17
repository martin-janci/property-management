---
name: ppt-bridge-mcp
description: Use the ppt-bridge MCP to run dev-stack, DB queries, and tests against a remote host from a cloud routine.
when_to_use: The implementer is running in cloud mode (no local Docker) and the plan needs dev-stack / seed / DB / tests against mefistos or hetzner.
mode: cloud-ok
capabilities: [C2, C3]
tags: [infra, workflow]
---

# PPT Bridge MCP

The bridge at `https://p.rlt.sk/mcp` exposes 10 tools that route SSH-exec
calls to a configured host. In a cloud routine, this is how the implementer
agent touches Docker, psql, and the test runner without a local sandbox.

## When to invoke

You are running in a claude.ai routine or remote session (no local Docker,
no local Chrome) and the plan needs to bring services up, query the DB, or
run integration tests. Skip this skill when you have local Docker (use
`ppt-dev-stack` instead).

## What it gives you

- Catalogue of the 10 bridge tools with one-line "use it when…" entries
- Prod-safety patterns (`is_prod`, `confirm: true`, `preview: true`,
  kill switch)
- Where to configure hosts and inspect audit logs

## Inputs

- Connector `ppt-bridge` attached to the session (`https://p.rlt.sk/mcp`,
  OAuth Google)
- A primary host set (`set_primary_host`) or `host=…` arg per call

## Steps

1. **Discover hosts.** `list_hosts` returns each configured host with its
   `is_prod` flag and capability list. Pick `mefistos` for dev work,
   `hetzner` only when the plan explicitly targets prod-edge ops.
2. **Set the primary** so subsequent calls don't need `host=`:
   ```
   set_primary_host host=mefistos
   ```
3. **Bring the stack up / inspect.** `ppt_dev_up` → `ppt_dev_logs
   service=<svc> tail=200` → `ppt_dev_down` when done. For finer control or
   prod-edge ops, `ppt_docker_compose action=ps|logs|up|down|restart`.
4. **Seed.** `ppt_seed` runs the host's configured `seed_command`. NB: this
   repo has no `just seed` recipe (see `ppt-db-migrations`) — the host must
   have a `seed_command` configured at `https://p.rlt.sk/accounts`, or the
   call errors cleanly.
5. **DB queries.** `ppt_db_query sql="…"` for SELECTs. DML requires
   `db:mutate` capability *and* `confirm: true` per call. Always-on heredoc
   means no shell injection.
6. **Run tests.** `ppt_run_test area=backend|frontend|integration` wraps
   `just test-<area>`. Pass `filter=<pattern>` to drop through to
   `cargo`/`pnpm`/`gradle` directly.
7. **Preview before mutating.** Add `preview: true` to any destructive call
   to see the exact would-execute command without running it. Strongly
   recommended for first runs against a new host.

### Tool quick-reference

| Tool | When to use | Gotcha |
|---|---|---|
| `list_hosts` | Discover what's configured | Cheap, run first every session |
| `set_primary_host` | Avoid repeating `host=` | Session-scoped only |
| `ppt_dev_logs` | Tail a service after change | `tail` defaults to ~50; pass larger for hunt |
| `ppt_dev_up` | Bring stack up | Needs `dev:write`; ~30–60s before services healthy |
| `ppt_dev_down` | Tear stack down | Destructive — prod hosts need `confirm: true` |
| `ppt_seed` | Populate fixtures | Errors if `seed_command` unset on host |
| `ppt_db_query` | Read or (with `confirm`) write | DML requires `db:mutate` capability + `confirm: true` |
| `ppt_run_test` | Run tests in-host | Filter falls through to native runner |
| `ppt_docker_compose` | Finer compose control | `up/down/restart` = `docker:compose:write` |
| `ppt_browser_open` | (v2 skeleton) | Returns `not_implemented` today — use C4 locally |

## Prod safety

Four guards stack on every destructive call against `hetzner` (`is_prod=true`):

1. **Capability split** — caller must have `dev:write` or
   `docker:compose:write`. Missing capability = call refused.
2. **Kill switch** — `BRIDGE_DESTRUCTIVE_DISABLED=1` in the bridge env
   blocks all destructive ops globally. Check first if a call fails
   unexpectedly.
3. **`confirm: true`** — required per-call on prod. No accidental cleanup.
4. **`preview: true`** — opt-in dry-run; returns the resolved command.

Every call is audited at `https://p.rlt.sk/audit`. Prod calls get a
distinct badge. Forensics after an incident start here.

Host configuration lives at `https://p.rlt.sk/accounts`:
- `repo_path` (required)
- `stack_bin` / `stack_name` (defaults: `stack` / `pm-local`)
- `db_command`, `seed_command`
- `docker_compose_file`, `docker_compose_workdir`
- `is_prod`

## Deterministic verification

```bash
# 1. bridge is reachable (anonymous health endpoint)
curl -fsS -o /dev/null -w "%{http_code}\n" https://p.rlt.sk/healthz
# expected: 200

# 2. you have a connector slot for it (check the session connectors list, not via curl)
# manual: in claude.ai → Settings → Connectors → confirm "ppt-bridge" present
```

## Smoke check (single command)

```bash
curl -fsS https://p.rlt.sk/healthz
```

## After-task verification

After a session that used the bridge for destructive ops, spot-check the
audit log:

```bash
# manual: open https://p.rlt.sk/audit, filter by today's session id, confirm
# every is_prod=true call has confirm:true logged.
```

## Cross-references

- [`.research/implementer-prompt.md`](../../../.research/implementer-prompt.md)
  § *ppt-bridge MCP — cloud-side toolset* — full capability matrix
- [`ppt-dev-stack`](../ppt-dev-stack/SKILL.md) — local equivalent
- [`ppt-db-migrations`](../ppt-db-migrations/SKILL.md) — note re: missing
  `just seed`
