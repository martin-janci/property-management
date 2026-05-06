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

The custom Caddy image is built by Phase 0 task P0.5 from `docker/caddy/Dockerfile` and published to GHCR.

```bash
docker pull ghcr.io/martin-janci/ppt-caddy:latest    # built by Phase 0 task P0.5
mkdir -p /etc/caddy /var/lib/caddy
cp deploy/caddy/Caddyfile.template /etc/caddy/Caddyfile
docker run -d --name caddy --restart=unless-stopped \
  -p 80:80 -p 443:443 -p 2019:2019 \
  -v /etc/caddy:/etc/caddy \
  -v /var/lib/caddy:/data \
  ghcr.io/martin-janci/ppt-caddy:latest
```

Verify wildcard cert provisioning by adding a temporary `https://test.dev.rlt.sk` site to the Caddyfile and observing Caddy logs (`docker logs caddy`) issuing a DNS-01 challenge.

## 4. Postgres template DB

```bash
docker exec -i ppt-postgres psql -U ppt -d postgres < backend/scripts/init-template-db.sql
```

> **Note:** The `init-template-db.sql` script creates the empty `ppt_dev_template` database, but the schema (tables, RLS policies, etc.) must be applied via SQLx migrations BEFORE the script's seed `DO` block (or any subsequent seed step) is re-run. After the initial `CREATE DATABASE`, apply migrations either by:
>
> - **Option A:** `cargo run -p api-server` — the binary will run pending migrations against the configured database on first start.
> - **Option B:** `sqlx migrate run --database-url postgres://ppt:<password>@localhost:5432/ppt_dev_template` — run from the workspace root (requires `sqlx-cli`).
>
> The script's header comment documents this two-step process; see `backend/scripts/init-template-db.sql` for details. Once migrations are applied, re-run the seed `DO` block (or full script) to populate fixtures.

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

> **Note:** The systemd unit files (`ppt-deploy.socket`, `ppt-deploy.service`, `ppt-deploy-gc.timer`) live under `backend/servers/deploy-server/systemd/` in the repo. This directory will only exist after Phase 1 task P1.16 lands. Skip this section until that task is merged.

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

## 11. Phase 1 deployment (one-time)

Build binary on Hetzner box (or pull from CI artifact in a future iteration):

```bash
git clone <repo> /opt/ppt-deploy-build
cd /opt/ppt-deploy-build/backend
cargo build --release -p deploy-server --bin ppt-deploy --bin pmctl
sudo install -m 0755 target/release/ppt-deploy /usr/local/bin/
sudo install -m 0755 target/release/pmctl       /usr/local/bin/
```

Configure systemd:

```bash
sudo cp /opt/ppt-deploy-build/backend/servers/deploy-server/systemd/*.{socket,service,timer} /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now ppt-deploy.socket ppt-deploy-gc.timer
```

Generate API key for the Claude skill:

```bash
TOKEN=$(openssl rand -hex 32)
HASH=$(printf "%s" "$TOKEN" | sha256sum | awk '{print $1}')
echo "API key (give to Claude skill): $TOKEN"

sudo tee -a /etc/ppt-deploy/auth.yaml <<EOF
api_keys:
  - name: claude-skill
    hash: "$HASH"
EOF

# On the operator's laptop:
mkdir -p "$HOME/.config/ppt-deploy"
echo "$TOKEN" > "$HOME/.config/ppt-deploy/token"
chmod 600 "$HOME/.config/ppt-deploy/token"
```

Generate token for the GC cron (a separate API key avoids leaking the human-facing one):

```bash
GC_TOKEN=$(openssl rand -hex 32)
GC_HASH=$(printf "%s" "$GC_TOKEN" | sha256sum | awk '{print $1}')

sudo tee -a /etc/ppt-deploy/auth.yaml <<EOF
  - name: gc-cron
    hash: "$GC_HASH"
EOF

sudo tee /etc/ppt-deploy/gc.env <<EOF
PMCTL_TOKEN=$GC_TOKEN
EOF
sudo chmod 600 /etc/ppt-deploy/gc.env
```

Test:

```bash
PPT_DEPLOY_URL=https://deploy.rlt.sk pmctl version
```

Expected: JSON `{"status":"ok","version":"..."}`.

Open a worktree for smoke verification:

```bash
PPT_DEPLOY_URL=https://deploy.rlt.sk pmctl open feature/test --json
```

Expected: JSON with `worktree.urls.ppt` set to `https://wt-feature-test.dev.ppt.rlt.sk` (or similar), and the URL should resolve once Caddy provisions the wildcard cert.

Close it:

```bash
PPT_DEPLOY_URL=https://deploy.rlt.sk pmctl close feature-test --json
```
