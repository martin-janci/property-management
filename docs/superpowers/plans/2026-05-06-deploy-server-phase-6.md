# Deploy Server Phase 6 — Polish

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development.

**Goal:** Operational polish — read-only web dashboard, auto-rollback for staging, dep hygiene (Dependabot + cargo-deny), Caddy access-log–driven traffic tracking (replacing the heuristic from P2.5), and migration to rootless Docker.

**Architecture:** Each item is independent and can ship separately. Lower-risk items first, higher-risk last. Tasks are smaller than earlier phases — most are config/tooling additions.

**Spec source:** `docs/superpowers/specs/2026-05-06-deploy-server-design.md` § 10 Phase 6.

---

## File Structure

### New
```
backend/servers/deploy-server/src/api/dashboard.rs       # static asset serving + audit query
backend/servers/deploy-server/dashboard/                 # static SPA assets (vite or plain HTML)
.github/dependabot.yml
backend/deny.toml
backend/servers/deploy-server/src/infra/traffic.rs       # Caddy access log tail
```

### Modified
```
backend/servers/deploy-server/src/api/router.rs          # add /dashboard route
backend/servers/deploy-server/src/api/gc.rs              # consume real traffic data
backend/servers/deploy-server/src/api/promote.rs         # auto-rollback for staging
backend/servers/deploy-server/src/main.rs                # spawn traffic tail
docs/runbooks/deploy-server-prereqs.md                   # section 13: dashboard + auto-rollback
```

---

## Tasks

### Task P6.1: Caddy access log tail → traffic tracking

Replace the `release.promoted_at` heuristic in P2.5 with real traffic data.

Configure Caddy to write JSON access logs to `/var/lib/caddy/access.log`. Spawn a tokio task in main.rs that tails the file and updates `worktree.last_traffic_at` based on the Host header.

Create `backend/servers/deploy-server/src/infra/traffic.rs`:

```rust
use crate::infra::Store;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(serde::Deserialize)]
struct CaddyLogLine {
    request: CaddyRequest,
    ts: f64,
}

#[derive(serde::Deserialize)]
struct CaddyRequest {
    host: String,
}

pub async fn tail_caddy_log(path: String, store: Arc<Store>) {
    loop {
        if let Err(e) = tail_once(&path, &store).await {
            tracing::warn!(error = %e, "caddy log tail failed; retrying in 5s");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }
}

async fn tail_once(path: &str, store: &Arc<Store>) -> std::io::Result<()> {
    let f = File::open(path).await?;
    let mut reader = BufReader::new(f).lines();
    // Seek to end? For MVP just stream from current position; logrotate-aware tail is Phase 6+.
    while let Some(line) = reader.next_line().await? {
        if let Ok(entry) = serde_json::from_str::<CaddyLogLine>(&line) {
            // Extract worktree name from host: wt-<name>.dev.ppt.rlt.sk → name
            if let Some(name) = parse_worktree_from_host(&entry.request.host) {
                let _ = store.update_last_traffic(&name).await;
            }
        }
    }
    Ok(())
}

fn parse_worktree_from_host(host: &str) -> Option<String> {
    host.strip_prefix("wt-").and_then(|s| s.split('.').next()).map(|s| s.to_string())
}
```

Add `Store::update_last_traffic(&self, name: &str) -> Result<()>` that updates `worktree.last_traffic_at = now`.

In `main.rs`, after constructing store:

```rust
if let Some(log_path) = std::env::var("CADDY_ACCESS_LOG").ok() {
    let store_clone = store.clone();
    tokio::spawn(async move {
        deploy_server::infra::traffic::tail_caddy_log(log_path, store_clone).await;
    });
}
```

GC tick can now use real `last_traffic_at` (which is what worktree branch already does — only staging used the heuristic).

Update `gc.rs` staging arm:

```rust
// Phase 6: use real traffic from Caddy log if available; fall back to promoted_at heuristic.
let staging_idle = chrono::Duration::seconds(8 * 3600);
// (still based on promoted_at for now; staging-specific traffic tracking would need
// host-based filtering for *.staging.* — left as TODO)
```

(For staging, the heuristic stays; full per-target tracking is a future improvement. The tail handles worktree subdomains for now.)

Compile. Commit `feat(deploy-server): Caddy access log tail → real traffic tracking`.

---

### Task P6.2: Auto-rollback for staging

Currently `targets.staging.rollback_mode` is `manual`. Phase 6 makes auto-rollback the default for staging (lower risk than prod since staging has no users).

Update `config.yaml` template / runbook examples to set `rollback_mode: auto` for staging:

```yaml
staging:
  rollback_mode: auto      # Phase 6: try auto-rollback on staging first
prod:
  rollback_mode: manual    # default; flip to auto only after Phase 6 maturity
```

The promote handler from P4.4 already supports the `auto` path. No code change needed — just operator config.

Update `docs/runbooks/deploy-server-prereqs.md` to recommend `auto` for staging.

Commit `docs(runbook): recommend auto rollback mode for staging`.

---

### Task P6.3: Read-only web dashboard

Static SPA at `onyx.rlt.sk/dashboard` served by Caddy. The deploy server exposes JSON endpoints; the SPA fetches them.

Create a tiny static dashboard:
```
backend/servers/deploy-server/dashboard/
├── index.html
└── app.js     # fetches /api/audit and /api/worktrees, renders tables
```

Add Caddy route to serve `/dashboard/*` from filesystem path. Phase 6 keeps this minimal; full React SPA is overkill.

`index.html`:
```html
<!DOCTYPE html>
<html>
<head><title>ppt-deploy dashboard</title></head>
<body>
  <h1>ppt-deploy</h1>
  <h2>Worktrees</h2>
  <table id="worktrees"><thead><tr>
    <th>Name</th><th>Branch</th><th>State</th><th>URLs</th><th>Created</th>
  </tr></thead><tbody></tbody></table>
  <h2>Audit (last 50)</h2>
  <table id="audit"><thead><tr>
    <th>Time</th><th>Caller</th><th>Endpoint</th><th>Result</th>
  </tr></thead><tbody></tbody></table>
  <script src="app.js"></script>
</body>
</html>
```

`app.js`:
```javascript
const TOKEN = prompt('API token:');
const auth = { 'Authorization': `Bearer ${TOKEN}` };

async function load() {
  const wts = await fetch('/api/worktrees', { headers: auth }).then(r => r.json());
  const tbody = document.querySelector('#worktrees tbody');
  tbody.innerHTML = wts.map(w => `
    <tr>
      <td>${w.name}</td>
      <td>${w.branch}</td>
      <td>${w.state}</td>
      <td>${w.urls.ppt || ''}</td>
      <td>${new Date(w.created_at).toLocaleString()}</td>
    </tr>
  `).join('');

  const audit = await fetch('/api/audit?limit=50', { headers: auth }).then(r => r.json());
  document.querySelector('#audit tbody').innerHTML = audit.map(a => `
    <tr>
      <td>${new Date(a.ts * 1000).toLocaleString()}</td>
      <td>${a.caller_kind}:${a.caller_id}</td>
      <td>${a.endpoint}</td>
      <td>${a.result}</td>
    </tr>
  `).join('');
}

load();
setInterval(load, 30000);
```

Add `GET /api/audit?limit=N` handler in `api/audit.rs` (new file or extend the audit middleware module). Returns recent audit rows.

Add `Store::list_audit(&self, limit: i64) -> Result<Vec<AuditRow>>`.

Commit `feat(deploy-server): read-only web dashboard + /api/audit endpoint`.

---

### Task P6.4: Dependabot + cargo-deny

`.github/dependabot.yml`:

```yaml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/backend"
    schedule:
      interval: "weekly"
    groups:
      tokio:
        patterns: ["tokio*"]
      axum:
        patterns: ["axum*", "tower*"]
      serde:
        patterns: ["serde*"]
      sqlx:
        patterns: ["sqlx*"]
  - package-ecosystem: "npm"
    directory: "/frontend"
    schedule:
      interval: "weekly"
  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
```

`backend/deny.toml`:

```toml
[graph]
targets = [{ triple = "x86_64-unknown-linux-gnu" }]

[advisories]
vulnerability = "deny"
unmaintained = "warn"
yanked = "deny"
notice = "warn"
ignore = []

[bans]
multiple-versions = "warn"
deny = []

[licenses]
unlicensed = "deny"
allow = ["MIT", "Apache-2.0", "BSD-3-Clause", "ISC", "Unicode-DFS-2016", "MPL-2.0", "CC0-1.0", "BSD-2-Clause", "Zlib"]
```

Add CI step to run `cargo deny check`:

In `.github/workflows/backend.yml` (existing):

```yaml
- name: cargo deny
  uses: EmbarkStudios/cargo-deny-action@v1
```

Commit `chore: dependabot + cargo-deny`.

---

### Task P6.5: Per-worktree token scoping (deferred from P1)

If multiple humans/agents start using the deploy server, scope API tokens to specific worktrees / actions. Out of MVP scope but documented.

For now: skip. Document in spec § 12 open questions.

No commit (already covered by spec).

---

### Task P6.6: Rootless Docker migration

Plan-only. Migration from rootful → rootless requires:
1. Switch deploy server's docker socket from `/var/run/docker.sock` → `~ppt-deploy/.local/share/docker.sock`.
2. Test all worktree + staging flows.
3. Update systemd service to run with `User=ppt-deploy` (already there) but without `docker` group.

Out of immediate scope but documented.

Append section 13 to operator runbook describing the migration path. Commit `docs(runbook): rootless Docker migration plan`.

---

### Task P6.7: cargo-deny audit in CI

If P6.4 is shipped, this is included. Add a smoke verification step.

Commit `ci: enforce cargo-deny check on PRs`.

---

## Self-Review Coverage

| Spec deliverable (Phase 6) | Plan task |
|---|---|
| Web dashboard (read-only) | P6.3 |
| Auto-rollback on staging | P6.2 |
| Dependabot + cargo-deny | P6.4, P6.7 |
| Per-worktree token scoping | P6.5 (deferred) |
| Rootless Docker | P6.6 (documented) |
| Real traffic tracking | P6.1 |

Phase 6 deferrals (acceptable):
- Per-worktree token scoping (security need not yet pressing)
- Rootless Docker (migration risk; do when ready)
- Full SPA dashboard (current minimal HTML is enough)
