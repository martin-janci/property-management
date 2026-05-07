---
name: ppt-deploy
description: Manage worktree deployments to *.dev.ppt.rlt.sk and *.dev.rlt.sk via the ppt-deploy server. Use when the user wants to spin up, list, status, or close a worktree dev environment, or to deploy/promote staging or prod. Triggers on phrases like "open worktree", "deploy this branch", "promote v1.2.3", "close worktree".
---

# ppt-deploy skill

Wraps the `pmctl` CLI to deploy worktree branches to subdomain dev URLs and to manage staging/prod releases.

## Where the deploy server lives

- **URL**: `https://onyx.rlt.sk` (Hetzner, behind Caddy, custom build with cloudflare DNS-01).
- **Health**: `curl https://onyx.rlt.sk/health` — public, no auth.
- **Auth**: bearer token (sha256 hash on the server side, full token on the client). Token is stored in macOS Keychain on the operator's laptop; the server never sees it persisted in cleartext.

## Setup (one-time, per developer machine)

```bash
# Save the API token in macOS Keychain (paste the token when prompted)
security add-generic-password -s ppt-deploy-token -a "$USER" -w

# In your shell rc (~/.zshrc, ~/.bashrc, etc.):
export PPT_DEPLOY_URL=https://onyx.rlt.sk
export PPT_DEPLOY_TOKEN=$(security find-generic-password -s ppt-deploy-token -w 2>/dev/null)
```

If `pmctl` isn't on `$PATH`, build it:

```bash
cd backend && cargo build --release -p deploy-server --bin pmctl
sudo install target/release/pmctl /usr/local/bin/pmctl
```

`pmctl --url` defaults to `https://onyx.rlt.sk`, so once `PPT_DEPLOY_TOKEN` is exported the rest is `pmctl <subcommand>`.

## Quick reference

- `pmctl open <branch>` → spawns frontend dev containers, registers Caddy routes, prints URLs.
- `pmctl close <name>` → graceful shutdown, marks for TTL cleanup.
- `pmctl status [name]` / `pmctl list` → state introspection.
- `pmctl version` / `pmctl --json` → JSON output for parsing.
- `pmctl logs <name> [--service api|reality|ppt|reality-web]` → SSE log stream.
- `pmctl deploy <tag>` / `pmctl promote <tag>` / `pmctl rollback` → release lifecycle.

### Without `pmctl` (raw curl)

The exposed paths are `/api/...` (NOT `/v1/...`). Examples:

```bash
# List worktrees
curl -H "Authorization: Bearer $PPT_DEPLOY_TOKEN" "$PPT_DEPLOY_URL/api/worktrees"

# Open a worktree (shared backend)
curl -X POST -H "Authorization: Bearer $PPT_DEPLOY_TOKEN" -H "Content-Type: application/json" \
  "$PPT_DEPLOY_URL/api/worktree" \
  -d '{"branch":"feature/UC-14","backend":"shared"}'

# Health (no auth)
curl "$PPT_DEPLOY_URL/health"
```

Full endpoint list: `references/api.md`.

## When to use which command

- User: "deploy my branch" → `commands/open-worktree.md`
- User: "shut down this worktree" → `commands/close-worktree.md`
- User: "what's running?" → just call `pmctl list --json` and summarize.

## Frontend mode switching

When opening a worktree, the skill writes the right `.env.local` per app so the frontend talks to the new backend by default. See `references/modes.md` and `commands/open-worktree.md` for the exact env-var names (they differ between ppt-web and reality-web).

## API surface

For low-level calls (instead of CLI), see `references/api.md`.
