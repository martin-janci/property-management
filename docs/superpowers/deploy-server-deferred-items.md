# Deploy Server — Deferred Items

After the autonomous fix sprints (13 must-fix items + 20 should-fix items closed), four
larger architectural redesigns remain in the backlog. They are deferred deliberately
because each requires either a schema migration, a multi-week refactor, or a contract
discussion that doesn't yet have a forcing function.

This document captures the decisions made and the trigger conditions that should
re-prioritize each item.

## D1. `Release.state` lifecycle/position conflation

**What:** `Release.state` mixes two orthogonal axes:
- *Lifecycle*: `Candidate` → `Active` → `Retired`
- *Position on target*: `live` vs `previous`

Today these are squashed into one enum (`Candidate | Staging | Prod | Previous | Archived`), which works but cannot express "the release that was live on staging and is now a candidate for prod".

**Why deferred:** Schema redesign requires a migration (`0002_*.sql`) plus reworking
every `current_release_for(target, state)` query. The current shape is functionally
adequate for one staging + one prod target. Cost > value until we have either:

- Multi-region prod (each region has its own live position)
- Canary releases (multiple "live" rows per target with weights)
- Promotion across environments tracked as a chain

**Trigger to revisit:** First proposal for canary % traffic shifting OR the second
production region.

**Migration sketch when triggered:**
```sql
-- 0002_release_target.sql
CREATE TABLE release_target (
  target TEXT NOT NULL,        -- 'staging' | 'prod' | 'eu-prod' | ...
  tag TEXT NOT NULL,           -- FK release.tag
  position TEXT NOT NULL,      -- 'live' | 'previous'
  promoted_at INTEGER NOT NULL,
  PRIMARY KEY (target, position)
);
ALTER TABLE release ADD COLUMN lifecycle TEXT NOT NULL DEFAULT 'active';
-- Backfill from existing `state` column, then drop `state`.
```

## D2. `open_handler` 200-line god-function

**What:** `api/worktree.rs::open_handler` orchestrates: resume-from-dump, git fetch,
port allocation, container spawning, Caddy registration, optional dedicated DB
creation, GHA dispatch + 10-min poll loop, branch-tag derivation, backend container
spawn, second Caddy registration, state persistence. ~250 lines, no transaction
boundary, can't be unit-tested without real Docker + GH + Postgres.

**Why deferred:** Refactor into a workflow engine (durable workflows with steps,
compensations, retries) is the right architectural move but is a 1–2 week project.
It would unblock D1 partially (workflows naturally version their state) and resolve
the synchronous 10-minute GHA wait that pins an HTTP request worker.

**Trigger to revisit:** First incident where a partial-failure leaves orphaned
containers/DBs/dumps that an operator has to clean up by hand. Or the first
multi-step workflow beyond open (e.g. a multi-region promote that touches 3 targets).

**Sketch:**
```rust
// New: backend/servers/deploy-server/src/workflow/
pub trait Step: Send + Sync {
    async fn apply(&self, ctx: &Ctx) -> Result<()>;
    async fn compensate(&self, ctx: &Ctx) -> Result<()>;
}
pub struct Workflow { pub steps: Vec<Box<dyn Step>>, pub state: WorkflowState }
// Workflow rows persisted in sqlite; worker picks pending → runs steps → records progress.
// On failure: runs compensations on completed steps in reverse.
```

`open_handler` becomes ~30 lines that constructs a `Workflow` and returns its ID.
`pmctl open` polls/streams progress. Agent-friendly.

## D3. `router::build` 14-arg signature

**What:** `router::build` takes 14 positional arguments (`#[allow(clippy::too_many_arguments)]`).
Constructed in `main.rs` and three smoke tests. Highly fragile — one rename and all
four call sites break.

**Why deferred:** Cosmetic; the smoke test deduplication (P5.7) made this a one-place
update for the test path. Production has only one call site. A `RouterDeps` struct
would help readability but the underlying design (per-service dependencies threaded
through `WorktreeService` etc.) is the real issue, addressed by D2.

**Trigger to revisit:** Bundled with D2.

**Sketch:**
```rust
pub struct RouterDeps {
    pub store: Arc<Store>,
    pub git: Arc<GitFetcher>,
    pub docker: Arc<DockerClient>,
    pub caddy: Arc<CaddyClient>,
    pub auth: AuthDeps,        // api_keys, oidc, webhook_cfg
    pub services: ServiceDeps, // release, promote, worktree
    pub config: ConfigDeps,    // frontend_image, domains, backend_image_prefix
    pub locks: Arc<WorktreeLockRegistry>,
    pub gh: Arc<GhClient>,
    pub postgres: Arc<PostgresOps>,
}
pub fn build(deps: RouterDeps) -> Router { ... }
```

## D4. Domain types leak infrastructure details

**What:** `domain/worktree.rs::Worktree` has `containers: Vec<String>` (Docker container names),
`dump_path: Option<String>` (filesystem path), `db_name: Option<String>` (Postgres DB name).
These are infra details masquerading as a "stable contract".

The naming choice (`domain/`) implies these should be stable across infra changes —
but in practice they ARE the wire format for sqlite + JSON API, not abstractions.

**Why deferred:** This is a naming/contract debate, not a functional issue. Two
viable resolutions:

1. **Rename** `domain/` → `model/` and accept that these types are the persistence
   shape. Five-minute change, accurate naming.
2. **Hide** infra fields behind opaque types (`Vec<ContainerRef>` where `ContainerRef`
   is just an opaque ID), and let infra-side wrappers carry the docker container
   names. Larger refactor, more orthogonality.

**Trigger to revisit:** Either decision can be made when the first non-Docker
deployment backend appears (e.g. K3s, Nomad), at which point hiding infra details
becomes load-bearing rather than philosophical. Until then, "domain" works as a
synonym for "model" with no production cost.

**Recommended action when triggered:** Option 1 (rename to `model/`) — the
abstraction lift in option 2 doesn't pay off for the same-Docker-stack-everywhere
deployment model.

## Items NOT deferred (also not yet addressed)

The following lower-priority polish from the code review remains open. They are
small enough to be batched into routine cleanup commits without their own dedicated
sprint:

- `parse_duration_secs` exists in `promote.rs` but `Target.idle_timeout` config
  uses a hardcoded 8h (parser unused for that field)
- Audit table unbounded growth — needs a retention pass in GC
- Migration is single file `0001_init.sql` — no `CHECK` constraints on enum cols
- Dashboard token in `prompt()` + `sessionStorage` — XSS-readable
- Public API doc comments — only ~24% of pub items have `///`
- `references/api.md` lists 7 endpoints, router exposes 13+
- pmctl `--help` 11 subcommands without clap groups
- `Cargo.toml` has `tempfile` in both `[dependencies]` and `[dev-dependencies]`
- Vite plugin executes `git rev-parse` 3× per build (cache via env var)
- `replace(['/','_'], "-")` collapsed (clippy `collapsible_str_replace`)
- `if_same_then_else` in `infra/blue_green.rs:99` (intentional tie-break logic)

These are tracked in the broader code review summary but don't warrant new sprints.
