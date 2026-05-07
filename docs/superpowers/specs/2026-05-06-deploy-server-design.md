# Deploy Server Design

**Date:** 2026-05-06
**Status:** Draft — pending user review
**Author:** Brainstormed with Claude

## 1. Overview

A small, on-demand deployment control plane (`ppt-deploy`) that manages container lifecycles for three target environments — per-worktree dev, staging, and prod — across the Property Management stack (`api-server`, `reality-server`, `ppt-web`, `reality-web`).

**Primary goals:**
- Sub-second cold-start, ~0 RAM idle (systemd socket activation).
- One-command worktree spawn → live subdomain (`<alias>.dev.ppt.rlt.sk`).
- Default shared backend per worktree, opt-in dedicated backend with isolated Postgres database.
- Auto-deploy staging on `main` merge, blue-green promote to prod via tag.
- Frontend developer can switch between local backend, worktree-remote backend, and MSW mock at runtime.
- AI-agent friendly: deterministic CLI, `--dry-run` everywhere, JSON output, audit log.

**Non-goals (MVP):**
- Multi-tenancy of the deploy server itself.
- Web dashboard (deferred to Phase 6).
- Auto-rollback (configurable but defaults to manual).
- Cross-host orchestration (Phase 5 opt-in).

## 2. Locked-in decisions

| # | Decision | Rationale |
|---|---|---|
| 1 | **Topology**: deploy server runs on-demand, single Hetzner VPS for MVP, opt-in for prod-elsewhere later | Cost-optimized for greenfield (no users yet) |
| 2 | **Wake mechanism**: systemd socket activation + cron watchdog (5 min) | ~0 RAM idle, lazy active GC |
| 3 | **Reverse proxy**: Caddy + DNS-01 wildcard ACME, custom `xcaddy` build | One wildcard cert per zone, hot reload via admin API |
| 4 | **DNS provider**: configurable (Cloudflare / Hetzner DNS / acme-dns delegate), default Cloudflare | Provider-agnostic via YAML config |
| 5 | **Image flow**: bind-mount dev for worktree frontend, GH-built images for staging/prod | Fast iteration on worktree, prod-like for staging+prod |
| 6 | **Source push**: `git push origin <branch>` → server `git fetch` (no rsync) | GH is single source of truth, audit via PR history |
| 7 | **Rust build risks**: matrix consolidation, `cargo chef` dep batching, feature unification, background warmup, `profile.release` strip+lto, `mold` linker | Tackled as Phase 0 cross-cutting |
| 8 | **DB strategy**: schema search_path for default frontend-only, DB-per-worktree for opt-in dedicated backend (`CREATE DATABASE … TEMPLATE ppt_dev_template`) | RLS-friendly, clean isolation when needed |
| 9 | **TTL**: configurable per worktree, default 2 days | Cleanup snapshots after this window |
| 10 | **Auth**: public HTTPS endpoint + GH OIDC (CI) + global API key (Claude skill) + audit log | Standard, no VPN, CI-native |
| 11 | **Frontend mode**: build-time default (`.env.local`) + dev panel runtime override | Claude skill writes default; manual override at runtime |
| 12 | **Mock layer**: MSW (Mock Service Worker) | Industry standard, zero coupling to API client |
| 13 | **Worktree pairing**: Vite plugin reads git worktree info, injects `__WORKTREE_NAME__`; `.env.local` overrides take priority | Magic-with-escape-hatch |
| 14 | **Worktree lifecycle**: layered (Claude skill explicit + GH PR closed webhook + inactivity timeout + TTL) | Three layers cannot lose state |
| 15 | **Worktree identity**: explicit alias (default = sanitized branch name) | Human-readable subdomains |
| 16 | **Staging**: longer idle timeout (8h), pause overnight, resume on-demand | Stable test environment, no idle cost overnight |
| 17 | **Resume from TTL**: synchronous (`pmctl open` blocks with progress) | Deterministic for agent loops |
| 18 | **Prod release**: tag → GHA build → register candidate; `pmctl promote` flips traffic | Blue-green free, rollback trivial |
| 19 | **Deploy strategy**: blue-green (2× memory for ~30 s during swap) | 0-downtime, cheap |
| 20 | **Rollback**: configurable, default manual | Auto-rollback risks false positives until monitoring matures |
| 21 | **Future targets**: `targets.yaml` declarative — opt-in remote Docker socket per target | Migration without code rewrite |
| 22 | **`--dry-run`** on every mutating command | Mandatory for AI agent loops |
| 23 | **Tech**: Rust (axum + bollard + sqlx + clap) in `backend/` Cargo workspace | Stack consistency, smallest idle footprint, recycled patterns |

## 3. Architecture

```
                                    ┌──────────────────────────────────────────────┐
                                    │  Hetzner VPS (rlt.sk)  — single host (MVP)   │
                                    │                                              │
  GH Actions ──── OIDC ────────►   │  ┌──────────────────────────────┐            │
  (push to main, tags)             │  │  Caddy  :443                  │            │
                                    │  │  *.dev.rlt.sk                 │            │
  Claude skill (laptop) ─ API key ►│  │  *.dev.ppt.rlt.sk             │            │
                                    │  │  *.staging.rlt.sk             │            │
  GH webhook (PR closed) ──────────►│  │  onyx.rlt.sk                │            │
                                    │  │  rlt.sk + ppt.rlt.sk (prod)   │            │
                                    │  └────┬───────┬──────────────────┘            │
                                    │       │       │                               │
                                    │       │       └────► systemd socket           │
                                    │       │              ┌────────────────────┐   │
                                    │       │              │ ppt-deploy (Rust)  │   │
                                    │       │   spawns ───►│ on-demand binary   │   │
                                    │       │              │ - Docker SDK       │   │
                                    │       │              │ - Caddy admin API  │   │
                                    │       │              │ - sqlite state     │   │
                                    │       │              │ - git ops          │   │
                                    │       │              │ - pg_dump/restore  │   │
                                    │       │              └─────────┬──────────┘   │
                                    │       ▼                        │              │
                                    │  ┌────────────────────────────▼─────────┐    │
                                    │  │ Docker (rootful, deploy user in      │    │
                                    │  │  docker group; rootless in Phase 6)  │    │
                                    │  │  ┌─────────┐ ┌─────────┐ ┌────────┐  │    │
                                    │  │  │ prod    │ │ staging │ │ wt-*   │  │    │
                                    │  │  │ stack   │ │ stack   │ │ dyn.   │  │    │
                                    │  │  └─────────┘ └─────────┘ └────────┘  │    │
                                    │  │  ┌─────────────────┐                  │    │
                                    │  │  │ postgres (1×)   │ DB-per-worktree │    │
                                    │  │  └─────────────────┘                  │    │
                                    │  └──────────────────────────────────────┘    │
                                    │                                              │
                                    │  /var/lib/ppt-deploy/                       │
                                    │    state.db          (sqlite)               │
                                    │    audit.log                                │
                                    │    snapshots/        (pg_dumps, TTL'd)      │
                                    │    worktrees/        (cloned source code)   │
                                    │                                              │
                                    │  cron: ppt-deploy gc  every 5 min           │
                                    └──────────────────────────────────────────────┘
```

### Components

| Component | Form | Idle footprint |
|---|---|---|
| `ppt-deploy` (server) | Rust binary, systemd-socket-activated | 0 RAM idle, ~5 MB on first request |
| `pmctl` (CLI) | Rust binary, second `bin` target | ad-hoc (laptop / SSH) |
| Caddy | systemd service, always running | ~10–15 MB |
| Postgres | docker container, always running | ~50 MB |
| `ppt-deploy-gc.timer` | systemd timer, fires every 5 min | ~0 (short run) |
| Frontend dev containers | docker, lifecycle managed | spun up/down |
| Backend containers | docker, shared default + opt-in dedicated | spun up/down |

## 4. Lifecycle scenarios

### A. Open new worktree (frontend-only, shared backend)

```
laptop                          deploy-server (Hetzner)            github
──────                          ────────────────────────            ──────
$ pmctl open feature-uc14
  ─POST /api/worktree──────────►
                                check sqlite: new worktree
                                git fetch origin feature/UC-14
                                  → /var/lib/ppt-deploy/worktrees/feature-uc14/
                                docker volume create wt-uc14-pnpm
                                docker run -d ppt-frontend-dev \
                                  -v /var/.../feature-uc14/frontend:/app \
                                  -v wt-uc14-pnpm:/app/.pnpm-store
                                  → container "wt-uc14-ppt"
                                docker run -d ppt-frontend-dev (reality)
                                  → container "wt-uc14-reality"
                                Caddy admin API:
                                  add upstream wt-uc14.dev.ppt.rlt.sk → :PORT_A
                                  add upstream wt-uc14.dev.rlt.sk    → :PORT_B
                                sqlite UPSERT worktree row
                                audit.log: open feature-uc14
  ◄──── 200 OK { urls } ────────
$ # browser: https://wt-uc14.dev.ppt.rlt.sk

$ git push origin feature/UC-14 ─────────────────────────────────────►
                                ◄──── webhook push ────────────────────
                                git fetch (idempotent), HMR sees changes
```

### B. Worktree opt-in dedicated backend

```
$ pmctl open feature-uc14 --backend=dedicated --as=uc14
  ─POST /api/worktree { dedicated: true }─►
                                git fetch
                                createdb ppt_wt_uc14 TEMPLATE ppt_dev_template
                                trigger GHA workflow_dispatch:
                                  branch=feature/UC-14, target=api-server
                                  (background)
                                docker run frontend dev
                                Caddy: register frontend (live)
                                       register api.wt-uc14... → :HOLD (503)
  ◄──── 200 OK { urls, backend_status: "building" } ────
$ pmctl status feature-uc14
  ─GET /api/worktree/feature-uc14────►
                                check GHA build status
  ◄──── { backend_status: "building", eta: "2m" } ────
# (after 3 min)
                                GHA pushes ghcr.io/.../api-server:feature-uc14
                                webhook GH `package` → docker pull + run
                                Caddy upstream switch 503→live
$ pmctl status feature-uc14
  ◄──── { backend_status: "ready" } ────
```

### C. Reopen worktree within TTL window

```
# (worktree was paused 6 h ago, dump exists)
$ pmctl open feature-uc14
  ─POST /api/worktree──►
                                sqlite: state="paused", dump_path=...
                                pg_restore from dump → 12 s (sync, progress)
                                docker start (existing containers)
                                Caddy upstream re-enable
  ◄──── 200 OK { urls } [12 s] ────
```

### D. PR closed → cleanup

```
GH PR closed (merged or not)
  ─webhook─►                    sqlite UPDATE state="closing"
                                docker stop wt-uc14-* (graceful)
                                pg_dump ppt_wt_uc14 > snapshots/uc14-<ts>.dump
                                dropdb ppt_wt_uc14
                                Caddy: remove upstreams
                                schedule cleanup at now() + ttl
                                audit.log: closed feature-uc14
                                # cron GC at TTL: rm -rf worktree dir + dump
```

### E. Promote v1.2.3 to prod

```
$ git tag v1.2.3 && git push --tags
                                GHA builds images: ghcr.io/.../*:v1.2.3
                                GHA POST /api/release { tag: v1.2.3 }
                                deploy-server: docker pull
                                  start as candidate (port +1000)
                                  sqlite: prod_candidate=v1.2.3

$ pmctl promote v1.2.3 --target=prod --dry-run
  ◄── current=v1.2.2, candidate=v1.2.3, would-switch upstreams [...]

$ pmctl promote v1.2.3 --target=prod
                                Caddy admin API: atomic upstream switch
                                health check (60 s grace) → if OK: stop old
                                                            if FAIL: warn (manual rollback)
                                sqlite: prod=v1.2.3, prod_prev=v1.2.2

$ pmctl rollback --target=prod
                                Caddy upstream flip → v1.2.2
                                audit.log: rollback to v1.2.2
```

### F. Background GC tick (every 5 min)

```
ppt-deploy-gc.timer fires
  pmctl gc tick
    └─► systemd starts ppt-deploy
        for each worktree in sqlite:
          if last_traffic > 30m and state=running: pause container
          if last_traffic > 24h and state=paused: pg_dump + rm container
          if state=closed and now > closed_at + ttl: rm worktree dir, rm dump
        for each release in sqlite:
          if state=candidate and age > 7d: rm container, mark stale
```

## 5. HTTP API

All endpoints `Bearer`-authenticated (GH OIDC JWT or API key). Audit log entry per call.

| Endpoint | Caller | Purpose |
|---|---|---|
| `POST /api/worktree` | pmctl | Open worktree (`{branch, alias?, backend, ttl?}`) |
| `GET /api/worktree/{name}` | pmctl | Status |
| `POST /api/worktree/{name}/close` | pmctl, GH webhook | Graceful close |
| `DELETE /api/worktree/{name}` | pmctl `--hard` | Skip TTL, full cleanup |
| `GET /api/worktrees` | pmctl `list` | All worktrees |
| `POST /api/release` | GHA on tag | Register `prod-candidate` |
| `POST /api/deploy` | GHA on main | Deploy staging |
| `POST /api/promote` | pmctl | Promote tag → target |
| `POST /api/rollback` | pmctl | Rollback target |
| `GET /api/targets` | pmctl | List targets from `targets.yaml` |
| `POST /api/gc/tick` | systemd timer | Idle/TTL housekeeping |
| `POST /api/webhook/github` | GH | `pull_request closed`, `package`, `push` |
| `GET /api/audit?since=…&worktree=…` | pmctl, dashboard | Audit log query |
| `GET /api/logs/{name}?follow=true` | pmctl `logs -f` | SSE stream from Docker logs |
| `GET /health` | Caddy, monitoring | Liveness |

## 6. `pmctl` CLI

Every mutating command supports `--dry-run` and `--json`.

```
# worktree management
pmctl open <branch> [--backend=shared|dedicated] [--as=<alias>] [--ttl=2d]
pmctl close <name> [--hard]
pmctl status [<name>]
pmctl list [--state=running|paused|closed]
pmctl logs <name> [-f] [--service=ppt|reality|api|all]
pmctl psql <name>                         # psql via SSH tunnel to worktree DB
pmctl shell <name> [--service=...]        # docker exec -it

# release / deploy
pmctl deploy staging [--tag=latest]
pmctl promote <tag> --target=prod [--dry-run]
pmctl rollback --target=prod [--to=<tag>]
pmctl wake <target>                       # resume paused staging/worktree on demand
pmctl targets

# admin
pmctl audit [--since=2d] [--worktree=<name>]
pmctl token rotate
pmctl gc tick
pmctl version
```

## 7. Claude skill

```
.claude/skills/ppt-deploy/
├── SKILL.md              # description, when-to-use
├── commands/
│   ├── open-worktree.md  # checks branch, calls pmctl, writes .env.local
│   ├── close-worktree.md # detects current worktree, pmctl close
│   ├── deploy-staging.md # rare manual override; CI is auto
│   └── promote-prod.md   # interactive: dry-run first, confirm, promote
└── references/
    ├── api.md            # HTTP API reference
    └── modes.md          # frontend mode switching cheat sheet
```

`open-worktree.md` flow:
1. `git rev-parse --abbrev-ref HEAD` → branch.
2. `pmctl open --json $branch` → URL.
3. Write `frontend/.env.local`:
   ```
   VITE_API_DEFAULT=worktree
   VITE_API_BASE=https://wt-<alias>.dev.ppt.rlt.sk
   VITE_REALITY_API_BASE=https://wt-<alias>.dev.rlt.sk
   ```
4. Report: "Worktree `feature-uc14` ready: https://… (frontend bind-mount, shared backend). DB: `public` schema."

## 8. State storage

`/var/lib/ppt-deploy/state.db` (sqlite, sqlx migrations):

```sql
CREATE TABLE worktree (
  name              TEXT PRIMARY KEY,           -- alias or sanitized branch
  branch            TEXT NOT NULL,
  backend_mode      TEXT NOT NULL,              -- 'shared' | 'dedicated'
  state             TEXT NOT NULL,              -- 'running' | 'paused' | 'closed' | 'closing'
  urls              JSON NOT NULL,              -- { ppt: '...', reality: '...', api: '...' }
  containers        JSON NOT NULL,              -- ['wt-uc14-ppt', ...]
  db_name           TEXT,                       -- 'ppt_wt_uc14' or NULL (shared)
  dump_path         TEXT,                       -- /var/.../snapshots/uc14-<ts>.dump
  ttl_seconds       INTEGER NOT NULL DEFAULT 172800,
  last_traffic_at   INTEGER,
  closed_at         INTEGER,
  created_at        INTEGER NOT NULL,
  created_by        TEXT NOT NULL
);

CREATE TABLE release (
  tag               TEXT PRIMARY KEY,
  images            JSON NOT NULL,
  state             TEXT NOT NULL,              -- 'candidate' | 'staging' | 'prod' | 'previous'
  target            TEXT,
  promoted_at       INTEGER,
  notes             TEXT
);

CREATE TABLE audit (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  ts                INTEGER NOT NULL,
  caller_kind       TEXT NOT NULL,              -- 'oidc' | 'api_key' | 'webhook'
  caller_id         TEXT NOT NULL,
  endpoint          TEXT NOT NULL,
  params            JSON,
  result            TEXT,
  duration_ms       INTEGER
);

CREATE INDEX idx_audit_ts ON audit(ts);
CREATE INDEX idx_worktree_state ON worktree(state);
```

`last_traffic_at` is updated by a small Caddy access-log tail (or by Caddy hook calling `POST /api/gc/tick` with worktree name in metadata). Decided in Phase 1 implementation.

## 9. Configuration

### Filesystem layout

```
/etc/ppt-deploy/
  config.yaml          # bind addr, ttl defaults, paths
  targets.yaml         # target definitions
  auth.yaml            # OIDC issuer, JWKS url, allowed repos, webhook secret
  dns.yaml             # DNS provider config (used to render Caddy block)
/var/lib/ppt-deploy/
  state.db
  audit.log            # text log (rotated)
  snapshots/           # pg_dumps
  worktrees/           # cloned source trees
  logs/                # systemd journal redirect
/run/ppt-deploy/
  server.sock          # if we ever switch to UNIX socket
```

### `targets.yaml`

```yaml
targets:
  staging:
    docker_socket: unix:///var/run/docker.sock
    caddy_url: http://localhost:2019
    domain_suffix: staging.rlt.sk
    idle_timeout: 8h
  prod:
    docker_socket: unix:///var/run/docker.sock
    caddy_url: http://localhost:2019
    domain_suffix: rlt.sk
    promote_strategy: blue-green
    rollback_mode: manual                          # 'manual' | 'auto'
    health_grace: 60s
```

### `dns.yaml`

```yaml
dns:
  provider: cloudflare                              # cloudflare | hetzner | acme-dns
  cloudflare:
    api_token: ${CF_DNS_TOKEN}
  hetzner:
    api_token: ${HETZNER_DNS_TOKEN}
  acme_dns:
    server: https://acme.rlt.sk
    username: ${ACME_DNS_USER}
    password: ${ACME_DNS_PASS}
    subdomain: ${ACME_DNS_SUBDOMAIN}
```

Caddy is custom-built with all three plugins (`xcaddy build --with caddy-dns/cloudflare --with caddy-dns/hetzner --with caddy-dns/acme-dns`). Active provider chosen at runtime via this file; deploy server renders the matching Caddy admin API config block.

## 10. Phasing

### Phase 0 — Prerequisites (parallelizable, no dependency on deploy server)

- DNS wildcard records for `*.dev.rlt.sk`, `*.dev.ppt.rlt.sk`, `*.staging.rlt.sk`, `*.staging.ppt.rlt.sk`.
- Caddy custom build with `cloudflare + hetzner + acme-dns` plugins; wildcard cert provisioning verified.
- Postgres template DB `ppt_dev_template` with DDL + RLS policies + minimal demo data (1 building, 1 manager, 1 owner).
- Frontend dev container base image (`docker/frontend/Dockerfile.dev`) — `node:20-alpine + pnpm + dev entrypoint`.
- CI build optimization (Rust risks): matrix consolidation, `profile.release` strip+lto, `mold` linker. Independent of deploy server, can ship first.

### Phase 1 — MVP: worktree open/close, frontend-only, shared backend

Goal: `pmctl open feature-X` → `https://wt-feature-x.dev.ppt.rlt.sk` in 5 s.

Deliverables:
- `backend/deploy-server/` Rust crate
- `pmctl` second `bin` target
- HTTP API: `POST /api/worktree`, `GET /api/worktree/{name}`, `POST /api/worktree/{name}/close`, `GET /api/worktrees`, `GET /health`
- systemd socket activation unit
- Caddy admin API client
- sqlite state (worktree + audit tables)
- GH OIDC + API key auth
- Frontend bind-mount run via Docker SDK (bollard)
- `git fetch` shell-out
- Vite plugin `vite-plugin-ppt-worktree`
- Frontend dev panel (`frontend/shared/dev-panel/`) — visible only in dev builds; persists mode in `localStorage`. Includes:
  - Mode dropdown (`local` / `worktree` / `mock`)
  - "Re-seed mock" button (re-runs MSW handlers with fresh seeds)
  - "Snapshot current state to localStorage" button (saves current API responses for repeatable testing)
- MSW setup for `frontend/ppt/mocks/` and `frontend/reality/mocks/`
- Claude skill `commands/open-worktree.md`, `close-worktree.md`
- GH webhook receiver for `pull_request closed`
- Cron GC: idle pause (30 min), idle stop (24 h)
- Smoke tests

### Phase 2 — Staging auto-deploy on merge to main

- GHA `docker-build.yml` POSTs `/api/deploy` after image push
- Blue-green swap (or simple swap MVP) for staging
- `pmctl deploy staging` manual override
- Idle timeout 8 h
- Resume staging on inbound traffic OR `pmctl wake staging`

### Phase 3 — Dedicated backend per worktree

- DB-per-worktree (`createdb FROM TEMPLATE`)
- Server triggers GHA `workflow_dispatch`, polls completion
- Backend container per worktree
- `pg_dump`/`pg_restore` for resume from TTL
- Sync resume in `pmctl open` (progress)
- `pmctl logs <name> [-f]` via SSE
- `pmctl psql <name>` via SSH tunnel
- Background warmup from Claude skill (`open` triggers GHA build, status poll)

### Phase 4 — Prod release flow

- Tag → GHA build → `POST /api/release` registers candidate
- `pmctl promote <tag> --target=prod [--dry-run]`
- Blue-green swap on prod
- `pmctl rollback --target=prod [--to=<tag>]`
- Health grace 60 s + warning output
- Auto-rollback flag in `targets.yaml` (default off)

### Phase 5 — Prod-elsewhere readiness (opt-in)

- `targets.yaml`: `prod` target with `docker_socket: ssh://prod@new-server.rlt.sk`
- Tailscale ACL for cross-host (or SSH key on deploy server)
- Caddy on remote host with admin API on Tailnet
- Test via `staging-elsewhere` target before prod switch

### Phase 6 — Polish

- Read-only web dashboard (audit log, worktree status, deploy history) on `onyx.rlt.sk/dashboard`
- Auto-rollback on staging (precursor to prod auto-rollback)
- Dependabot + cargo-deny for dep hygiene
- Per-worktree token scoping (if security needs change)
- Migrate to rootless Docker

## 11. Cross-cutting

### Docker
- **Rootful Docker for MVP**, `ppt-deploy` user in `docker` group. Migrate to rootless in Phase 6.

### Secrets
- GH webhook secret in `auth.yaml`, rotation via `pmctl token rotate webhook` (writes new secret + reconfigures GH webhook via GH API).
- API key in same file, rotation via `pmctl token rotate api`.
- **Git fetch credentials**: GitHub deploy key (SSH, read-only, single repo) installed on the Hetzner box at `/var/lib/ppt-deploy/.ssh/id_ed25519`. Used by `git fetch origin <branch>` for worktree source. Alternative: fine-grained PAT scoped to `contents:read` for one repo, stored in `auth.yaml`. Deploy key is preferred (no expiry, scoped per-repo).
- **GH API token (server → GH)** for `workflow_dispatch` triggers (Phase 3) and `package` event reads: fine-grained PAT with `actions:write`, `packages:read`, scoped to the single repo. Stored in `auth.yaml`.

### Audit
- Every API call writes one `audit` row (caller_kind, caller_id, endpoint, params, result, duration).
- Audit log retention: 90 days (configurable in `config.yaml`).

### Concurrency
- `pmctl open` for two different worktrees can run in parallel.
- Per-branch lock for `git fetch` (not global).
- Caddy admin API calls are serialized by Caddy itself (no app-side lock needed).

### Disk budget
- Hetzner box has 160 GB free.
- TTL-trimmed snapshots: typical steady state 2–5 worktrees, ~1 GB.
- Worktree source trees: ~500 MB each.
- Acceptable headroom; alert at 80 % via cron.

### Testing
- Integration tests use docker-in-docker on local + GHA.
- Smoke test against staging Hetzner on PR review.

## 12. Open questions / future work

| # | Question | Resolution |
|---|---|---|
| 1 | Stale `prod-candidate` cleanup TTL | 7 days, auto-cleanup via GC |
| 2 | E2E test of deploy server itself | docker-in-docker locally + manual smoke on staging |
| 3 | Web dashboard scope (read-only Phase 6) | Static React app, Caddy-served, calls `/api/audit` and `/api/worktrees` |
| 4 | Auto-rollback heuristics (Phase 4 opt-in) | Health check `GET /health` 5× over 60 s grace; if 3+ fail → flip back. Refine after first prod incident. |
| 5 | Per-worktree token scoping | Defer until multi-user need arises |
| 6 | Backup strategy for `state.db` itself | Daily `sqlite3 .backup` to S3-compatible storage; Phase 4 |

## 13. Approval

This spec is brainstormed and approved in principle by the user across sections 1–4 of the brainstorming flow. Awaiting written-spec review before invoking the `writing-plans` skill to produce the implementation plan.
