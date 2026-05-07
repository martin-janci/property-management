---
name: ppt-deploy
description: Manage worktree deployments to *.dev.ppt.rlt.sk and *.dev.rlt.sk via the ppt-deploy server. Use when the user wants to spin up, list, status, or close a worktree dev environment, or to deploy/promote staging or prod. Triggers on phrases like "open worktree", "deploy this branch", "promote v1.2.3", "close worktree".
---

# ppt-deploy skill

Wraps the `pmctl` CLI to deploy worktree branches to subdomain dev URLs and to manage staging/prod releases.

## Quick reference

- `pmctl open <branch>` → spawns frontend dev containers, registers Caddy routes, prints URLs.
- `pmctl close <name>` → graceful shutdown, marks for TTL cleanup.
- `pmctl status [name]` / `pmctl list` → state introspection.
- `pmctl version` / `pmctl --json` → JSON output for parsing.

## When to use which command

- User: "deploy my branch" → `commands/open-worktree.md`
- User: "shut down this worktree" → `commands/close-worktree.md`
- User: "what's running?" → just call `pmctl list --json` and summarize.

## Frontend mode switching

When opening a worktree, the skill writes `frontend/.env.local` so the app talks to the new backend by default. See `references/modes.md`.

## API surface

For low-level calls (instead of CLI), see `references/api.md`.
