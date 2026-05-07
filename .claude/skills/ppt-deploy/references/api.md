# HTTP API reference (deploy server)

Base URL: `https://onyx.rlt.sk` (production). All paths start with `/api/...`
or `/health`. There is no `/v1/...` prefix.

| Endpoint | Method | Body | Notes |
|---|---|---|---|
| `/health` | GET | — | Public, returns `{status, version}` |
| `/api/worktree` | POST | `{branch, alias?, backend, ttl_seconds?}` | Open. `backend` ∈ `shared`/`dedicated`. |
| `/api/worktrees` | GET | — | List |
| `/api/worktree/{name}` | GET | — | Status |
| `/api/worktree/{name}/close` | POST | — | Close (graceful) |
| `/api/logs/{name}` | GET | — | SSE stream; query `?service=ppt\|reality\|api\|reality-api\|all` (default `all`). `reality-web` is NOT valid — the frontend container suffix is `reality`. |
| `/api/audit` | GET | — | Recent audit rows; query `?limit=N` clamped to `1..=500` (default 100). |
| `/api/deploy` | POST | `{tag, target?}` | Staging deploy (Phase 2). Default target=`staging`. |
| `/api/wake/{target}` | POST | — | Resume a paused/stopped target. |
| `/api/release` | POST | `{tag, images, notes?}` | Register prod-candidate (called by release.yml on tag push). `notes` is optional and gets stored verbatim on the candidate row. |
| `/api/promote` | POST | `{tag, target, dry_run?}` | Promote candidate to live (Phase 4). `target` is required; `dry_run` is bool, default false. |
| `/api/rollback` | POST | `{target, to?}` | Rollback. `target` required; optional `to` (note: field is `to`, NOT `tag`) — when present, rolls forward/back to that specific tag instead of the previous one. |
| `/api/gc/tick` | POST | — | Bearer auth (systemd timer; not for humans). |
| `/api/webhook/github` | POST | GH webhook payload | HMAC-only (no bearer); validates `X-Hub-Signature-256`. |

## Auth

`Authorization: Bearer <token>` for everything except `/health` and the
GitHub webhook (which uses HMAC).

Token sources (in order of precedence used by `pmctl`):
1. `--token <token>` flag
2. `$PPT_DEPLOY_TOKEN` env var
3. `~/.config/ppt-deploy/token` file

Recommended on macOS: store the token in Keychain and export it once per
shell session via:

```bash
export PPT_DEPLOY_TOKEN=$(security find-generic-password -s ppt-deploy-token -w)
```

## Scopes

The token's grant set is checked per endpoint. The default operator token
has scope `*` (all). CI tokens are typically scoped tighter:
- `release:deploy` — `POST /api/deploy`
- `release:register` — `POST /api/release` (prod candidates from tag-push)
- `release:wake` — `POST /api/wake/{target}`
- `release:promote` — `POST /api/promote`
- `release:rollback` — `POST /api/rollback`
- `worktree:open` — `POST /api/worktree`
- `worktree:close` — `POST /api/worktree/{name}/close`
- `worktree:read` — `GET /api/worktrees`, `GET /api/worktree/{name}`,
  `GET /api/logs/{name}`
- `gc:tick` — `POST /api/gc/tick` (used only by the systemd timer)
- `audit:read` — `GET /api/audit`
