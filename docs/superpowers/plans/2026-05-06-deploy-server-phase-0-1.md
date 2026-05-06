# Deploy Server Phase 0 + Phase 1 (MVP) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a working `pmctl open <branch>` command that, in ≤ 5 s, brings up a per-worktree frontend dev URL on `<alias>.dev.ppt.rlt.sk` and `<alias>.dev.rlt.sk`, talking to the shared backend, with full audit logging and graceful close/cleanup.

**Architecture:** Rust crate `backend/servers/deploy-server/` with two binaries (`ppt-deploy` HTTP server, `pmctl` CLI). systemd-socket-activated server delegates work to bollard (Docker), Caddy admin API, sqlite (sqlx), and shell-out git. Frontend integration via Vite plugin + dev panel + MSW. Claude skill wraps `pmctl` in agent-friendly commands.

**Tech Stack:** Rust 1.75 (axum, sqlx-sqlite, bollard, jsonwebtoken, clap, listenfd), Caddy 2.x (custom `xcaddy` build), Postgres 16, Docker (rootful), systemd (socket activation + timer), TypeScript (Vite plugin, React dev panel, MSW).

**Spec source:** [docs/superpowers/specs/2026-05-06-deploy-server-design.md](../specs/2026-05-06-deploy-server-design.md)

---

## File Structure

### New (Phase 0)

```
docker/frontend/Dockerfile.dev.worktree     # bind-mount dev runner for worktrees
docker/caddy/Dockerfile                     # xcaddy custom build w/ DNS plugins
backend/scripts/init-template-db.sql        # ppt_dev_template DDL + RLS + seed
docs/runbooks/deploy-server-prereqs.md      # operator setup checklist (DNS, Caddy, secrets)
```

### New (Phase 1 — server)

```
backend/servers/deploy-server/
├── Cargo.toml
├── migrations/
│   └── 0001_init.sql
├── systemd/
│   ├── ppt-deploy.socket
│   ├── ppt-deploy.service
│   ├── ppt-deploy-gc.service
│   └── ppt-deploy-gc.timer
└── src/
    ├── main.rs                   # bin: ppt-deploy
    ├── lib.rs
    ├── config.rs
    ├── error.rs
    ├── auth/
    │   ├── mod.rs
    │   ├── api_key.rs
    │   └── oidc.rs
    ├── api/
    │   ├── mod.rs
    │   ├── router.rs
    │   ├── worktree.rs
    │   ├── webhook.rs
    │   ├── gc.rs
    │   └── health.rs
    ├── domain/
    │   ├── mod.rs
    │   └── worktree.rs
    ├── infra/
    │   ├── mod.rs
    │   ├── store.rs
    │   ├── docker.rs
    │   ├── caddy.rs
    │   ├── git.rs
    │   └── audit.rs
    └── bin/
        └── pmctl.rs              # bin: pmctl
```

### New (Phase 1 — frontend)

```
frontend/packages/dev-panel/
├── package.json
├── tsconfig.json
└── src/
    ├── index.ts
    ├── DevPanel.tsx
    └── store.ts                  # localStorage-backed mode persistence

frontend/packages/vite-plugin-ppt-worktree/
├── package.json
├── tsconfig.json
└── src/
    └── index.ts                  # vite plugin: injects __WORKTREE_NAME__

frontend/apps/ppt-web/src/mocks/      # MSW
├── browser.ts
├── handlers/
│   ├── index.ts
│   ├── auth.ts
│   ├── buildings.ts
│   └── faults.ts
└── seeds/
    └── data.ts

frontend/apps/reality-web/src/mocks/  # MSW (reality)
├── browser.ts
├── handlers/
│   ├── index.ts
│   └── listings.ts
└── seeds/
    └── data.ts
```

### New (Phase 1 — Claude skill)

```
.claude/skills/ppt-deploy/
├── SKILL.md
├── commands/
│   ├── open-worktree.md
│   └── close-worktree.md
└── references/
    ├── api.md
    └── modes.md
```

### Modified (Phase 0)

```
.github/workflows/docker-build.yml         # consolidate matrix into single job
backend/Cargo.toml                         # [profile.release] strip+lto+codegen-units, sqlx sqlite feature, new deps
docker/backend/Dockerfile                  # mold linker for build stage
```

### Modified (Phase 1)

```
backend/Cargo.toml                         # add deploy-server as workspace member
frontend/apps/ppt-web/vite.config.ts       # use vite-plugin-ppt-worktree
frontend/apps/ppt-web/src/main.tsx         # MSW init + DevPanel mount
frontend/apps/reality-web/vite.config.ts   # use vite-plugin-ppt-worktree
frontend/apps/reality-web/src/main.tsx     # MSW init + DevPanel mount
frontend/pnpm-workspace.yaml               # include new packages
```

---

## PHASE 0 — Prerequisites

These tasks are independent of the deploy server and can run in parallel.

### Task P0.1: Consolidate CI Docker matrix into single job

Risk #1 from spec § 7: matrix split causes 10 GB GHA cache eviction. Single job using one `target/` dir halves build time and disk.

**Files:**
- Modify: `.github/workflows/docker-build.yml`
- Modify: `docker/backend/Dockerfile`

- [ ] **Step 1: Read current workflow + Dockerfile**

```bash
cat .github/workflows/docker-build.yml
cat docker/backend/Dockerfile
```

- [ ] **Step 2: Modify Dockerfile to build both binaries in one stage**

Edit `docker/backend/Dockerfile` builder stage:

```dockerfile
# In the builder stage, replace single-bin cargo build with multi-bin
RUN cargo chef cook --release --recipe-path recipe.json
COPY backend/ /app/backend/
WORKDIR /app/backend
RUN cargo build --release --bin api-server --bin reality-server
```

Add a final-stage selector via `ARG TARGET=api-server` so existing `target:` references still work:

```dockerfile
FROM debian:bookworm-slim AS api-server
ARG TARGET=api-server
COPY --from=builder /app/backend/target/release/api-server /usr/local/bin/api-server
ENTRYPOINT ["/usr/local/bin/api-server"]

FROM debian:bookworm-slim AS reality-server
COPY --from=builder /app/backend/target/release/reality-server /usr/local/bin/reality-server
ENTRYPOINT ["/usr/local/bin/reality-server"]
```

- [ ] **Step 3: Modify workflow to share cache scope**

Replace the matrix block in `.github/workflows/docker-build.yml`:

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target: [api-server, reality-server]
    steps:
      # ... unchanged through metadata ...
      - name: Build and push
        uses: docker/build-push-action@v6
        with:
          context: .
          file: docker/backend/Dockerfile
          target: ${{ matrix.target }}
          platforms: linux/amd64
          push: ${{ github.event_name != 'pull_request' && (github.ref == 'refs/heads/main' || startsWith(github.ref, 'refs/tags/v') || inputs.push == true) }}
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha,scope=backend-shared       # SHARED scope
          cache-to: type=gha,mode=max,scope=backend-shared
```

- [ ] **Step 4: Trigger CI on a feature branch and verify both targets succeed using shared cache**

Push a no-op commit to a branch:

```bash
git checkout -b ci/test-shared-cache
git commit --allow-empty -m "ci: test shared cache scope"
git push -u origin ci/test-shared-cache
```

Open the resulting GH Actions run, verify both `api-server` and `reality-server` build steps show "cache hit" for deps layer on the second run (push another empty commit).

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/docker-build.yml docker/backend/Dockerfile
git commit -m "ci: consolidate backend matrix into shared cache scope"
```

---

### Task P0.2: Optimize `[profile.release]` and add mold linker

Risk #5 from spec § 7. Strip + LTO shrinks binary; mold linker shaves 30–60 s off link.

**Files:**
- Modify: `backend/Cargo.toml`
- Modify: `docker/backend/Dockerfile`

- [ ] **Step 1: Add release profile settings to workspace Cargo.toml**

Edit `backend/Cargo.toml`, after `[workspace.dependencies]` block, append:

```toml
[profile.release]
strip = "symbols"
lto = "thin"
codegen-units = 1
```

- [ ] **Step 2: Install mold in Docker builder stage**

Edit `docker/backend/Dockerfile` builder stage, before `cargo build`:

```dockerfile
RUN apt-get update && apt-get install -y --no-install-recommends mold clang \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTFLAGS="-C link-arg=-fuse-ld=mold"
```

- [ ] **Step 3: Locally verify build still succeeds**

```bash
cd backend && cargo build --release --bin api-server
ls -lh target/release/api-server
```

Expected: binary size noticeably smaller than pre-strip (typically 80 MB → 15 MB).

- [ ] **Step 4: Commit**

```bash
git add backend/Cargo.toml docker/backend/Dockerfile
git commit -m "build: strip+lto release profile, add mold linker"
```

---

### Task P0.3: Frontend dev container Dockerfile (worktree mode)

The deploy server bind-mounts a worktree's source tree into this image and runs `pnpm dev`. Single image works for both `ppt-web` and `reality-web` (selected via `APP` env var).

**Files:**
- Create: `docker/frontend/Dockerfile.dev.worktree`

- [ ] **Step 1: Create the Dockerfile**

```dockerfile
# docker/frontend/Dockerfile.dev.worktree
# Bind-mount runner for worktree dev mode. Source is mounted at /app at runtime.
FROM node:20-alpine

WORKDIR /app

RUN apk add --no-cache git \
 && corepack enable \
 && corepack prepare pnpm@9.15.0 --activate

# pnpm store mounted as named volume → fast install across worktrees
ENV PNPM_HOME=/pnpm
ENV PATH=$PNPM_HOME:$PATH

# Default: run ppt-web; override APP=reality-web to run reality.
ENV APP=ppt-web

EXPOSE 5173 3000

ENTRYPOINT ["sh", "-c", "cd /app/frontend && pnpm install --prefer-offline --frozen-lockfile=false && pnpm --filter ./apps/$APP dev --host 0.0.0.0 --port ${PORT:-5173}"]
```

- [ ] **Step 2: Build the image locally to verify it compiles**

```bash
docker build -t ppt-frontend-dev:local -f docker/frontend/Dockerfile.dev.worktree .
docker images ppt-frontend-dev:local
```

Expected: image built, ~150 MB.

- [ ] **Step 3: Smoke-run against current repo as bind mount**

```bash
docker run --rm -d --name ppt-fe-smoke \
  -v "$(pwd):/app" \
  -v ppt-pnpm-store:/pnpm \
  -e APP=ppt-web -e PORT=5173 \
  -p 5173:5173 \
  ppt-frontend-dev:local

# wait for "ready in X ms" in logs
docker logs -f ppt-fe-smoke 2>&1 | grep -m1 "ready in"
curl -s http://localhost:5173/ | head -1   # should return HTML
docker rm -f ppt-fe-smoke
```

Expected: Vite dev server responds with HTML on port 5173.

- [ ] **Step 4: Commit**

```bash
git add docker/frontend/Dockerfile.dev.worktree
git commit -m "build: add Dockerfile.dev.worktree for bind-mount worktree dev mode"
```

---

### Task P0.4: Postgres template database script

`ppt_dev_template` is cloned for each opt-in dedicated worktree (Phase 3) and seeded for shared-default worktree DB. Schema + RLS + minimal demo data (1 building, 1 manager, 1 owner).

**Files:**
- Create: `backend/scripts/init-template-db.sql`

- [ ] **Step 1: Inspect current schema source (init-db.sql)**

```bash
wc -l backend/scripts/init-db.sql
head -30 backend/scripts/init-db.sql
```

- [ ] **Step 2: Create the template script**

```sql
-- backend/scripts/init-template-db.sql
-- Idempotent setup of ppt_dev_template (cloned for dedicated worktree DBs).

\set ON_ERROR_STOP on

DROP DATABASE IF EXISTS ppt_dev_template;
CREATE DATABASE ppt_dev_template TEMPLATE template0;

\c ppt_dev_template

-- Run the canonical schema.
\i /docker-entrypoint-initdb.d/01-init.sql

-- Minimal demo data so a fresh worktree clone is immediately usable.
INSERT INTO tenants (id, name, slug)
VALUES ('00000000-0000-0000-0000-000000000001', 'Demo Tenant', 'demo')
ON CONFLICT (id) DO NOTHING;

INSERT INTO users (id, email, password_hash, role, tenant_id, full_name)
VALUES
  ('11111111-1111-1111-1111-111111111111',
   'demo-manager@example.test',
   '$argon2id$v=19$m=19456,t=2,p=1$REPLACEME$REPLACEME',
   'manager',
   '00000000-0000-0000-0000-000000000001',
   'Demo Manager'),
  ('22222222-2222-2222-2222-222222222222',
   'demo-owner@example.test',
   '$argon2id$v=19$m=19456,t=2,p=1$REPLACEME$REPLACEME',
   'owner',
   '00000000-0000-0000-0000-000000000001',
   'Demo Owner')
ON CONFLICT (id) DO NOTHING;

INSERT INTO buildings (id, tenant_id, name, address)
VALUES ('33333333-3333-3333-3333-333333333333',
        '00000000-0000-0000-0000-000000000001',
        'Demo Building',
        'Hlavná 1, 811 01 Bratislava')
ON CONFLICT (id) DO NOTHING;

-- Mark template as ready (so deploy server knows it can clone).
COMMENT ON DATABASE ppt_dev_template IS 'ppt-deploy template, ready';
```

> **NOTE for engineer:** the column names above (`tenants.slug`, `users.role`, `buildings.address`) match the current schema as of this plan's authoring. If `01-init.sql` differs, align this script with the actual schema before committing — the demo rows should reference real columns. Run `psql -d ppt_dev_template -c '\d tenants users buildings'` to verify.

- [ ] **Step 3: Run locally and verify**

```bash
docker compose -f docker-compose.dev.yml up -d postgres
docker cp backend/scripts/init-template-db.sql ppt-postgres:/tmp/
docker exec -e PGPASSWORD=ppt_dev_password ppt-postgres \
  psql -U ppt -d postgres -f /tmp/init-template-db.sql

docker exec -e PGPASSWORD=ppt_dev_password ppt-postgres \
  psql -U ppt -d ppt_dev_template -c "SELECT count(*) FROM buildings;"
```

Expected: `count` returns 1.

- [ ] **Step 4: Test cloning works**

```bash
docker exec -e PGPASSWORD=ppt_dev_password ppt-postgres \
  psql -U ppt -d postgres -c "CREATE DATABASE ppt_wt_smoke TEMPLATE ppt_dev_template;"

docker exec -e PGPASSWORD=ppt_dev_password ppt-postgres \
  psql -U ppt -d ppt_wt_smoke -c "SELECT count(*) FROM buildings;"

docker exec -e PGPASSWORD=ppt_dev_password ppt-postgres \
  psql -U ppt -d postgres -c "DROP DATABASE ppt_wt_smoke;"
```

Expected: clone succeeds, demo data present.

- [ ] **Step 5: Commit**

```bash
git add backend/scripts/init-template-db.sql
git commit -m "db: add ppt_dev_template seed script for worktree DB cloning"
```

---

### Task P0.5: Caddy custom build with DNS plugins

`xcaddy` is the official Caddy build tool. We need cloudflare + hetzner + acme-dns plugins all compiled in; runtime selects which via config.

**Files:**
- Create: `docker/caddy/Dockerfile`

- [ ] **Step 1: Create the Dockerfile**

```dockerfile
# docker/caddy/Dockerfile
# Custom Caddy build with DNS-01 plugins for cloudflare, hetzner, acme-dns.
FROM caddy:2.8-builder AS builder

RUN xcaddy build \
    --with github.com/caddy-dns/cloudflare \
    --with github.com/caddy-dns/hetzner \
    --with github.com/caddy-dns/acmedns

FROM caddy:2.8

COPY --from=builder /usr/bin/caddy /usr/bin/caddy
```

- [ ] **Step 2: Build and verify plugins are present**

```bash
docker build -t ppt-caddy:local -f docker/caddy/Dockerfile .
docker run --rm ppt-caddy:local caddy list-modules | grep dns.providers
```

Expected output includes:
```
dns.providers.acmedns
dns.providers.cloudflare
dns.providers.hetzner
```

- [ ] **Step 3: Commit**

```bash
git add docker/caddy/Dockerfile
git commit -m "build: custom Caddy with cloudflare+hetzner+acme-dns plugins"
```

---

### Task P0.6: Operator runbook for one-time prerequisites

Manual steps the deploy server cannot automate (DNS records, secret provisioning, GH webhook registration). Documented as a checklist runbook so any operator (including the user) can run through it once.

**Files:**
- Create: `docs/runbooks/deploy-server-prereqs.md`

- [ ] **Step 1: Create the runbook**

```markdown
# Deploy Server Prerequisites Runbook

One-time setup steps that must be completed before Phase 1 server can run.

## 1. DNS

Add wildcard A/AAAA records pointing at the Hetzner VPS:

- `*.dev.rlt.sk` → <hetzner-ipv4>
- `*.dev.ppt.rlt.sk` → <hetzner-ipv4>
- `*.staging.rlt.sk` → <hetzner-ipv4>
- `*.staging.ppt.rlt.sk` → <hetzner-ipv4>
- `deploy.rlt.sk` → <hetzner-ipv4>

Verify via `dig +short test.dev.rlt.sk @1.1.1.1`.

## 2. DNS provider API token (for Caddy DNS-01)

Choose ONE provider, create an API token scoped to the relevant zone, and write it to `/etc/ppt-deploy/dns.yaml`:

- **Cloudflare (recommended):** Create a scoped token with `Zone:Read` + `DNS:Edit` for `rlt.sk`. Store as `${CF_DNS_TOKEN}`.
- **Hetzner DNS:** Create an API token in Hetzner Console. Store as `${HETZNER_DNS_TOKEN}`.
- **acme-dns:** Run `acme-dns` daemon on the box, register a credential, add CNAME `_acme-challenge.dev.rlt.sk → <random>.acme.rlt.sk`.

## 3. Caddy install

```bash
docker pull ghcr.io/<owner>/ppt-caddy:latest    # built by Phase 0 task P0.5
mkdir -p /etc/caddy /var/lib/caddy
cp deploy/caddy/Caddyfile.template /etc/caddy/Caddyfile
docker run -d --name caddy --restart=unless-stopped \
  -p 80:80 -p 443:443 -p 2019:2019 \
  -v /etc/caddy:/etc/caddy \
  -v /var/lib/caddy:/data \
  ghcr.io/<owner>/ppt-caddy:latest
```

Verify wildcard cert provisioning by adding a temporary `https://test.dev.rlt.sk` site to the Caddyfile and observing Caddy logs (`docker logs caddy`) issuing a DNS-01 challenge.

## 4. Postgres template DB

```bash
docker exec -i ppt-postgres psql -U ppt -d postgres < backend/scripts/init-template-db.sql
```

Verify: `docker exec ppt-postgres psql -U ppt -d ppt_dev_template -c '\dt'` lists tables.

## 5. Deploy server filesystem layout

```bash
sudo useradd -r -s /bin/false ppt-deploy
sudo usermod -aG docker ppt-deploy
sudo mkdir -p /etc/ppt-deploy /var/lib/ppt-deploy/{snapshots,worktrees,logs} /run/ppt-deploy
sudo chown -R ppt-deploy:ppt-deploy /var/lib/ppt-deploy /run/ppt-deploy
sudo chown root:ppt-deploy /etc/ppt-deploy && sudo chmod 750 /etc/ppt-deploy
```

## 6. GitHub deploy key

```bash
sudo -u ppt-deploy ssh-keygen -t ed25519 -N '' -f /var/lib/ppt-deploy/.ssh/id_ed25519
sudo cat /var/lib/ppt-deploy/.ssh/id_ed25519.pub
```

Add the public key to GitHub repo: Settings → Deploy keys → Add deploy key (read-only).

## 7. GH App or fine-grained PAT (server-side)

Create a fine-grained PAT scoped to the repo:
- `Actions: Read and write` (for `workflow_dispatch`)
- `Contents: Read`
- `Metadata: Read`
- `Packages: Read`

Store in `/etc/ppt-deploy/auth.yaml` as `gh_api_token`.

## 8. GH OIDC issuer trust

In `/etc/ppt-deploy/auth.yaml`:
```yaml
oidc:
  issuer: https://token.actions.githubusercontent.com
  jwks_url: https://token.actions.githubusercontent.com/.well-known/jwks
  audience: ppt-deploy
  allowed_repos:
    - martin-janci/property-management
  allowed_refs:
    - refs/heads/main
    - refs/heads/feature/*
    - refs/tags/v*
```

## 9. systemd units

After Phase 1 implementation, install:
```bash
sudo cp backend/servers/deploy-server/systemd/*.{socket,service,timer} /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now ppt-deploy.socket ppt-deploy-gc.timer
```

## 10. GitHub webhook

Repo → Settings → Webhooks → Add webhook:
- Payload URL: `https://deploy.rlt.sk/api/webhook/github`
- Content type: `application/json`
- Secret: generated value, also written to `/etc/ppt-deploy/auth.yaml` as `webhook_secret`
- Events: Pull requests, Pushes, Packages
```

- [ ] **Step 2: Commit**

```bash
git add docs/runbooks/deploy-server-prereqs.md
git commit -m "docs: deploy server one-time prerequisites runbook"
```

---

## PHASE 1 — MVP Server, CLI, Frontend Integration, Claude skill

### Task P1.1: Add `deploy-server` crate to workspace

**Files:**
- Modify: `backend/Cargo.toml`
- Create: `backend/servers/deploy-server/Cargo.toml`
- Create: `backend/servers/deploy-server/src/main.rs`
- Create: `backend/servers/deploy-server/src/lib.rs`

- [ ] **Step 1: Add workspace member**

Edit `backend/Cargo.toml`, in the `[workspace] members` array, after `"servers/reality-server",`:

```toml
    "servers/reality-server",
    "servers/deploy-server",
]
```

Add new workspace dependencies under `[workspace.dependencies]`:

```toml
# Phase 1: deploy-server
bollard = "0.16"
listenfd = "1.0"
sqlx-sqlite-not-loaded-here-but-via-features = "ignore"   # marker, we use sqlx with sqlite feature
tokio-util = { version = "0.7", features = ["io"] }
futures-util = "0.3"
async-trait = "0.1"
tempfile = "3.10"
which = "6.0"
```

(Leave `sqlx` workspace dep unchanged; the deploy-server's Cargo.toml will request the `sqlite` feature additionally.)

- [ ] **Step 2: Create deploy-server Cargo.toml**

```toml
# backend/servers/deploy-server/Cargo.toml
[package]
name = "deploy-server"
version.workspace = true
edition.workspace = true

[lib]
name = "deploy_server"
path = "src/lib.rs"

[[bin]]
name = "ppt-deploy"
path = "src/main.rs"

[[bin]]
name = "pmctl"
path = "src/bin/pmctl.rs"

[dependencies]
tokio = { workspace = true }
axum = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
serde_yaml = "0.9"
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite", "chrono", "json", "migrate"] }
jsonwebtoken = { workspace = true }
clap = { workspace = true }
listenfd = "1.0"
bollard = "0.16"
tokio-util = { version = "0.7", features = ["io"] }
futures-util = "0.3"
async-trait = "0.1"
tempfile = "3.10"
which = "6.0"
reqwest = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
chrono = { workspace = true }
hmac = { workspace = true }
sha2 = { workspace = true }
hex = { workspace = true }
uuid = { workspace = true }

[dev-dependencies]
httpmock = "0.7"
tempfile = "3.10"
```

- [ ] **Step 3: Create stub lib.rs and main.rs**

```rust
// backend/servers/deploy-server/src/lib.rs
//! ppt-deploy: on-demand worktree/staging/prod deployment control plane.

pub mod api;
pub mod auth;
pub mod config;
pub mod domain;
pub mod error;
pub mod infra;
```

```rust
// backend/servers/deploy-server/src/main.rs
fn main() -> anyhow::Result<()> {
    println!("ppt-deploy stub");
    Ok(())
}
```

Create empty module files so `cargo check` succeeds:

```bash
mkdir -p backend/servers/deploy-server/src/{api,auth,domain,infra,bin}
touch backend/servers/deploy-server/src/{api,auth,domain,infra}/mod.rs
touch backend/servers/deploy-server/src/{config.rs,error.rs}
echo 'fn main() { println!("pmctl stub"); }' > backend/servers/deploy-server/src/bin/pmctl.rs
```

- [ ] **Step 4: Verify workspace compiles**

```bash
cd backend && cargo check -p deploy-server
```

Expected: compiles cleanly. If sqlx-sqlite version conflict appears, drop the marker workspace entry (it was just a comment).

- [ ] **Step 5: Commit**

```bash
git add backend/Cargo.toml backend/servers/deploy-server/
git commit -m "feat(deploy-server): scaffold crate with bin targets"
```

---

### Task P1.2: Domain types

Self-contained, no external deps. TDD with serde round-trips.

**Files:**
- Create: `backend/servers/deploy-server/src/domain/worktree.rs`
- Create: `backend/servers/deploy-server/src/domain/release.rs`
- Modify: `backend/servers/deploy-server/src/domain/mod.rs`

- [ ] **Step 1: Write failing test for Worktree serde**

```rust
// backend/servers/deploy-server/src/domain/worktree.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackendMode {
    Shared,
    Dedicated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorktreeState {
    Running,
    Paused,
    Closing,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorktreeUrls {
    pub ppt: Option<String>,
    pub reality: Option<String>,
    pub api: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worktree {
    pub name: String,
    pub branch: String,
    pub backend_mode: BackendMode,
    pub state: WorktreeState,
    pub urls: WorktreeUrls,
    pub containers: Vec<String>,
    pub db_name: Option<String>,
    pub dump_path: Option<String>,
    pub ttl_seconds: i64,
    pub last_traffic_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_round_trip_json() {
        let wt = Worktree {
            name: "feature-uc14".into(),
            branch: "feature/UC-14".into(),
            backend_mode: BackendMode::Shared,
            state: WorktreeState::Running,
            urls: WorktreeUrls {
                ppt: Some("https://wt-feature-uc14.dev.ppt.rlt.sk".into()),
                reality: Some("https://wt-feature-uc14.dev.rlt.sk".into()),
                api: None,
            },
            containers: vec!["wt-feature-uc14-ppt".into(), "wt-feature-uc14-reality".into()],
            db_name: None,
            dump_path: None,
            ttl_seconds: 172_800,
            last_traffic_at: None,
            closed_at: None,
            created_at: Utc::now(),
            created_by: "oidc:martin-janci/property-management@feature/UC-14".into(),
        };

        let json = serde_json::to_string(&wt).unwrap();
        let parsed: Worktree = serde_json::from_str(&json).unwrap();
        assert_eq!(wt.name, parsed.name);
        assert_eq!(wt.backend_mode, parsed.backend_mode);
        assert_eq!(wt.state, parsed.state);
    }

    #[test]
    fn backend_mode_serializes_lowercase() {
        let json = serde_json::to_string(&BackendMode::Dedicated).unwrap();
        assert_eq!(json, "\"dedicated\"");
    }
}
```

- [ ] **Step 2: Create release.rs with similar shape**

```rust
// backend/servers/deploy-server/src/domain/release.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseState {
    Candidate,
    Staging,
    Prod,
    Previous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub tag: String,
    /// Map of service name (e.g. "api-server") → image ref.
    pub images: HashMap<String, String>,
    pub state: ReleaseState,
    pub target: Option<String>,
    pub promoted_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}
```

- [ ] **Step 3: Wire mod.rs**

```rust
// backend/servers/deploy-server/src/domain/mod.rs
pub mod release;
pub mod worktree;

pub use release::{Release, ReleaseState};
pub use worktree::{BackendMode, Worktree, WorktreeState, WorktreeUrls};
```

- [ ] **Step 4: Run tests**

```bash
cd backend && cargo test -p deploy-server domain::worktree
```

Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add backend/servers/deploy-server/src/domain/
git commit -m "feat(deploy-server): domain types Worktree, Release"
```

---

### Task P1.3: Error type

**Files:**
- Modify: `backend/servers/deploy-server/src/error.rs`

- [ ] **Step 1: Define unified error**

```rust
// backend/servers/deploy-server/src/error.rs
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeployError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("already exists: {0}")]
    Conflict(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("invalid input: {0}")]
    BadRequest(String),

    #[error("docker error: {0}")]
    Docker(#[from] bollard::errors::Error),

    #[error("sqlite error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("http client error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("internal: {0}")]
    Internal(String),
}

impl DeployError {
    fn status(&self) -> StatusCode {
        match self {
            DeployError::NotFound(_) => StatusCode::NOT_FOUND,
            DeployError::Conflict(_) => StatusCode::CONFLICT,
            DeployError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            DeployError::Forbidden(_) => StatusCode::FORBIDDEN,
            DeployError::BadRequest(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for DeployError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = Json(json!({
            "error": self.to_string(),
        }));
        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, DeployError>;
```

- [ ] **Step 2: Wire into lib.rs**

```rust
// backend/servers/deploy-server/src/lib.rs
//! ppt-deploy: on-demand worktree/staging/prod deployment control plane.

pub mod api;
pub mod auth;
pub mod config;
pub mod domain;
pub mod error;
pub mod infra;

pub use error::{DeployError, Result};
```

- [ ] **Step 3: Compile-check**

```bash
cd backend && cargo check -p deploy-server
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add backend/servers/deploy-server/src/error.rs backend/servers/deploy-server/src/lib.rs
git commit -m "feat(deploy-server): unified DeployError + axum IntoResponse"
```

---

### Task P1.4: Configuration loading

YAML config with env-var substitution. Three files: `config.yaml`, `targets.yaml`, `auth.yaml`, `dns.yaml`. Loaded once at startup.

**Files:**
- Modify: `backend/servers/deploy-server/src/config.rs`

- [ ] **Step 1: Write failing test**

```rust
// backend/servers/deploy-server/src/config.rs (top)
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub bind: String,
    pub state_dir: String,
    pub worktree_dir: String,
    pub snapshot_dir: String,
    #[serde(default = "default_ttl")]
    pub default_ttl_seconds: i64,
    pub idle_pause_seconds: i64,
    pub idle_stop_seconds: i64,
    pub git_repo_url: String,
}

fn default_ttl() -> i64 { 172_800 }

#[derive(Debug, Clone, Deserialize)]
pub struct TargetsConfig {
    pub targets: HashMap<String, Target>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Target {
    pub docker_socket: String,
    pub caddy_url: String,
    pub domain_suffix: String,
    #[serde(default)]
    pub idle_timeout: Option<String>,
    #[serde(default)]
    pub promote_strategy: Option<String>,
    #[serde(default = "default_rollback_mode")]
    pub rollback_mode: String,
    #[serde(default)]
    pub health_grace: Option<String>,
}

fn default_rollback_mode() -> String { "manual".into() }

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub api_keys: Vec<ApiKey>,
    pub oidc: OidcConfig,
    pub webhook_secret: String,
    pub gh_api_token: String,
    pub gh_deploy_key_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiKey {
    pub name: String,
    pub hash: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OidcConfig {
    pub issuer: String,
    pub jwks_url: String,
    pub audience: String,
    pub allowed_repos: Vec<String>,
    pub allowed_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DnsConfig {
    pub provider: String,
    #[serde(flatten)]
    pub providers: serde_yaml::Value,
}

pub fn load_yaml<T: serde::de::DeserializeOwned>(path: &Path) -> crate::Result<T> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| crate::DeployError::Config(format!("read {}: {e}", path.display())))?;
    let expanded = shellexpand::env(&raw)
        .map_err(|e| crate::DeployError::Config(format!("env expand {}: {e}", path.display())))?;
    serde_yaml::from_str(&expanded)
        .map_err(|e| crate::DeployError::Config(format!("parse {}: {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::io::Write;

    #[test]
    fn loads_config_with_env_substitution() {
        std::env::set_var("PPT_TEST_REPO", "git@github.com:test/repo.git");
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "bind: 0.0.0.0:8443\nstate_dir: /var/lib/ppt-deploy\nworktree_dir: /var/lib/ppt-deploy/worktrees\nsnapshot_dir: /var/lib/ppt-deploy/snapshots\nidle_pause_seconds: 1800\nidle_stop_seconds: 86400\ngit_repo_url: ${{PPT_TEST_REPO}}").unwrap();
        let cfg: Config = load_yaml(&path).unwrap();
        assert_eq!(cfg.bind, "0.0.0.0:8443");
        assert_eq!(cfg.git_repo_url, "git@github.com:test/repo.git");
        assert_eq!(cfg.default_ttl_seconds, 172_800);
    }
}
```

- [ ] **Step 2: Add `shellexpand` dep**

Edit `backend/servers/deploy-server/Cargo.toml`, under `[dependencies]`:

```toml
shellexpand = "3.1"
```

- [ ] **Step 3: Run test**

```bash
cd backend && cargo test -p deploy-server config::tests
```

Expected: pass.

- [ ] **Step 4: Commit**

```bash
git add backend/servers/deploy-server/Cargo.toml backend/servers/deploy-server/src/config.rs
git commit -m "feat(deploy-server): YAML config with env-var substitution"
```

---

### Task P1.5: sqlx migrations + state store

**Files:**
- Create: `backend/servers/deploy-server/migrations/0001_init.sql`
- Create: `backend/servers/deploy-server/src/infra/store.rs`
- Modify: `backend/servers/deploy-server/src/infra/mod.rs`

- [ ] **Step 1: Create migration**

```sql
-- backend/servers/deploy-server/migrations/0001_init.sql
CREATE TABLE worktree (
  name              TEXT PRIMARY KEY,
  branch            TEXT NOT NULL,
  backend_mode      TEXT NOT NULL,
  state             TEXT NOT NULL,
  urls              TEXT NOT NULL,                 -- JSON
  containers        TEXT NOT NULL,                 -- JSON array
  db_name           TEXT,
  dump_path         TEXT,
  ttl_seconds       INTEGER NOT NULL DEFAULT 172800,
  last_traffic_at   INTEGER,                       -- unix ts seconds
  closed_at         INTEGER,
  created_at        INTEGER NOT NULL,
  created_by        TEXT NOT NULL
);

CREATE INDEX idx_worktree_state ON worktree(state);

CREATE TABLE release (
  tag               TEXT PRIMARY KEY,
  images            TEXT NOT NULL,                 -- JSON
  state             TEXT NOT NULL,
  target            TEXT,
  promoted_at       INTEGER,
  notes             TEXT
);

CREATE TABLE audit (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  ts                INTEGER NOT NULL,
  caller_kind       TEXT NOT NULL,
  caller_id         TEXT NOT NULL,
  endpoint          TEXT NOT NULL,
  params            TEXT,                          -- JSON
  result            TEXT,
  duration_ms       INTEGER
);

CREATE INDEX idx_audit_ts ON audit(ts);
```

- [ ] **Step 2: Write the store with TDD**

```rust
// backend/servers/deploy-server/src/infra/store.rs
use crate::domain::{BackendMode, Worktree, WorktreeState, WorktreeUrls};
use crate::Result;
use chrono::{DateTime, TimeZone, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::str::FromStr;

#[derive(Clone)]
pub struct Store {
    pool: Pool<Sqlite>,
}

impl Store {
    pub async fn open(db_path: &Path) -> Result<Self> {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}?mode=rwc", db_path.display()))
            .map_err(|e| crate::DeployError::Config(format!("sqlite opts: {e}")))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(opts)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await
            .map_err(|e| crate::DeployError::Internal(format!("migrate: {e}")))?;

        Ok(Self { pool })
    }

    pub async fn upsert_worktree(&self, wt: &Worktree) -> Result<()> {
        let urls = serde_json::to_string(&wt.urls).unwrap();
        let containers = serde_json::to_string(&wt.containers).unwrap();
        let backend = match wt.backend_mode { BackendMode::Shared => "shared", BackendMode::Dedicated => "dedicated" };
        let state = match wt.state {
            WorktreeState::Running => "running",
            WorktreeState::Paused => "paused",
            WorktreeState::Closing => "closing",
            WorktreeState::Closed => "closed",
        };

        sqlx::query(
            r#"INSERT INTO worktree
                (name, branch, backend_mode, state, urls, containers, db_name, dump_path,
                 ttl_seconds, last_traffic_at, closed_at, created_at, created_by)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(name) DO UPDATE SET
                  branch=excluded.branch,
                  backend_mode=excluded.backend_mode,
                  state=excluded.state,
                  urls=excluded.urls,
                  containers=excluded.containers,
                  db_name=excluded.db_name,
                  dump_path=excluded.dump_path,
                  ttl_seconds=excluded.ttl_seconds,
                  last_traffic_at=excluded.last_traffic_at,
                  closed_at=excluded.closed_at"#,
        )
        .bind(&wt.name)
        .bind(&wt.branch)
        .bind(backend)
        .bind(state)
        .bind(urls)
        .bind(containers)
        .bind(wt.db_name.as_deref())
        .bind(wt.dump_path.as_deref())
        .bind(wt.ttl_seconds)
        .bind(wt.last_traffic_at.map(|t| t.timestamp()))
        .bind(wt.closed_at.map(|t| t.timestamp()))
        .bind(wt.created_at.timestamp())
        .bind(&wt.created_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_worktree(&self, name: &str) -> Result<Option<Worktree>> {
        let row = sqlx::query_as::<_, WorktreeRow>(
            r#"SELECT name, branch, backend_mode, state, urls, containers, db_name, dump_path,
                       ttl_seconds, last_traffic_at, closed_at, created_at, created_by
                FROM worktree WHERE name = ?"#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        row.map(WorktreeRow::into_domain).transpose()
    }

    pub async fn list_worktrees(&self) -> Result<Vec<Worktree>> {
        let rows = sqlx::query_as::<_, WorktreeRow>(
            r#"SELECT name, branch, backend_mode, state, urls, containers, db_name, dump_path,
                       ttl_seconds, last_traffic_at, closed_at, created_at, created_by
                FROM worktree ORDER BY created_at DESC"#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(WorktreeRow::into_domain).collect()
    }

    pub async fn record_audit(
        &self,
        caller_kind: &str,
        caller_id: &str,
        endpoint: &str,
        params: Option<&str>,
        result: &str,
        duration_ms: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO audit (ts, caller_kind, caller_id, endpoint, params, result, duration_ms)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(Utc::now().timestamp())
        .bind(caller_kind)
        .bind(caller_id)
        .bind(endpoint)
        .bind(params)
        .bind(result)
        .bind(duration_ms)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct WorktreeRow {
    name: String,
    branch: String,
    backend_mode: String,
    state: String,
    urls: String,
    containers: String,
    db_name: Option<String>,
    dump_path: Option<String>,
    ttl_seconds: i64,
    last_traffic_at: Option<i64>,
    closed_at: Option<i64>,
    created_at: i64,
    created_by: String,
}

impl WorktreeRow {
    fn into_domain(self) -> Result<Worktree> {
        let backend_mode = match self.backend_mode.as_str() {
            "shared" => BackendMode::Shared,
            "dedicated" => BackendMode::Dedicated,
            other => return Err(crate::DeployError::Internal(format!("bad backend_mode {other}"))),
        };
        let state = match self.state.as_str() {
            "running" => WorktreeState::Running,
            "paused" => WorktreeState::Paused,
            "closing" => WorktreeState::Closing,
            "closed" => WorktreeState::Closed,
            other => return Err(crate::DeployError::Internal(format!("bad state {other}"))),
        };
        let urls: WorktreeUrls = serde_json::from_str(&self.urls)
            .map_err(|e| crate::DeployError::Internal(format!("bad urls json: {e}")))?;
        let containers: Vec<String> = serde_json::from_str(&self.containers)
            .map_err(|e| crate::DeployError::Internal(format!("bad containers json: {e}")))?;
        Ok(Worktree {
            name: self.name,
            branch: self.branch,
            backend_mode,
            state,
            urls,
            containers,
            db_name: self.db_name,
            dump_path: self.dump_path,
            ttl_seconds: self.ttl_seconds,
            last_traffic_at: self.last_traffic_at.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
            closed_at: self.closed_at.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
            created_at: Utc.timestamp_opt(self.created_at, 0).unwrap(),
            created_by: self.created_by,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn upsert_and_get_round_trip() {
        let dir = tempdir().unwrap();
        let store = Store::open(&dir.path().join("state.db")).await.unwrap();
        let wt = Worktree {
            name: "foo".into(),
            branch: "feature/foo".into(),
            backend_mode: BackendMode::Shared,
            state: WorktreeState::Running,
            urls: WorktreeUrls { ppt: Some("https://x".into()), reality: None, api: None },
            containers: vec!["c1".into()],
            db_name: None,
            dump_path: None,
            ttl_seconds: 7200,
            last_traffic_at: None,
            closed_at: None,
            created_at: Utc::now(),
            created_by: "test".into(),
        };
        store.upsert_worktree(&wt).await.unwrap();
        let got = store.get_worktree("foo").await.unwrap().unwrap();
        assert_eq!(got.branch, "feature/foo");
        assert_eq!(got.containers, vec!["c1".to_string()]);

        let list = store.list_worktrees().await.unwrap();
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn audit_insert() {
        let dir = tempdir().unwrap();
        let store = Store::open(&dir.path().join("state.db")).await.unwrap();
        store.record_audit("api_key", "claude-skill", "POST /api/worktree", Some("{}"), "ok", 42).await.unwrap();
        let row: (i64,) = sqlx::query_as("SELECT count(*) FROM audit")
            .fetch_one(&store.pool).await.unwrap();
        assert_eq!(row.0, 1);
    }
}
```

- [ ] **Step 3: Wire mod.rs**

```rust
// backend/servers/deploy-server/src/infra/mod.rs
pub mod store;
pub use store::Store;
```

- [ ] **Step 4: Run tests**

```bash
cd backend && cargo test -p deploy-server infra::store
```

Expected: 2 tests pass. (sqlx may complain about offline mode — set `SQLX_OFFLINE=false` or run with `DATABASE_URL` unset; we use runtime-checked queries so it should not.)

- [ ] **Step 5: Commit**

```bash
git add backend/servers/deploy-server/migrations backend/servers/deploy-server/src/infra/
git commit -m "feat(deploy-server): sqlite store with worktree/release/audit tables"
```

---

### Task P1.6: Caddy admin API client

**Files:**
- Create: `backend/servers/deploy-server/src/infra/caddy.rs`
- Modify: `backend/servers/deploy-server/src/infra/mod.rs`

- [ ] **Step 1: Write client + tests using httpmock**

```rust
// backend/servers/deploy-server/src/infra/caddy.rs
use crate::Result;
use serde_json::json;

pub struct CaddyClient {
    base: String,
    http: reqwest::Client,
}

impl CaddyClient {
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            http: reqwest::Client::builder().build().unwrap(),
        }
    }

    /// Register a host → upstream mapping. Idempotent: replaces existing route for `host`.
    pub async fn register_route(&self, host: &str, upstream: &str) -> Result<()> {
        let route_id = format!("ppt-deploy-{}", sanitize_id(host));
        let payload = json!({
            "@id": route_id,
            "match": [{"host": [host]}],
            "handle": [
                {
                    "handler": "reverse_proxy",
                    "upstreams": [{"dial": upstream}]
                }
            ]
        });
        // PUT replaces if exists, creates otherwise (Caddy admin API semantics).
        let url = format!("{}/id/{}", self.base, route_id);
        let resp = self.http.put(&url).json(&payload).send().await?;
        if !resp.status().is_success() {
            // Fallback: route doesn't exist yet, append to apps.http.servers.srv0.routes.
            let append_url = format!("{}/config/apps/http/servers/srv0/routes/...", self.base);
            self.http.post(&append_url)
                .json(&json!([payload]))
                .send().await?
                .error_for_status()?;
        }
        Ok(())
    }

    pub async fn unregister_route(&self, host: &str) -> Result<()> {
        let route_id = format!("ppt-deploy-{}", sanitize_id(host));
        let url = format!("{}/id/{}", self.base, route_id);
        let resp = self.http.delete(&url).send().await?;
        // 200 OK or 404 (already gone) are both fine.
        if resp.status().is_success() || resp.status().as_u16() == 404 {
            Ok(())
        } else {
            Err(crate::DeployError::Internal(format!("caddy unregister: {}", resp.status())))
        }
    }
}

fn sanitize_id(host: &str) -> String {
    host.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[tokio::test]
    async fn register_route_calls_admin_api() {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(PUT).path_contains("/id/ppt-deploy-");
            then.status(200);
        });
        let client = CaddyClient::new(server.base_url());
        client.register_route("wt-uc14.dev.ppt.rlt.sk", "127.0.0.1:51001").await.unwrap();
        m.assert();
    }

    #[tokio::test]
    async fn unregister_404_is_ok() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(DELETE).path_contains("/id/ppt-deploy-");
            then.status(404);
        });
        let client = CaddyClient::new(server.base_url());
        client.unregister_route("missing.dev.rlt.sk").await.unwrap();
    }
}
```

- [ ] **Step 2: Wire mod.rs**

```rust
// backend/servers/deploy-server/src/infra/mod.rs
pub mod caddy;
pub mod store;

pub use caddy::CaddyClient;
pub use store::Store;
```

- [ ] **Step 3: Run tests**

```bash
cd backend && cargo test -p deploy-server infra::caddy
```

Expected: 2 tests pass.

- [ ] **Step 4: Commit**

```bash
git add backend/servers/deploy-server/src/infra/
git commit -m "feat(deploy-server): Caddy admin API client"
```

---

### Task P1.7: Docker SDK wrapper (frontend dev container)

**Files:**
- Create: `backend/servers/deploy-server/src/infra/docker.rs`
- Modify: `backend/servers/deploy-server/src/infra/mod.rs`

- [ ] **Step 1: Write the wrapper**

```rust
// backend/servers/deploy-server/src/infra/docker.rs
use crate::Result;
use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions, StopContainerOptions};
use bollard::models::{HostConfig, Mount, MountTypeEnum, PortBinding};
use bollard::Docker;
use std::collections::HashMap;

pub struct DockerClient {
    docker: Docker,
}

#[derive(Debug, Clone)]
pub struct FrontendDevSpec {
    pub container_name: String,    // e.g. "wt-uc14-ppt"
    pub app: String,                // "ppt-web" or "reality-web"
    pub source_path: String,        // host path to worktree (mounted as /app)
    pub host_port: u16,             // host port to bind container 5173
    pub pnpm_volume: String,        // named volume for /pnpm
    pub image: String,              // "ppt-frontend-dev:local" until we publish
}

impl DockerClient {
    pub fn from_socket(docker_socket: &str) -> Result<Self> {
        let docker = if docker_socket.starts_with("unix://") {
            Docker::connect_with_unix(docker_socket, 30, bollard::API_DEFAULT_VERSION)?
        } else if docker_socket.starts_with("ssh://") {
            // SSH-tunneled docker socket (Phase 5). Not used in MVP.
            return Err(crate::DeployError::Config(format!("ssh:// docker socket not supported in MVP: {docker_socket}")));
        } else {
            Docker::connect_with_local_defaults()?
        };
        Ok(Self { docker })
    }

    pub async fn run_frontend_dev(&self, spec: &FrontendDevSpec) -> Result<String> {
        // Idempotency: if a container with the same name exists, remove it first.
        let _ = self.docker.remove_container(
            &spec.container_name,
            Some(RemoveContainerOptions { force: true, ..Default::default() }),
        ).await;

        let mut env = vec![
            format!("APP={}", spec.app),
            format!("PORT=5173"),
        ];
        env.push(format!("PNPM_HOME=/pnpm"));

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            "5173/tcp".to_string(),
            Some(vec![PortBinding {
                host_ip: Some("127.0.0.1".to_string()),
                host_port: Some(spec.host_port.to_string()),
            }]),
        );

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert("5173/tcp".to_string(), HashMap::<(), ()>::new());

        let mounts = vec![
            Mount {
                target: Some("/app".to_string()),
                source: Some(spec.source_path.clone()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(false),
                ..Default::default()
            },
            Mount {
                target: Some("/pnpm".to_string()),
                source: Some(spec.pnpm_volume.clone()),
                typ: Some(MountTypeEnum::VOLUME),
                ..Default::default()
            },
        ];

        let host_config = HostConfig {
            mounts: Some(mounts),
            port_bindings: Some(port_bindings),
            restart_policy: Some(bollard::models::RestartPolicy {
                name: Some(bollard::models::RestartPolicyNameEnum::UNLESS_STOPPED),
                ..Default::default()
            }),
            ..Default::default()
        };

        let config = Config {
            image: Some(spec.image.clone()),
            env: Some(env),
            exposed_ports: Some(exposed_ports),
            host_config: Some(host_config),
            ..Default::default()
        };

        let create = self.docker.create_container(
            Some(CreateContainerOptions { name: spec.container_name.clone(), platform: None }),
            config,
        ).await?;

        self.docker.start_container(&create.id, None::<StartContainerOptions<String>>).await?;

        Ok(create.id)
    }

    pub async fn stop_container(&self, name: &str) -> Result<()> {
        let _ = self.docker.stop_container(name, Some(StopContainerOptions { t: 10 })).await;
        Ok(())
    }

    pub async fn remove_container(&self, name: &str) -> Result<()> {
        let _ = self.docker.remove_container(
            name,
            Some(RemoveContainerOptions { force: true, ..Default::default() }),
        ).await;
        Ok(())
    }
}
```

- [ ] **Step 2: Wire mod.rs and add a smoke-style test**

```rust
// backend/servers/deploy-server/src/infra/mod.rs
pub mod caddy;
pub mod docker;
pub mod store;

pub use caddy::CaddyClient;
pub use docker::{DockerClient, FrontendDevSpec};
pub use store::Store;
```

```rust
// at the bottom of docker.rs, add
#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: requires local docker daemon. Skipped in CI by checking env.
    #[tokio::test]
    #[ignore]
    async fn run_frontend_dev_against_local_docker() {
        let client = DockerClient::from_socket("unix:///var/run/docker.sock").unwrap();
        let spec = FrontendDevSpec {
            container_name: "ppt-deploy-test-fe".into(),
            app: "ppt-web".into(),
            source_path: std::env::current_dir().unwrap().parent().unwrap().to_string_lossy().to_string(),
            host_port: 51999,
            pnpm_volume: "ppt-deploy-test-pnpm".into(),
            image: "ppt-frontend-dev:local".into(),
        };
        client.run_frontend_dev(&spec).await.unwrap();
        client.stop_container(&spec.container_name).await.unwrap();
        client.remove_container(&spec.container_name).await.unwrap();
    }
}
```

- [ ] **Step 3: Compile-check**

```bash
cd backend && cargo check -p deploy-server
```

Expected: clean. (Smoke test is `#[ignore]` so unit `cargo test` does not run it.)

- [ ] **Step 4: Optional manual smoke test**

```bash
cd backend && cargo test -p deploy-server -- --ignored docker::tests::run_frontend_dev
```

Run only when local docker is available and `ppt-frontend-dev:local` image is built (P0.3).

- [ ] **Step 5: Commit**

```bash
git add backend/servers/deploy-server/src/infra/
git commit -m "feat(deploy-server): bollard-based docker client for frontend dev containers"
```

---

### Task P1.8: Git fetch wrapper with per-branch lock

**Files:**
- Create: `backend/servers/deploy-server/src/infra/git.rs`
- Modify: `backend/servers/deploy-server/src/infra/mod.rs`

- [ ] **Step 1: Write wrapper + tests**

```rust
// backend/servers/deploy-server/src/infra/git.rs
use crate::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct GitFetcher {
    repo_url: String,
    worktree_dir: PathBuf,
    deploy_key_path: PathBuf,
    locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl GitFetcher {
    pub fn new(repo_url: impl Into<String>, worktree_dir: impl Into<PathBuf>, deploy_key_path: impl Into<PathBuf>) -> Self {
        Self {
            repo_url: repo_url.into(),
            worktree_dir: worktree_dir.into(),
            deploy_key_path: deploy_key_path.into(),
            locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Fetch a branch into `<worktree_dir>/<sanitized_branch>/`.
    /// Per-branch lock prevents two concurrent calls for the same branch racing.
    pub async fn fetch_branch(&self, branch: &str) -> Result<PathBuf> {
        let lock = {
            let mut locks = self.locks.lock().await;
            locks.entry(branch.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _guard = lock.lock().await;

        let dest = self.worktree_dir.join(sanitize(branch));
        let ssh_cmd = format!("ssh -i {} -o StrictHostKeyChecking=accept-new", self.deploy_key_path.display());

        if dest.join(".git").exists() {
            // Update existing
            self.run_git(&dest, &ssh_cmd, &["fetch", "origin", branch]).await?;
            self.run_git(&dest, &ssh_cmd, &["reset", "--hard", &format!("origin/{branch}")]).await?;
        } else {
            // Fresh clone
            tokio::fs::create_dir_all(&self.worktree_dir).await?;
            self.run_git_in(&self.worktree_dir, &ssh_cmd, &[
                "clone", "--branch", branch, "--depth", "1",
                &self.repo_url, dest.to_str().unwrap(),
            ]).await?;
        }
        Ok(dest)
    }

    async fn run_git(&self, cwd: &Path, ssh_cmd: &str, args: &[&str]) -> Result<()> {
        self.run_git_in(cwd, ssh_cmd, args).await
    }

    async fn run_git_in(&self, cwd: &Path, ssh_cmd: &str, args: &[&str]) -> Result<()> {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GIT_SSH_COMMAND", ssh_cmd)
            .output()
            .await?;
        if !output.status.success() {
            return Err(crate::DeployError::Internal(format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(())
    }
}

pub fn sanitize(branch: &str) -> String {
    branch.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_branch_to_subdomain() {
        assert_eq!(sanitize("feature/UC-14"), "feature-uc-14");
        assert_eq!(sanitize("hotfix/Critical Fix"), "hotfix-critical-fix");
        assert_eq!(sanitize("///---bad---///"), "bad");
    }

    #[tokio::test]
    async fn fetch_with_local_repo_fixture() {
        // Create a tiny local bare repo + clone, simulating origin.
        let tmp = tempfile::tempdir().unwrap();
        let bare = tmp.path().join("origin.git");
        let work = tmp.path().join("seed");
        std::fs::create_dir_all(&bare).unwrap();
        std::process::Command::new("git").args(["init", "--bare"]).arg(&bare).status().unwrap();
        std::process::Command::new("git").args(["init"]).arg(&work).status().unwrap();
        std::process::Command::new("git").args(["-C", work.to_str().unwrap(), "config", "user.email", "t@t"]).status().unwrap();
        std::process::Command::new("git").args(["-C", work.to_str().unwrap(), "config", "user.name", "t"]).status().unwrap();
        std::fs::write(work.join("README.md"), "hi").unwrap();
        std::process::Command::new("git").args(["-C", work.to_str().unwrap(), "add", "."]).status().unwrap();
        std::process::Command::new("git").args(["-C", work.to_str().unwrap(), "commit", "-m", "init"]).status().unwrap();
        std::process::Command::new("git").args(["-C", work.to_str().unwrap(), "branch", "-M", "feature-x"]).status().unwrap();
        std::process::Command::new("git").args(["-C", work.to_str().unwrap(), "remote", "add", "origin"]).arg(&bare).status().unwrap();
        std::process::Command::new("git").args(["-C", work.to_str().unwrap(), "push", "-u", "origin", "feature-x"]).status().unwrap();

        let dest_root = tmp.path().join("worktrees");
        let fetcher = GitFetcher::new(
            bare.to_string_lossy().to_string(),
            dest_root.clone(),
            "/dev/null",   // unused in this local fixture
        );
        // For local repos GIT_SSH_COMMAND is irrelevant; git uses file:// transport.
        let dest = fetcher.fetch_branch("feature-x").await.unwrap();
        assert!(dest.join("README.md").exists());
    }
}
```

- [ ] **Step 2: Wire mod.rs**

```rust
// backend/servers/deploy-server/src/infra/mod.rs
pub mod caddy;
pub mod docker;
pub mod git;
pub mod store;

pub use caddy::CaddyClient;
pub use docker::{DockerClient, FrontendDevSpec};
pub use git::GitFetcher;
pub use store::Store;
```

- [ ] **Step 3: Run tests**

```bash
cd backend && cargo test -p deploy-server infra::git
```

Expected: 2 tests pass (sanitize unit + local-repo fetch).

- [ ] **Step 4: Commit**

```bash
git add backend/servers/deploy-server/src/infra/
git commit -m "feat(deploy-server): git fetch wrapper with per-branch lock"
```

---

### Task P1.9: Auth — API key + GH OIDC validators

**Files:**
- Create: `backend/servers/deploy-server/src/auth/api_key.rs`
- Create: `backend/servers/deploy-server/src/auth/oidc.rs`
- Modify: `backend/servers/deploy-server/src/auth/mod.rs`

- [ ] **Step 1: API key validator (sha256 of stored hash)**

```rust
// backend/servers/deploy-server/src/auth/api_key.rs
use crate::config::ApiKey;
use sha2::{Digest, Sha256};

pub struct ApiKeyValidator {
    keys: Vec<ApiKey>,
}

impl ApiKeyValidator {
    pub fn new(keys: Vec<ApiKey>) -> Self { Self { keys } }

    pub fn validate(&self, presented: &str) -> Option<&str> {
        let presented_hash = hex::encode(Sha256::digest(presented.as_bytes()));
        self.keys.iter().find(|k| k.hash == presented_hash).map(|k| k.name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn matching_hash_returns_name() {
        let key = "secret-token-abc";
        let hash = hex::encode(Sha256::digest(key.as_bytes()));
        let v = ApiKeyValidator::new(vec![ApiKey { name: "claude-skill".into(), hash }]);
        assert_eq!(v.validate(key), Some("claude-skill"));
    }

    #[test]
    fn wrong_token_rejected() {
        let v = ApiKeyValidator::new(vec![ApiKey { name: "x".into(), hash: "deadbeef".into() }]);
        assert!(v.validate("nope").is_none());
    }
}
```

- [ ] **Step 2: OIDC validator (jsonwebtoken + cached JWKS)**

```rust
// backend/servers/deploy-server/src/auth/oidc.rs
use crate::config::OidcConfig;
use crate::Result;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct OidcValidator {
    cfg: OidcConfig,
    jwks: Arc<RwLock<Option<Jwks>>>,
}

#[derive(Clone, Deserialize)]
struct Jwks { keys: Vec<JwkKey> }

#[derive(Clone, Deserialize)]
struct JwkKey {
    kid: String,
    n: String,
    e: String,
    #[allow(dead_code)]
    alg: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GhOidcClaims {
    pub sub: String,
    pub aud: String,
    pub repository: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
}

impl OidcValidator {
    pub fn new(cfg: OidcConfig) -> Self {
        Self { cfg, jwks: Arc::new(RwLock::new(None)) }
    }

    async fn fetch_jwks(&self) -> Result<Jwks> {
        let resp = reqwest::get(&self.cfg.jwks_url).await?;
        let jwks: Jwks = resp.json().await?;
        Ok(jwks)
    }

    async fn key_for(&self, kid: &str) -> Result<DecodingKey> {
        {
            let g = self.jwks.read().await;
            if let Some(j) = g.as_ref() {
                if let Some(k) = j.keys.iter().find(|k| k.kid == kid) {
                    return DecodingKey::from_rsa_components(&k.n, &k.e)
                        .map_err(|e| crate::DeployError::Unauthorized(format!("jwk decode: {e}")));
                }
            }
        }
        // Cache miss → refresh
        let fresh = self.fetch_jwks().await?;
        let key = fresh.keys.iter().find(|k| k.kid == kid)
            .ok_or_else(|| crate::DeployError::Unauthorized(format!("unknown kid {kid}")))
            .and_then(|k| DecodingKey::from_rsa_components(&k.n, &k.e)
                .map_err(|e| crate::DeployError::Unauthorized(format!("jwk decode: {e}"))))?;
        *self.jwks.write().await = Some(fresh);
        Ok(key)
    }

    pub async fn validate(&self, token: &str) -> Result<GhOidcClaims> {
        let header = decode_header(token)
            .map_err(|e| crate::DeployError::Unauthorized(format!("bad header: {e}")))?;
        let kid = header.kid.ok_or_else(|| crate::DeployError::Unauthorized("missing kid".into()))?;
        let key = self.key_for(&kid).await?;

        let mut val = Validation::new(Algorithm::RS256);
        val.set_audience(&[&self.cfg.audience]);
        val.set_issuer(&[&self.cfg.issuer]);

        let data = decode::<GhOidcClaims>(token, &key, &val)
            .map_err(|e| crate::DeployError::Unauthorized(format!("jwt verify: {e}")))?;
        let claims = data.claims;

        if !self.cfg.allowed_repos.iter().any(|r| r == &claims.repository) {
            return Err(crate::DeployError::Forbidden(format!("repo {} not allowed", claims.repository)));
        }
        if !self.cfg.allowed_refs.iter().any(|p| ref_matches(p, &claims.git_ref)) {
            return Err(crate::DeployError::Forbidden(format!("ref {} not allowed", claims.git_ref)));
        }
        Ok(claims)
    }
}

fn ref_matches(pattern: &str, candidate: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix('*') {
        candidate.starts_with(prefix)
    } else {
        pattern == candidate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ref_pattern_match() {
        assert!(ref_matches("refs/heads/feature/*", "refs/heads/feature/foo"));
        assert!(ref_matches("refs/heads/main", "refs/heads/main"));
        assert!(!ref_matches("refs/heads/main", "refs/heads/dev"));
    }
}
```

- [ ] **Step 3: Wire mod.rs**

```rust
// backend/servers/deploy-server/src/auth/mod.rs
pub mod api_key;
pub mod oidc;

pub use api_key::ApiKeyValidator;
pub use oidc::{GhOidcClaims, OidcValidator};
```

- [ ] **Step 4: Run tests**

```bash
cd backend && cargo test -p deploy-server auth
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add backend/servers/deploy-server/src/auth/
git commit -m "feat(deploy-server): API key + GH OIDC validators"
```

---

### Task P1.10: Auth + audit middleware

**Files:**
- Create: `backend/servers/deploy-server/src/infra/audit.rs`
- Modify: `backend/servers/deploy-server/src/infra/mod.rs`

- [ ] **Step 1: Write middleware**

```rust
// backend/servers/deploy-server/src/infra/audit.rs
use crate::auth::{ApiKeyValidator, OidcValidator};
use crate::infra::Store;
use crate::DeployError;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct AuthState {
    pub api_keys: Arc<ApiKeyValidator>,
    pub oidc: Arc<OidcValidator>,
    pub store: Arc<Store>,
}

#[derive(Clone, Debug)]
pub struct CallerIdentity {
    pub kind: String,
    pub id: String,
}

pub async fn auth_and_audit(
    State(state): State<AuthState>,
    mut req: Request,
    next: Next,
) -> Response {
    let started = Instant::now();
    let endpoint = format!("{} {}", req.method(), req.uri().path());

    // Skip auth for /health
    if req.uri().path() == "/health" {
        let resp = next.run(req).await;
        return resp;
    }

    let token = match extract_bearer(&req) {
        Some(t) => t,
        None => {
            let _ = state.store.record_audit(
                "unauth", "-", &endpoint, None, "error:missing_bearer", started.elapsed().as_millis() as i64,
            ).await;
            return error_resp(StatusCode::UNAUTHORIZED, "missing bearer");
        }
    };

    // Try API key first (fast), then OIDC (slower).
    let identity = if let Some(name) = state.api_keys.validate(&token) {
        CallerIdentity { kind: "api_key".into(), id: name.into() }
    } else {
        match state.oidc.validate(&token).await {
            Ok(claims) => CallerIdentity {
                kind: "oidc".into(),
                id: format!("{}@{}", claims.repository, claims.git_ref),
            },
            Err(e) => {
                let _ = state.store.record_audit(
                    "unauth", "-", &endpoint, None, &format!("error:{e}"), started.elapsed().as_millis() as i64,
                ).await;
                return error_resp(StatusCode::UNAUTHORIZED, &format!("auth failed: {e}"));
            }
        }
    };

    req.extensions_mut().insert(identity.clone());

    let resp = next.run(req).await;
    let status = resp.status();
    let result = if status.is_success() { "ok".to_string() } else { format!("error:{}", status.as_u16()) };
    let _ = state.store.record_audit(
        &identity.kind, &identity.id, &endpoint, None, &result, started.elapsed().as_millis() as i64,
    ).await;
    resp
}

fn extract_bearer(req: &Request) -> Option<String> {
    let val: &HeaderValue = req.headers().get(header::AUTHORIZATION)?;
    let s = val.to_str().ok()?;
    s.strip_prefix("Bearer ").map(str::to_string)
}

fn error_resp(status: StatusCode, msg: &str) -> Response {
    let body = serde_json::to_vec(&serde_json::json!({"error": msg})).unwrap();
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap()
}
```

- [ ] **Step 2: Wire mod.rs**

```rust
// backend/servers/deploy-server/src/infra/mod.rs
pub mod audit;
pub mod caddy;
pub mod docker;
pub mod git;
pub mod store;

pub use audit::{auth_and_audit, AuthState, CallerIdentity};
pub use caddy::CaddyClient;
pub use docker::{DockerClient, FrontendDevSpec};
pub use git::GitFetcher;
pub use store::Store;
```

- [ ] **Step 3: Compile-check**

```bash
cd backend && cargo check -p deploy-server
```

- [ ] **Step 4: Commit**

```bash
git add backend/servers/deploy-server/src/infra/
git commit -m "feat(deploy-server): auth + audit middleware"
```

---

### Task P1.11: API handler — health

**Files:**
- Create: `backend/servers/deploy-server/src/api/health.rs`
- Modify: `backend/servers/deploy-server/src/api/mod.rs`

- [ ] **Step 1: Handler + test**

```rust
// backend/servers/deploy-server/src/api/health.rs
use axum::Json;
use serde_json::json;

pub async fn handler() -> Json<serde_json::Value> {
    Json(json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_returns_ok() {
        let Json(body) = handler().await;
        assert_eq!(body["status"], "ok");
    }
}
```

- [ ] **Step 2: Run test**

```bash
cd backend && cargo test -p deploy-server api::health
```

- [ ] **Step 3: Commit**

```bash
git add backend/servers/deploy-server/src/api/
git commit -m "feat(deploy-server): /health handler"
```

---

### Task P1.12: API handler — POST /api/worktree (open) + state machine

**Files:**
- Create: `backend/servers/deploy-server/src/api/worktree.rs`
- Modify: `backend/servers/deploy-server/src/api/mod.rs`

- [ ] **Step 1: Handler with logical orchestration**

```rust
// backend/servers/deploy-server/src/api/worktree.rs
use crate::domain::{BackendMode, Worktree, WorktreeState, WorktreeUrls};
use crate::infra::{CaddyClient, CallerIdentity, DockerClient, FrontendDevSpec, GitFetcher, Store};
use crate::infra::git::sanitize;
use crate::{DeployError, Result};
use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct WorktreeService {
    pub store: Arc<Store>,
    pub git: Arc<GitFetcher>,
    pub docker: Arc<DockerClient>,
    pub caddy: Arc<CaddyClient>,
    pub frontend_image: String,
    pub domain_dev_ppt: String,            // "dev.ppt.rlt.sk"
    pub domain_dev_reality: String,        // "dev.rlt.sk"
}

#[derive(Debug, Deserialize)]
pub struct OpenRequest {
    pub branch: String,
    pub alias: Option<String>,
    #[serde(default = "default_backend")]
    pub backend: BackendMode,
    pub ttl_seconds: Option<i64>,
}

fn default_backend() -> BackendMode { BackendMode::Shared }

#[derive(Debug, Serialize)]
pub struct OpenResponse {
    pub worktree: Worktree,
    pub backend_status: String,           // "ready" | "building"
}

pub async fn open_handler(
    State(svc): State<Arc<WorktreeService>>,
    axum::Extension(caller): axum::Extension<CallerIdentity>,
    Json(req): Json<OpenRequest>,
) -> Result<Json<OpenResponse>> {
    let name = req.alias.clone().unwrap_or_else(|| sanitize(&req.branch));
    if name.is_empty() {
        return Err(DeployError::BadRequest("alias resolves to empty name".into()));
    }

    if matches!(req.backend, BackendMode::Dedicated) {
        return Err(DeployError::BadRequest(
            "dedicated backend mode is implemented in Phase 3".into(),
        ));
    }

    // 1. Fetch source.
    let source_path = svc.git.fetch_branch(&req.branch).await?;

    // 2. Allocate ports (simple deterministic hash of name → 51000–51999 range).
    let port_ppt = pick_port(&format!("{name}-ppt"));
    let port_reality = pick_port(&format!("{name}-reality"));

    // 3. Spawn frontend dev containers.
    let pnpm_volume = format!("ppt-deploy-pnpm-{name}");
    let ppt_container = format!("wt-{name}-ppt");
    let reality_container = format!("wt-{name}-reality");

    svc.docker.run_frontend_dev(&FrontendDevSpec {
        container_name: ppt_container.clone(),
        app: "ppt-web".into(),
        source_path: source_path.to_string_lossy().to_string(),
        host_port: port_ppt,
        pnpm_volume: pnpm_volume.clone(),
        image: svc.frontend_image.clone(),
    }).await?;
    svc.docker.run_frontend_dev(&FrontendDevSpec {
        container_name: reality_container.clone(),
        app: "reality-web".into(),
        source_path: source_path.to_string_lossy().to_string(),
        host_port: port_reality,
        pnpm_volume: pnpm_volume.clone(),
        image: svc.frontend_image.clone(),
    }).await?;

    // 4. Register Caddy routes.
    let host_ppt = format!("wt-{name}.{}", svc.domain_dev_ppt);
    let host_reality = format!("wt-{name}.{}", svc.domain_dev_reality);
    svc.caddy.register_route(&host_ppt, &format!("127.0.0.1:{port_ppt}")).await?;
    svc.caddy.register_route(&host_reality, &format!("127.0.0.1:{port_reality}")).await?;

    // 5. Persist state.
    let now = chrono::Utc::now();
    let wt = Worktree {
        name: name.clone(),
        branch: req.branch.clone(),
        backend_mode: req.backend.clone(),
        state: WorktreeState::Running,
        urls: WorktreeUrls {
            ppt: Some(format!("https://{host_ppt}")),
            reality: Some(format!("https://{host_reality}")),
            api: None,
        },
        containers: vec![ppt_container, reality_container],
        db_name: None,
        dump_path: None,
        ttl_seconds: req.ttl_seconds.unwrap_or(172_800),
        last_traffic_at: Some(now),
        closed_at: None,
        created_at: now,
        created_by: format!("{}:{}", caller.kind, caller.id),
    };
    svc.store.upsert_worktree(&wt).await?;

    Ok(Json(OpenResponse { worktree: wt, backend_status: "ready".into() }))
}

pub async fn get_handler(
    State(svc): State<Arc<WorktreeService>>,
    Path(name): Path<String>,
) -> Result<Json<Worktree>> {
    let wt = svc.store.get_worktree(&name).await?
        .ok_or_else(|| DeployError::NotFound(format!("worktree {name}")))?;
    Ok(Json(wt))
}

pub async fn list_handler(
    State(svc): State<Arc<WorktreeService>>,
) -> Result<Json<Vec<Worktree>>> {
    Ok(Json(svc.store.list_worktrees().await?))
}

pub async fn close_handler(
    State(svc): State<Arc<WorktreeService>>,
    Path(name): Path<String>,
) -> Result<Json<Worktree>> {
    let mut wt = svc.store.get_worktree(&name).await?
        .ok_or_else(|| DeployError::NotFound(format!("worktree {name}")))?;

    // Stop containers, ignore individual errors (best-effort cleanup).
    for c in &wt.containers {
        let _ = svc.docker.stop_container(c).await;
    }

    // Unregister Caddy routes.
    if let Some(host) = wt.urls.ppt.as_deref().and_then(|u| u.strip_prefix("https://")) {
        let _ = svc.caddy.unregister_route(host).await;
    }
    if let Some(host) = wt.urls.reality.as_deref().and_then(|u| u.strip_prefix("https://")) {
        let _ = svc.caddy.unregister_route(host).await;
    }

    wt.state = WorktreeState::Closed;
    wt.closed_at = Some(chrono::Utc::now());
    svc.store.upsert_worktree(&wt).await?;
    Ok(Json(wt))
}

fn pick_port(seed: &str) -> u16 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    seed.hash(&mut h);
    51000 + (h.finish() % 1000) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_port_deterministic_and_in_range() {
        let a = pick_port("foo");
        let b = pick_port("foo");
        assert_eq!(a, b);
        assert!((51000..52000).contains(&a));
    }
}
```

- [ ] **Step 2: Wire api/mod.rs**

```rust
// backend/servers/deploy-server/src/api/mod.rs
pub mod health;
pub mod router;
pub mod worktree;
```

- [ ] **Step 3: Run unit tests**

```bash
cd backend && cargo test -p deploy-server api::worktree
```

- [ ] **Step 4: Commit**

```bash
git add backend/servers/deploy-server/src/api/
git commit -m "feat(deploy-server): worktree open/get/list/close handlers"
```

---

### Task P1.13: API router + main entry with systemd socket activation

**Files:**
- Create: `backend/servers/deploy-server/src/api/router.rs`
- Modify: `backend/servers/deploy-server/src/main.rs`

- [ ] **Step 1: Router**

```rust
// backend/servers/deploy-server/src/api/router.rs
use crate::api::{health, worktree};
use crate::auth::{ApiKeyValidator, OidcValidator};
use crate::infra::{audit, AuthState, CaddyClient, DockerClient, GitFetcher, Store};
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;

pub fn build(
    store: Arc<Store>,
    git: Arc<GitFetcher>,
    docker: Arc<DockerClient>,
    caddy: Arc<CaddyClient>,
    api_keys: Arc<ApiKeyValidator>,
    oidc: Arc<OidcValidator>,
    frontend_image: String,
    domain_dev_ppt: String,
    domain_dev_reality: String,
) -> Router {
    let svc = Arc::new(worktree::WorktreeService {
        store: store.clone(),
        git, docker, caddy,
        frontend_image,
        domain_dev_ppt,
        domain_dev_reality,
    });

    let auth_state = AuthState {
        api_keys,
        oidc,
        store: store.clone(),
    };

    Router::new()
        .route("/api/worktree", post(worktree::open_handler))
        .route("/api/worktrees", get(worktree::list_handler))
        .route("/api/worktree/:name", get(worktree::get_handler))
        .route("/api/worktree/:name/close", post(worktree::close_handler))
        .with_state(svc)
        .layer(from_fn_with_state(auth_state, audit::auth_and_audit))
        .route("/health", get(health::handler))
}
```

- [ ] **Step 2: main.rs**

```rust
// backend/servers/deploy-server/src/main.rs
use anyhow::Context;
use deploy_server::api::router;
use deploy_server::auth::{ApiKeyValidator, OidcValidator};
use deploy_server::config::{load_yaml, AuthConfig, Config, TargetsConfig};
use deploy_server::infra::{CaddyClient, DockerClient, GitFetcher, Store};
use listenfd::ListenFd;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    let etc = PathBuf::from(std::env::var("PPT_DEPLOY_ETC").unwrap_or_else(|_| "/etc/ppt-deploy".into()));
    let cfg: Config = load_yaml(&etc.join("config.yaml")).context("load config.yaml")?;
    let targets: TargetsConfig = load_yaml(&etc.join("targets.yaml")).context("load targets.yaml")?;
    let auth: AuthConfig = load_yaml(&etc.join("auth.yaml")).context("load auth.yaml")?;

    let staging = targets.targets.get("staging").context("targets.staging missing")?;

    let store = Arc::new(Store::open(&PathBuf::from(&cfg.state_dir).join("state.db")).await?);
    let git = Arc::new(GitFetcher::new(
        &cfg.git_repo_url,
        &cfg.worktree_dir,
        &auth.gh_deploy_key_path,
    ));
    let docker = Arc::new(DockerClient::from_socket(&staging.docker_socket)?);
    let caddy = Arc::new(CaddyClient::new(&staging.caddy_url));
    let api_keys = Arc::new(ApiKeyValidator::new(auth.api_keys.clone()));
    let oidc = Arc::new(OidcValidator::new(auth.oidc.clone()));

    let app = router::build(
        store, git, docker, caddy, api_keys, oidc,
        std::env::var("PPT_FRONTEND_IMAGE").unwrap_or_else(|_| "ppt-frontend-dev:local".into()),
        format!("dev.ppt.{}", staging.domain_suffix.trim_start_matches("staging.")),
        format!("dev.{}", staging.domain_suffix.trim_start_matches("staging.")),
    );

    let mut fd = ListenFd::from_env();
    let listener = if let Some(l) = fd.take_tcp_listener(0)? {
        l.set_nonblocking(true)?;
        TcpListener::from_std(l)?
    } else {
        TcpListener::bind(&cfg.bind).await?
    };
    tracing::info!(bind = %listener.local_addr()?, "ppt-deploy listening");
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 3: Compile**

```bash
cd backend && cargo build -p deploy-server --bin ppt-deploy
```

- [ ] **Step 4: Commit**

```bash
git add backend/servers/deploy-server/src/api/router.rs backend/servers/deploy-server/src/main.rs
git commit -m "feat(deploy-server): axum router + systemd socket activation"
```

---

### Task P1.14: GH webhook handler

**Files:**
- Create: `backend/servers/deploy-server/src/api/webhook.rs`
- Modify: `backend/servers/deploy-server/src/api/router.rs`
- Modify: `backend/servers/deploy-server/src/api/mod.rs`

- [ ] **Step 1: Handler with HMAC verification**

```rust
// backend/servers/deploy-server/src/api/webhook.rs
use crate::api::worktree::WorktreeService;
use crate::infra::git::sanitize;
use crate::{DeployError, Result};
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct WebhookConfig {
    pub secret: String,
}

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub action: Option<String>,
    pub pull_request: Option<PullRequest>,
}

#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub head: PrRef,
}

#[derive(Debug, Deserialize)]
pub struct PrRef {
    #[serde(rename = "ref")]
    pub git_ref: String,
}

pub async fn handler(
    State((svc, cfg)): State<(Arc<WorktreeService>, WebhookConfig)>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>> {
    verify_signature(&headers, &body, &cfg.secret)?;
    let payload: WebhookPayload = serde_json::from_slice(&body)
        .map_err(|e| DeployError::BadRequest(format!("bad json: {e}")))?;

    if payload.action.as_deref() == Some("closed") {
        if let Some(pr) = &payload.pull_request {
            let name = sanitize(&pr.head.git_ref);
            // Best-effort close; ignore not-found.
            if svc.store.get_worktree(&name).await?.is_some() {
                let path = axum::extract::Path(name.clone());
                let _ = crate::api::worktree::close_handler(State(svc.clone()), path).await;
            }
        }
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

fn verify_signature(headers: &HeaderMap, body: &[u8], secret: &str) -> Result<()> {
    let sig = headers.get("X-Hub-Signature-256")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("sha256="))
        .ok_or_else(|| DeployError::Unauthorized("missing X-Hub-Signature-256".into()))?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| DeployError::Internal(format!("hmac key: {e}")))?;
    mac.update(body);
    let expected = hex::encode(mac.finalize().into_bytes());
    if !constant_time_eq(sig.as_bytes(), expected.as_bytes()) {
        return Err(DeployError::Unauthorized("bad signature".into()));
    }
    Ok(())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) { diff |= x ^ y; }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_round_trip() {
        let secret = "topsecret";
        let body = b"hello";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let sig = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        let mut headers = HeaderMap::new();
        headers.insert("X-Hub-Signature-256", sig.parse().unwrap());

        verify_signature(&headers, body, secret).unwrap();
    }

    #[test]
    fn bad_signature_rejected() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Hub-Signature-256", "sha256=00".parse().unwrap());
        assert!(verify_signature(&headers, b"x", "k").is_err());
    }
}
```

- [ ] **Step 2: Wire api/mod.rs**

```rust
// backend/servers/deploy-server/src/api/mod.rs
pub mod health;
pub mod router;
pub mod webhook;
pub mod worktree;
```

- [ ] **Step 3: Wire router** (extend `build` to accept webhook config and add the route bypassing the bearer middleware — webhook auth is HMAC, not Bearer)

Edit `backend/servers/deploy-server/src/api/router.rs`:

Add new param `webhook_cfg: webhook::WebhookConfig` and a new route `/api/webhook/github` that uses its own state tuple:

```rust
// inside build(), after the existing routes:
        .merge(
            Router::new()
                .route("/api/webhook/github", post(webhook::handler))
                .with_state((svc.clone(), webhook_cfg))
        )
```

Update main.rs to construct `WebhookConfig` from `auth.webhook_secret` and pass it.

- [ ] **Step 4: Run tests**

```bash
cd backend && cargo test -p deploy-server api::webhook
```

- [ ] **Step 5: Commit**

```bash
git add backend/servers/deploy-server/src/api/
git commit -m "feat(deploy-server): GH webhook handler with HMAC verification"
```

---

### Task P1.15: GC tick (idle pause / stop / TTL cleanup)

**Files:**
- Create: `backend/servers/deploy-server/src/api/gc.rs`
- Modify: `backend/servers/deploy-server/src/api/router.rs`
- Modify: `backend/servers/deploy-server/src/api/mod.rs`

- [ ] **Step 1: Handler logic**

```rust
// backend/servers/deploy-server/src/api/gc.rs
use crate::api::worktree::WorktreeService;
use crate::config::Config;
use crate::domain::WorktreeState;
use crate::Result;
use axum::extract::State;
use axum::Json;
use chrono::Utc;
use std::sync::Arc;

#[derive(Clone)]
pub struct GcContext {
    pub svc: Arc<WorktreeService>,
    pub cfg: Arc<Config>,
}

#[derive(serde::Serialize)]
pub struct GcReport {
    pub paused: Vec<String>,
    pub stopped: Vec<String>,
    pub cleaned: Vec<String>,
}

pub async fn tick_handler(
    State(ctx): State<GcContext>,
) -> Result<Json<GcReport>> {
    let now = Utc::now();
    let pause_after = chrono::Duration::seconds(ctx.cfg.idle_pause_seconds);
    let stop_after  = chrono::Duration::seconds(ctx.cfg.idle_stop_seconds);

    let mut report = GcReport { paused: vec![], stopped: vec![], cleaned: vec![] };
    let worktrees = ctx.svc.store.list_worktrees().await?;
    for mut wt in worktrees {
        match wt.state {
            WorktreeState::Running => {
                if let Some(last) = wt.last_traffic_at {
                    if now - last > pause_after {
                        for c in &wt.containers { let _ = ctx.svc.docker.stop_container(c).await; }
                        wt.state = WorktreeState::Paused;
                        ctx.svc.store.upsert_worktree(&wt).await?;
                        report.paused.push(wt.name.clone());
                    }
                }
            }
            WorktreeState::Paused => {
                if let Some(last) = wt.last_traffic_at {
                    if now - last > stop_after {
                        for c in &wt.containers { let _ = ctx.svc.docker.remove_container(c).await; }
                        // Phase 1 has shared backend → no DB to dump. Phase 3 will add pg_dump here.
                        wt.state = WorktreeState::Closed;
                        wt.closed_at = Some(now);
                        ctx.svc.store.upsert_worktree(&wt).await?;
                        report.stopped.push(wt.name.clone());
                    }
                }
            }
            WorktreeState::Closed => {
                if let Some(closed_at) = wt.closed_at {
                    if (now - closed_at).num_seconds() > wt.ttl_seconds {
                        // Remove worktree dir
                        let dir = std::path::PathBuf::from(&ctx.cfg.worktree_dir)
                            .join(crate::infra::git::sanitize(&wt.branch));
                        let _ = tokio::fs::remove_dir_all(&dir).await;
                        // Remove sqlite row by upserting nothing → not provided; leave row but mark cleaned via ttl=0.
                        // Future task: add Store::delete_worktree.
                        report.cleaned.push(wt.name.clone());
                    }
                }
            }
            WorktreeState::Closing => {} // transient, leave for next tick
        }
    }
    Ok(Json(report))
}
```

> **NOTE for engineer:** The "Future task: add Store::delete_worktree" line is not a placeholder for THIS plan — it is acknowledged as Phase 3 work. For Phase 1, keeping the closed row with `worktree dir removed` is acceptable; the row carries history value.

- [ ] **Step 2: Wire api/mod.rs**

```rust
// backend/servers/deploy-server/src/api/mod.rs
pub mod gc;
pub mod health;
pub mod router;
pub mod webhook;
pub mod worktree;
```

- [ ] **Step 3: Add route in router** (gated by API key only — cron uses local API key)

In `router.rs`, in the auth-protected branch:

```rust
        .route("/api/gc/tick", post(gc::tick_handler))
```

…but the `gc::tick_handler` requires `State(GcContext)` while siblings use `State(Arc<WorktreeService>)`. Easiest path: build a parallel `Router::new().route(...).with_state(gc_ctx)` and `.merge` it into the auth-layered router. Update main.rs to pass `cfg: Arc<Config>` into `router::build`.

- [ ] **Step 4: Compile + run all unit tests**

```bash
cd backend && cargo test -p deploy-server
```

- [ ] **Step 5: Commit**

```bash
git add backend/servers/deploy-server/src/api/ backend/servers/deploy-server/src/main.rs
git commit -m "feat(deploy-server): GC tick endpoint for idle pause/stop/cleanup"
```

---

### Task P1.16: systemd units

**Files:**
- Create: `backend/servers/deploy-server/systemd/ppt-deploy.socket`
- Create: `backend/servers/deploy-server/systemd/ppt-deploy.service`
- Create: `backend/servers/deploy-server/systemd/ppt-deploy-gc.service`
- Create: `backend/servers/deploy-server/systemd/ppt-deploy-gc.timer`

- [ ] **Step 1: Socket unit**

```ini
# backend/servers/deploy-server/systemd/ppt-deploy.socket
[Unit]
Description=ppt-deploy listening socket

[Socket]
ListenStream=127.0.0.1:8443
NoDelay=true
Accept=false

[Install]
WantedBy=sockets.target
```

> Caddy on the same host terminates TLS for `deploy.rlt.sk` and reverse-proxies to `127.0.0.1:8443`. The deploy server itself does NOT terminate TLS — Caddy does.

- [ ] **Step 2: Service unit**

```ini
# backend/servers/deploy-server/systemd/ppt-deploy.service
[Unit]
Description=ppt-deploy on-demand server
Requires=ppt-deploy.socket
After=network.target docker.service

[Service]
Type=notify
ExecStart=/usr/local/bin/ppt-deploy
User=ppt-deploy
Group=ppt-deploy
Environment=PPT_DEPLOY_ETC=/etc/ppt-deploy
Environment=RUST_LOG=info
Environment=LISTEN_FDS=1
StandardOutput=journal
StandardError=journal
TimeoutStopSec=30s
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

> **NOTE:** `Type=notify` requires the binary to call `sd_notify("READY=1")`. Listenfd alone does NOT — for MVP use `Type=simple` and ignore the notify protocol; switch to `notify` later when we add the `sd-notify` crate.

Replace `Type=notify` with `Type=simple` for MVP:

```ini
Type=simple
```

- [ ] **Step 3: GC timer + service**

```ini
# backend/servers/deploy-server/systemd/ppt-deploy-gc.timer
[Unit]
Description=ppt-deploy GC tick

[Timer]
OnBootSec=2min
OnUnitActiveSec=5min
Unit=ppt-deploy-gc.service

[Install]
WantedBy=timers.target
```

```ini
# backend/servers/deploy-server/systemd/ppt-deploy-gc.service
[Unit]
Description=ppt-deploy GC tick (one-shot)

[Service]
Type=oneshot
ExecStart=/usr/bin/curl -s -X POST -H "Authorization: Bearer ${PMCTL_TOKEN}" http://127.0.0.1:8443/api/gc/tick
EnvironmentFile=/etc/ppt-deploy/gc.env
```

`/etc/ppt-deploy/gc.env` (operator creates per runbook):
```
PMCTL_TOKEN=<api-key-for-cron>
```

- [ ] **Step 4: Commit**

```bash
git add backend/servers/deploy-server/systemd/
git commit -m "ops(deploy-server): systemd socket + service + GC timer units"
```

---

### Task P1.17: pmctl CLI binary

**Files:**
- Modify: `backend/servers/deploy-server/src/bin/pmctl.rs`

- [ ] **Step 1: CLI implementation**

```rust
// backend/servers/deploy-server/src/bin/pmctl.rs
use anyhow::Context;
use clap::{Parser, Subcommand};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "pmctl", version)]
struct Cli {
    /// Deploy server base URL (default: from $PPT_DEPLOY_URL or https://deploy.rlt.sk)
    #[arg(long, env = "PPT_DEPLOY_URL", default_value = "https://deploy.rlt.sk")]
    url: String,
    /// API token (default: from $PPT_DEPLOY_TOKEN or ~/.config/ppt-deploy/token)
    #[arg(long, env = "PPT_DEPLOY_TOKEN")]
    token: Option<String>,
    /// Output JSON instead of human-readable.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Open a worktree.
    Open {
        branch: String,
        #[arg(long)]
        alias: Option<String>,
        #[arg(long, default_value = "shared")]
        backend: String,
        #[arg(long)]
        ttl: Option<i64>,
    },
    /// Close a worktree (graceful).
    Close { name: String, #[arg(long)] hard: bool },
    /// Show worktree status.
    Status { name: Option<String> },
    /// List all worktrees.
    List,
    /// Print version of the server.
    Version,
}

#[derive(Serialize)]
struct OpenBody<'a> {
    branch: &'a str,
    alias: Option<&'a String>,
    backend: &'a str,
    ttl_seconds: Option<i64>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let token = match cli.token {
        Some(t) => t,
        None => std::fs::read_to_string(
            dirs::config_dir().unwrap().join("ppt-deploy/token")
        ).context("read ~/.config/ppt-deploy/token")?.trim().to_string(),
    };

    let http = reqwest::Client::new();
    let auth = format!("Bearer {token}");

    match cli.cmd {
        Cmd::Open { branch, alias, backend, ttl } => {
            let body = OpenBody {
                branch: &branch, alias: alias.as_ref(), backend: &backend, ttl_seconds: ttl,
            };
            let resp = http.post(format!("{}/api/worktree", cli.url))
                .header("Authorization", &auth)
                .json(&body).send().await?;
            print_resp(resp, cli.json).await?;
        }
        Cmd::Close { name, hard: _ } => {
            // Phase 1 ignores --hard (Phase 3 will add it).
            let resp = http.post(format!("{}/api/worktree/{name}/close", cli.url))
                .header("Authorization", &auth)
                .send().await?;
            print_resp(resp, cli.json).await?;
        }
        Cmd::Status { name } => {
            let url = match name {
                Some(n) => format!("{}/api/worktree/{n}", cli.url),
                None => format!("{}/api/worktrees", cli.url),
            };
            let resp = http.get(url).header("Authorization", &auth).send().await?;
            print_resp(resp, cli.json).await?;
        }
        Cmd::List => {
            let resp = http.get(format!("{}/api/worktrees", cli.url))
                .header("Authorization", &auth).send().await?;
            print_resp(resp, cli.json).await?;
        }
        Cmd::Version => {
            let resp = http.get(format!("{}/health", cli.url)).send().await?;
            print_resp(resp, cli.json).await?;
        }
    }
    Ok(())
}

async fn print_resp(resp: reqwest::Response, as_json: bool) -> anyhow::Result<()> {
    let status = resp.status();
    let text = resp.text().await?;
    if as_json {
        println!("{text}");
    } else if status.is_success() {
        // Pretty: parse JSON, render key fields.
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            println!("{}", serde_json::to_string_pretty(&v)?);
        } else {
            println!("{text}");
        }
    } else {
        anyhow::bail!("HTTP {}: {}", status, text);
    }
    Ok(())
}
```

- [ ] **Step 2: Add `dirs` crate to Cargo.toml**

Edit `backend/servers/deploy-server/Cargo.toml`:

```toml
dirs = "5.0"
```

- [ ] **Step 3: Build the binary**

```bash
cd backend && cargo build -p deploy-server --bin pmctl
```

- [ ] **Step 4: Smoke (against running server with PPT_DEPLOY_URL=http://127.0.0.1:8443)**

```bash
echo "test-token" > ~/.config/ppt-deploy/token
PPT_DEPLOY_URL=http://127.0.0.1:8443 ./target/debug/pmctl version --json
```

(Will fail if server isn't running; that's expected. The point is to confirm the binary runs and parses args.)

- [ ] **Step 5: Commit**

```bash
git add backend/servers/deploy-server/Cargo.toml backend/servers/deploy-server/src/bin/pmctl.rs
git commit -m "feat(pmctl): CLI for open/close/status/list/version"
```

---

### Task P1.18: Vite plugin — vite-plugin-ppt-worktree

**Files:**
- Create: `frontend/packages/vite-plugin-ppt-worktree/package.json`
- Create: `frontend/packages/vite-plugin-ppt-worktree/tsconfig.json`
- Create: `frontend/packages/vite-plugin-ppt-worktree/src/index.ts`
- Modify: `frontend/pnpm-workspace.yaml`

- [ ] **Step 1: package.json + tsconfig**

```json
// frontend/packages/vite-plugin-ppt-worktree/package.json
{
  "name": "@ppt/vite-plugin-worktree",
  "version": "0.0.1",
  "private": true,
  "type": "module",
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "peerDependencies": {
    "vite": "^5.0.0"
  },
  "devDependencies": {
    "vite": "^5.0.0",
    "typescript": "^5.4.0",
    "vitest": "^1.4.0"
  },
  "scripts": {
    "test": "vitest run"
  }
}
```

```json
// frontend/packages/vite-plugin-ppt-worktree/tsconfig.json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "types": ["node"]
  },
  "include": ["src/**/*"]
}
```

- [ ] **Step 2: Plugin source**

```typescript
// frontend/packages/vite-plugin-ppt-worktree/src/index.ts
import { execSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import type { Plugin } from 'vite';

export interface WorktreeInfo {
  name: string;       // sanitized branch name, e.g. "feature-uc-14"
  branch: string;     // raw branch, e.g. "feature/UC-14"
  isWorktree: boolean;
}

export function detectWorktree(cwd: string = process.cwd()): WorktreeInfo {
  try {
    const branch = execSync('git rev-parse --abbrev-ref HEAD', { cwd, encoding: 'utf8' }).trim();
    const sanitized = sanitize(branch);
    const gitCommonDir = execSync('git rev-parse --git-common-dir', { cwd, encoding: 'utf8' }).trim();
    const gitDir = execSync('git rev-parse --git-dir', { cwd, encoding: 'utf8' }).trim();
    const isWorktree = resolve(cwd, gitDir) !== resolve(cwd, gitCommonDir);
    return { name: sanitized, branch, isWorktree };
  } catch {
    return { name: 'unknown', branch: 'unknown', isWorktree: false };
  }
}

export function sanitize(branch: string): string {
  return branch
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

export interface PluginOptions {
  cwd?: string;
}

export default function pptWorktreePlugin(opts: PluginOptions = {}): Plugin {
  return {
    name: 'ppt-worktree',
    config() {
      const info = detectWorktree(opts.cwd);
      return {
        define: {
          __PPT_WORKTREE_NAME__: JSON.stringify(info.name),
          __PPT_WORKTREE_BRANCH__: JSON.stringify(info.branch),
          __PPT_IS_WORKTREE__: JSON.stringify(info.isWorktree),
        },
      };
    },
  };
}
```

- [ ] **Step 3: Add unit test**

```typescript
// frontend/packages/vite-plugin-ppt-worktree/src/index.test.ts
import { describe, expect, it } from 'vitest';
import { sanitize } from './index';

describe('sanitize', () => {
  it('lowercases and replaces non-alphanum', () => {
    expect(sanitize('feature/UC-14')).toBe('feature-uc-14');
    expect(sanitize('hotfix/Critical Fix')).toBe('hotfix-critical-fix');
  });
  it('strips leading/trailing dashes', () => {
    expect(sanitize('///---bad---///')).toBe('bad');
  });
});
```

- [ ] **Step 4: Add to workspace**

Edit `frontend/pnpm-workspace.yaml` if needed (already covers `packages/*`):

```yaml
packages:
  - "apps/*"
  - "packages/*"
```

- [ ] **Step 5: Install and test**

```bash
cd frontend && pnpm install
cd packages/vite-plugin-ppt-worktree && pnpm test
```

Expected: 2 tests pass.

- [ ] **Step 6: Commit**

```bash
git add frontend/packages/vite-plugin-ppt-worktree frontend/pnpm-workspace.yaml frontend/pnpm-lock.yaml
git commit -m "feat(frontend): vite-plugin-ppt-worktree injects __PPT_WORKTREE_NAME__"
```

---

### Task P1.19: Wire vite plugin into ppt-web and reality-web

**Files:**
- Modify: `frontend/apps/ppt-web/vite.config.ts`
- Modify: `frontend/apps/ppt-web/package.json`
- Modify: `frontend/apps/reality-web/next.config.ts` (if Next-based — for `reality-web` it's Next, plugin is for Vite only)
- Modify: `frontend/apps/reality-web/src/lib/worktree.ts` (helper for Next — read at build time via env)

- [ ] **Step 1: Add dep to ppt-web package.json**

```json
{
  "devDependencies": {
    "@ppt/vite-plugin-worktree": "workspace:*"
  }
}
```

- [ ] **Step 2: Update vite.config.ts**

```typescript
// frontend/apps/ppt-web/vite.config.ts (add to existing)
import pptWorktreePlugin from '@ppt/vite-plugin-worktree';
// ...
export default defineConfig({
  plugins: [
    react(),
    pptWorktreePlugin(),
    // ... existing plugins
  ],
  // ...
});
```

- [ ] **Step 3: For reality-web (Next.js), add a build-time injection**

Next doesn't have the same plugin model; use `next.config.ts` `env`:

```typescript
// frontend/apps/reality-web/next.config.ts
import { detectWorktree } from '@ppt/vite-plugin-worktree';

const info = detectWorktree();
const nextConfig = {
  env: {
    NEXT_PUBLIC_PPT_WORKTREE_NAME: info.name,
    NEXT_PUBLIC_PPT_WORKTREE_BRANCH: info.branch,
    NEXT_PUBLIC_PPT_IS_WORKTREE: String(info.isWorktree),
  },
  // ...existing config
};
export default nextConfig;
```

(The package re-exports `detectWorktree` so Next can call it directly during config.)

- [ ] **Step 4: Compile-check both apps**

```bash
cd frontend && pnpm install
pnpm --filter ./apps/ppt-web build
pnpm --filter ./apps/reality-web build
```

- [ ] **Step 5: Commit**

```bash
git add frontend/apps/ppt-web/vite.config.ts frontend/apps/ppt-web/package.json frontend/apps/reality-web/next.config.ts frontend/pnpm-lock.yaml
git commit -m "feat(frontend): wire vite-plugin-ppt-worktree into ppt-web + reality-web"
```

---

### Task P1.20: Dev panel React component

**Files:**
- Create: `frontend/packages/dev-panel/package.json`
- Create: `frontend/packages/dev-panel/tsconfig.json`
- Create: `frontend/packages/dev-panel/src/index.ts`
- Create: `frontend/packages/dev-panel/src/DevPanel.tsx`
- Create: `frontend/packages/dev-panel/src/store.ts`

- [ ] **Step 1: package.json + tsconfig**

```json
{
  "name": "@ppt/dev-panel",
  "version": "0.0.1",
  "private": true,
  "type": "module",
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "peerDependencies": {
    "react": "^18.0.0",
    "react-dom": "^18.0.0"
  },
  "devDependencies": {
    "@types/react": "^18.0.0",
    "react": "^18.0.0",
    "typescript": "^5.4.0",
    "vitest": "^1.4.0",
    "@testing-library/react": "^16.0.0"
  },
  "scripts": {
    "test": "vitest run"
  }
}
```

- [ ] **Step 2: store.ts — localStorage-backed mode**

```typescript
// frontend/packages/dev-panel/src/store.ts
export type ApiMode = 'local' | 'worktree' | 'mock';

const KEY = 'ppt-dev-panel-mode';

export function getMode(defaultMode: ApiMode): ApiMode {
  try {
    const v = localStorage.getItem(KEY);
    if (v === 'local' || v === 'worktree' || v === 'mock') return v;
  } catch {}
  return defaultMode;
}

export function setMode(mode: ApiMode): void {
  try {
    localStorage.setItem(KEY, mode);
  } catch {}
}

const SNAPSHOT_KEY = 'ppt-dev-panel-snapshot';
export function saveSnapshot(snapshot: unknown): void {
  try { localStorage.setItem(SNAPSHOT_KEY, JSON.stringify(snapshot)); } catch {}
}
export function loadSnapshot<T = unknown>(): T | null {
  try {
    const raw = localStorage.getItem(SNAPSHOT_KEY);
    return raw ? JSON.parse(raw) as T : null;
  } catch { return null; }
}
```

- [ ] **Step 3: DevPanel.tsx**

```tsx
// frontend/packages/dev-panel/src/DevPanel.tsx
import React, { useState } from 'react';
import { ApiMode, getMode, setMode } from './store';

export interface DevPanelProps {
  defaultMode: ApiMode;
  onModeChange: (mode: ApiMode) => void;
  onReseedMock?: () => void;
  onSnapshotState?: () => void;
}

export const DevPanel: React.FC<DevPanelProps> = ({ defaultMode, onModeChange, onReseedMock, onSnapshotState }) => {
  const [mode, setLocalMode] = useState<ApiMode>(() => getMode(defaultMode));
  const apply = (m: ApiMode) => {
    setMode(m);
    setLocalMode(m);
    onModeChange(m);
  };
  return (
    <div style={{
      position: 'fixed', bottom: 8, right: 8, zIndex: 99999,
      background: '#222', color: '#fff', padding: '8px 10px',
      borderRadius: 6, fontFamily: 'monospace', fontSize: 12,
      opacity: 0.85,
    }}>
      <div>API:&nbsp;
        <select value={mode} onChange={e => apply(e.target.value as ApiMode)}>
          <option value="local">local</option>
          <option value="worktree">worktree</option>
          <option value="mock">mock</option>
        </select>
      </div>
      {mode === 'mock' && onReseedMock && (
        <button onClick={onReseedMock} style={{ marginTop: 4 }}>Re-seed mock</button>
      )}
      {onSnapshotState && (
        <button onClick={onSnapshotState} style={{ marginTop: 4 }}>Snapshot state</button>
      )}
    </div>
  );
};
```

- [ ] **Step 4: index.ts re-exports**

```typescript
// frontend/packages/dev-panel/src/index.ts
export { DevPanel } from './DevPanel';
export { getMode, setMode, saveSnapshot, loadSnapshot } from './store';
export type { ApiMode } from './store';
```

- [ ] **Step 5: Test**

```typescript
// frontend/packages/dev-panel/src/store.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import { getMode, setMode } from './store';

describe('dev panel store', () => {
  beforeEach(() => localStorage.clear());

  it('returns default when nothing stored', () => {
    expect(getMode('worktree')).toBe('worktree');
  });

  it('persists across calls', () => {
    setMode('mock');
    expect(getMode('local')).toBe('mock');
  });

  it('ignores invalid stored values', () => {
    localStorage.setItem('ppt-dev-panel-mode', 'garbage');
    expect(getMode('local')).toBe('local');
  });
});
```

- [ ] **Step 6: Install + test**

```bash
cd frontend && pnpm install
pnpm --filter @ppt/dev-panel test
```

- [ ] **Step 7: Commit**

```bash
git add frontend/packages/dev-panel frontend/pnpm-lock.yaml
git commit -m "feat(frontend): @ppt/dev-panel React component + localStorage store"
```

---

### Task P1.21: MSW setup for ppt-web

**Files:**
- Create: `frontend/apps/ppt-web/src/mocks/browser.ts`
- Create: `frontend/apps/ppt-web/src/mocks/handlers/index.ts`
- Create: `frontend/apps/ppt-web/src/mocks/handlers/buildings.ts`
- Create: `frontend/apps/ppt-web/src/mocks/seeds/data.ts`
- Modify: `frontend/apps/ppt-web/src/main.tsx`
- Modify: `frontend/apps/ppt-web/package.json`

- [ ] **Step 1: Add MSW dep**

```json
{
  "devDependencies": {
    "msw": "^2.3.0"
  }
}
```

- [ ] **Step 2: Seeds**

```typescript
// frontend/apps/ppt-web/src/mocks/seeds/data.ts
export const seedBuildings = [
  { id: '33333333-3333-3333-3333-333333333333', name: 'Demo Building', address: 'Hlavná 1, Bratislava' },
];

export const seedFaults = [
  { id: 'f1', building_id: '33333333-3333-3333-3333-333333333333', title: 'Pokazený výťah', status: 'open' },
];
```

- [ ] **Step 3: Handlers**

```typescript
// frontend/apps/ppt-web/src/mocks/handlers/buildings.ts
import { http, HttpResponse } from 'msw';
import { seedBuildings } from '../seeds/data';

export const buildingsHandlers = [
  http.get('*/api/buildings', () => HttpResponse.json(seedBuildings)),
  http.get('*/api/buildings/:id', ({ params }) => {
    const b = seedBuildings.find(x => x.id === params.id);
    return b ? HttpResponse.json(b) : new HttpResponse(null, { status: 404 });
  }),
];
```

```typescript
// frontend/apps/ppt-web/src/mocks/handlers/index.ts
import { buildingsHandlers } from './buildings';

export const handlers = [
  ...buildingsHandlers,
];
```

- [ ] **Step 4: Browser worker**

```typescript
// frontend/apps/ppt-web/src/mocks/browser.ts
import { setupWorker } from 'msw/browser';
import { handlers } from './handlers';

export const worker = setupWorker(...handlers);
```

- [ ] **Step 5: Wire into main.tsx with mode switching**

```typescript
// frontend/apps/ppt-web/src/main.tsx (top of file additions)
import { DevPanel, getMode, type ApiMode } from '@ppt/dev-panel';
import { worker } from './mocks/browser';

const defaultMode: ApiMode = (import.meta.env.VITE_API_DEFAULT as ApiMode) || 'local';
const initialMode = getMode(defaultMode);

async function bootstrap() {
  if (import.meta.env.DEV && initialMode === 'mock') {
    await worker.start({ onUnhandledRequest: 'bypass' });
  }

  // ... existing render call
}

void bootstrap();
```

Add `<DevPanel>` at the root of the app (in `App.tsx` or wherever the root layout is) wrapped in `import.meta.env.DEV` check:

```tsx
{import.meta.env.DEV && (
  <DevPanel
    defaultMode={defaultMode}
    onModeChange={(m) => window.location.reload()}
  />
)}
```

- [ ] **Step 6: Generate MSW service worker file**

```bash
cd frontend/apps/ppt-web
pnpm exec msw init public/ --save
```

(creates `public/mockServiceWorker.js`)

- [ ] **Step 7: Add `@ppt/dev-panel` and `@ppt/vite-plugin-worktree` to ppt-web devDeps**

```json
{
  "devDependencies": {
    "@ppt/dev-panel": "workspace:*",
    "@ppt/vite-plugin-worktree": "workspace:*"
  }
}
```

- [ ] **Step 8: Build + smoke**

```bash
cd frontend && pnpm install
pnpm --filter @ppt/web build
pnpm --filter @ppt/web dev   # in another terminal
```

Open http://localhost:5173/, verify dev panel appears in bottom-right, switch to "mock" → reload → buildings list comes from MSW.

- [ ] **Step 9: Commit**

```bash
git add frontend/apps/ppt-web frontend/pnpm-lock.yaml
git commit -m "feat(ppt-web): MSW + dev panel integration"
```

---

### Task P1.22: MSW setup for reality-web

Mirror of P1.21 for reality-web. Skipped detailed code (engineer follows the same pattern with `listings.ts` handler instead of `buildings.ts`).

**Files:**
- Create: `frontend/apps/reality-web/src/mocks/browser.ts`
- Create: `frontend/apps/reality-web/src/mocks/handlers/index.ts`
- Create: `frontend/apps/reality-web/src/mocks/handlers/listings.ts`
- Create: `frontend/apps/reality-web/src/mocks/seeds/data.ts`
- Modify: `frontend/apps/reality-web/src/app/layout.tsx` (Next.js root layout)

- [ ] **Step 1: Mirror P1.21 structure**

Use the same file tree as ppt-web. The handler is for listings:

```typescript
// frontend/apps/reality-web/src/mocks/handlers/listings.ts
import { http, HttpResponse } from 'msw';
import { seedListings } from '../seeds/data';

export const listingsHandlers = [
  http.get('*/api/listings', () => HttpResponse.json(seedListings)),
];
```

```typescript
// frontend/apps/reality-web/src/mocks/seeds/data.ts
export const seedListings = [
  { id: 'l1', title: '2-izbový byt v centre', price_eur: 180000, city: 'Bratislava' },
];
```

- [ ] **Step 2: Next.js MSW init via `instrumentation.ts`**

```typescript
// frontend/apps/reality-web/src/instrumentation.ts
export async function register() {
  if (process.env.NEXT_PUBLIC_API_DEFAULT === 'mock' && typeof window !== 'undefined') {
    const { worker } = await import('./mocks/browser');
    await worker.start({ onUnhandledRequest: 'bypass' });
  }
}
```

- [ ] **Step 3: Add DevPanel mounted via root layout client-component wrapper**

Create `frontend/apps/reality-web/src/components/DevPanelMount.tsx`:

```tsx
'use client';
import { DevPanel, type ApiMode } from '@ppt/dev-panel';

export function DevPanelMount() {
  if (process.env.NODE_ENV !== 'development') return null;
  const defaultMode = (process.env.NEXT_PUBLIC_API_DEFAULT as ApiMode) || 'local';
  return (
    <DevPanel
      defaultMode={defaultMode}
      onModeChange={() => window.location.reload()}
    />
  );
}
```

Add `<DevPanelMount />` inside `app/layout.tsx`'s `<body>`.

- [ ] **Step 4: msw init for service worker**

```bash
cd frontend/apps/reality-web
pnpm exec msw init public/ --save
```

- [ ] **Step 5: Build + smoke**

```bash
cd frontend && pnpm --filter @ppt/reality-web dev
```

- [ ] **Step 6: Commit**

```bash
git add frontend/apps/reality-web frontend/pnpm-lock.yaml
git commit -m "feat(reality-web): MSW + dev panel integration"
```

---

### Task P1.23: Claude skill — SKILL.md, open-worktree.md, close-worktree.md

**Files:**
- Create: `.claude/skills/ppt-deploy/SKILL.md`
- Create: `.claude/skills/ppt-deploy/commands/open-worktree.md`
- Create: `.claude/skills/ppt-deploy/commands/close-worktree.md`
- Create: `.claude/skills/ppt-deploy/references/api.md`
- Create: `.claude/skills/ppt-deploy/references/modes.md`

- [ ] **Step 1: SKILL.md**

```markdown
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
```

- [ ] **Step 2: open-worktree.md**

```markdown
---
description: Spawn a worktree dev environment for the current branch (or specified branch) and configure the local frontend to talk to it.
---

# Open worktree

## Steps

1. Detect current branch (unless user specified one):
   ```bash
   BRANCH=$(git rev-parse --abbrev-ref HEAD)
   ```

2. Call pmctl:
   ```bash
   pmctl open "$BRANCH" --json
   ```
   Capture output JSON (contains `worktree.urls.ppt`, `worktree.urls.reality`, `worktree.name`).

3. Write `frontend/.env.local` (in the user's worktree, not server side):
   ```
   VITE_API_DEFAULT=worktree
   VITE_API_BASE=<urls.ppt>
   VITE_REALITY_API_BASE=<urls.reality>
   ```

4. Report to user:
   ```
   Worktree `<name>` ready:
   - ppt: <urls.ppt>
   - reality: <urls.reality>
   Backend: shared (default). Frontend mode set to `worktree` in .env.local.
   ```

## Notes

- If pmctl returns 409 conflict, the worktree already exists — call `pmctl status <name>` and report current URLs instead.
- For dedicated backend (Phase 3): pass `--backend=dedicated --as=<alias>`. Phase 1 returns 400 for this.
```

- [ ] **Step 3: close-worktree.md**

```markdown
---
description: Gracefully close a worktree dev environment, freeing resources. Caches snapshot for TTL window so reopening within 2 days is fast.
---

# Close worktree

## Steps

1. Detect current worktree name from branch:
   ```bash
   BRANCH=$(git rev-parse --abbrev-ref HEAD)
   NAME=$(echo "$BRANCH" | tr '/_ A-Z' '----a-z')
   # or use the alias the user chose at open time
   ```

2. Call pmctl:
   ```bash
   pmctl close "$NAME" --json
   ```

3. Remove the override line from `frontend/.env.local`:
   ```bash
   # Strip VITE_API_DEFAULT, VITE_API_BASE, VITE_REALITY_API_BASE if they reference this worktree.
   ```

4. Report:
   ```
   Worktree `<name>` closed. Snapshot will be GC'd after TTL (default 2 days).
   ```
```

- [ ] **Step 4: api.md, modes.md**

Reference files. `api.md` lists the HTTP endpoints (table from spec § 5). `modes.md` explains `local`/`worktree`/`mock`.

```markdown
# HTTP API reference (deploy server)

| Endpoint | Method | Body | Notes |
|---|---|---|---|
| `/api/worktree` | POST | `{branch, alias?, backend, ttl_seconds?}` | Open |
| `/api/worktrees` | GET | — | List |
| `/api/worktree/{name}` | GET | — | Status |
| `/api/worktree/{name}/close` | POST | — | Close |
| `/health` | GET | — | Public |

Auth: `Authorization: Bearer <token>`. Token from `~/.config/ppt-deploy/token` or `$PPT_DEPLOY_TOKEN`.
```

```markdown
# Frontend API modes

The dev panel offers three modes:

- **local** — Vite/Next dev server talks to `http://localhost:8080` (your local `cargo run -p api-server`).
- **worktree** — Talks to `https://wt-<alias>.dev.ppt.rlt.sk` (shared backend on Hetzner).
- **mock** — MSW intercepts every request, returns seeded data from `src/mocks/seeds/data.ts`.

Mode persists in `localStorage` (`ppt-dev-panel-mode`). The `.env.local`'s `VITE_API_DEFAULT` is the initial value; user override wins.
```

- [ ] **Step 5: Commit**

```bash
git add .claude/skills/ppt-deploy
git commit -m "feat(claude-skill): ppt-deploy skill with open/close commands"
```

---

### Task P1.24: End-to-end smoke test

A `tests/e2e/smoke.rs` that spins up the server with mocked dependencies and verifies the open → status → close flow.

**Files:**
- Create: `backend/servers/deploy-server/tests/smoke.rs`

- [ ] **Step 1: Write smoke test**

```rust
// backend/servers/deploy-server/tests/smoke.rs
//! End-to-end smoke: real sqlite, mocked Caddy + Docker.

use deploy_server::api::router;
use deploy_server::auth::{ApiKeyValidator, OidcValidator};
use deploy_server::config::{ApiKey, OidcConfig};
use deploy_server::infra::{CaddyClient, DockerClient, GitFetcher, Store};
use httpmock::prelude::*;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
#[ignore] // requires docker daemon
async fn open_status_close_flow() {
    let tmp = tempdir().unwrap();
    let store = Arc::new(Store::open(&tmp.path().join("state.db")).await.unwrap());

    let caddy_mock = MockServer::start();
    caddy_mock.mock(|when, then| { when.method(PUT); then.status(200); });
    caddy_mock.mock(|when, then| { when.method(DELETE); then.status(200); });

    // Local bare git repo as fixture (see infra::git tests for setup).
    let bare = tmp.path().join("origin.git");
    let work = tmp.path().join("seed");
    std::fs::create_dir_all(&bare).unwrap();
    std::process::Command::new("git").args(["init", "--bare"]).arg(&bare).status().unwrap();
    std::process::Command::new("git").args(["init"]).arg(&work).status().unwrap();
    std::process::Command::new("git").args(["-C", work.to_str().unwrap(), "config", "user.email", "t@t"]).status().unwrap();
    std::process::Command::new("git").args(["-C", work.to_str().unwrap(), "config", "user.name", "t"]).status().unwrap();
    std::fs::write(work.join("README.md"), "hi").unwrap();
    std::process::Command::new("git").args(["-C", work.to_str().unwrap(), "add", "."]).status().unwrap();
    std::process::Command::new("git").args(["-C", work.to_str().unwrap(), "commit", "-m", "init"]).status().unwrap();
    std::process::Command::new("git").args(["-C", work.to_str().unwrap(), "branch", "-M", "feature-x"]).status().unwrap();
    std::process::Command::new("git").args(["-C", work.to_str().unwrap(), "remote", "add", "origin"]).arg(&bare).status().unwrap();
    std::process::Command::new("git").args(["-C", work.to_str().unwrap(), "push", "-u", "origin", "feature-x"]).status().unwrap();

    let git = Arc::new(GitFetcher::new(
        bare.to_string_lossy().to_string(),
        tmp.path().join("worktrees"),
        "/dev/null",
    ));

    let docker = Arc::new(DockerClient::from_socket("unix:///var/run/docker.sock").unwrap());
    let caddy = Arc::new(CaddyClient::new(caddy_mock.base_url()));

    let api_key = "test-token";
    let hash = hex::encode(<sha2::Sha256 as sha2::Digest>::digest(api_key.as_bytes()));
    let api_keys = Arc::new(ApiKeyValidator::new(vec![ApiKey { name: "test".into(), hash }]));
    let oidc = Arc::new(OidcValidator::new(OidcConfig {
        issuer: "x".into(), jwks_url: "http://x".into(), audience: "x".into(),
        allowed_repos: vec![], allowed_refs: vec![],
    }));

    let app = router::build(
        store.clone(), git, docker, caddy, api_keys, oidc,
        "ppt-frontend-dev:local".into(),
        "dev.ppt.rlt.sk".into(), "dev.rlt.sk".into(),
    );

    let server = axum_test::TestServer::new(app).unwrap();
    let resp = server.post("/api/worktree")
        .add_header("Authorization", "Bearer test-token")
        .json(&serde_json::json!({"branch": "feature-x", "backend": "shared"}))
        .await;
    resp.assert_status_ok();

    let list = server.get("/api/worktrees")
        .add_header("Authorization", "Bearer test-token")
        .await;
    list.assert_status_ok();
    assert!(list.text().contains("feature-x"));

    let close = server.post("/api/worktree/feature-x/close")
        .add_header("Authorization", "Bearer test-token")
        .await;
    close.assert_status_ok();
}
```

- [ ] **Step 2: Add test deps**

`backend/servers/deploy-server/Cargo.toml` `[dev-dependencies]`:

```toml
axum-test = "15"
```

- [ ] **Step 3: Run with docker available**

```bash
cd backend && cargo test -p deploy-server --test smoke -- --ignored
```

Expected: pass on a workstation with docker daemon running and `ppt-frontend-dev:local` image present.

- [ ] **Step 4: Commit**

```bash
git add backend/servers/deploy-server/Cargo.toml backend/servers/deploy-server/tests/smoke.rs
git commit -m "test(deploy-server): e2e smoke for open→list→close flow"
```

---

### Task P1.25: Operator deployment runbook (close-the-loop)

Final operator-facing runbook tying P0.6 prereqs to actually starting Phase 1.

**Files:**
- Modify: `docs/runbooks/deploy-server-prereqs.md` (append)

- [ ] **Step 1: Append "Phase 1 deployment" section**

```markdown
## 11. Phase 1 deployment (one-time)

Build binary on Hetzner box (or pull from CI artifact in a future iteration):

```bash
git clone <repo> /opt/ppt-deploy-build
cd /opt/ppt-deploy-build/backend
cargo build --release -p deploy-server --bin ppt-deploy --bin pmctl
sudo install -m 0755 target/release/ppt-deploy /usr/local/bin/
sudo install -m 0755 target/release/pmctl       /usr/local/bin/
```

Configure:

```bash
sudo cp /opt/ppt-deploy-build/backend/servers/deploy-server/systemd/*.{socket,service,timer} /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now ppt-deploy.socket ppt-deploy-gc.timer
```

Generate API key:

```bash
TOKEN=$(openssl rand -hex 32)
HASH=$(printf "%s" "$TOKEN" | sha256sum | awk '{print $1}')
echo "API key (give to Claude skill): $TOKEN"
sudo tee -a /etc/ppt-deploy/auth.yaml <<EOF
api_keys:
  - name: claude-skill
    hash: "$HASH"
EOF
echo "$TOKEN" > "$HOME/.config/ppt-deploy/token"   # on operator's laptop
chmod 600 "$HOME/.config/ppt-deploy/token"
```

Test:

```bash
PPT_DEPLOY_URL=https://deploy.rlt.sk pmctl version
```

Expected: JSON `{"status":"ok","version":"..."}`.
```

- [ ] **Step 2: Commit**

```bash
git add docs/runbooks/deploy-server-prereqs.md
git commit -m "docs(runbook): Phase 1 deployment + API key bootstrap"
```

---

## Self-Review Notes

After implementing all tasks above, re-read the spec § 10 (Phasing) and verify Phase 0 + Phase 1 deliverables are covered:

| Spec deliverable (Phase 0/1) | Plan task |
|---|---|
| DNS wildcard records | P0.6 (manual) |
| Caddy custom build | P0.5 |
| Postgres template DB | P0.4 |
| Frontend dev container image | P0.3 |
| CI build optimization | P0.1, P0.2 |
| `backend/deploy-server/` Rust crate | P1.1 |
| `pmctl` second `bin` target | P1.1, P1.17 |
| HTTP API endpoints | P1.11–P1.15 |
| systemd socket activation | P1.13, P1.16 |
| Caddy admin API client | P1.6 |
| sqlite state | P1.5 |
| Audit log | P1.10 |
| GH OIDC + API key auth | P1.9, P1.10 |
| Frontend bind-mount via Docker SDK | P1.7 |
| `git fetch` shell-out | P1.8 |
| Vite plugin | P1.18, P1.19 |
| Frontend dev panel | P1.20, P1.21, P1.22 |
| MSW setup | P1.21, P1.22 |
| Claude skill | P1.23 |
| GH webhook receiver | P1.14 |
| Cron GC | P1.15, P1.16 |
| Smoke tests | P1.24 |

Phase 1 deliverables not in this plan (deferred to later phases per spec):
- Dedicated backend per worktree (Phase 3)
- pg_dump/restore in GC (Phase 3)
- `pmctl logs <name> -f` SSE stream (Phase 3)
- `pmctl psql <name>` (Phase 3)
- Staging auto-deploy from GHA (Phase 2)
- Tag-triggered prod release flow (Phase 4)

These deferrals are explicit in the spec § 10 and remain out of this plan's scope.
