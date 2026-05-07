# HTTP API reference (deploy server)

| Endpoint | Method | Body | Notes |
|---|---|---|---|
| `/api/worktree` | POST | `{branch, alias?, backend, ttl_seconds?}` | Open |
| `/api/worktrees` | GET | — | List |
| `/api/worktree/{name}` | GET | — | Status |
| `/api/worktree/{name}/close` | POST | — | Close |
| `/api/webhook/github` | POST | GH webhook payload | HMAC auth |
| `/api/gc/tick` | POST | — | Bearer auth (cron) |
| `/health` | GET | — | Public |

Auth: `Authorization: Bearer <token>`. Token from `~/.config/ppt-deploy/token` or `$PPT_DEPLOY_TOKEN`.
