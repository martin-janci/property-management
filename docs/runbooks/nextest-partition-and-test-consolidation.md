# Backend Test Runbook — nextest Archive/Partition + the 206→8 Suite Consolidation

**Task:** `devops-nextest-partition-runbook-2026-07-23`
**Owner:** pm-devops
**Applies to:** `backend/` integration tests, `backend/.config/nextest.toml`, `.github/workflows/backend.yml`
**Source of truth for the design:** `.research/build-experience-report-2026-07-22.md` (roadmap items 7 & 8)
**Landed by:** #2459 (nextest archive + per-test partition) · #2461 (api-server 206→8) · #2487 (db + reality-server)

---

## Who this is for

You are adding or moving a backend integration test (`backend/**/tests/*.rs`),
or you touched `backend.yml` / `nextest.toml` and need to understand why CI is
shaped the way it is.

**The one rule to remember:** do **not** add a new top-level `tests/<name>.rs`
file. Drop your test in `tests/suites/` and register it in one of the existing
`suite_N.rs` harnesses. The [orphan-test guard](#guardrail-the-orphan-test-guard)
will fail CI if you get this wrong, so you can't silently ship a
never-compiled test — but knowing the pattern up front saves a red run. See
[Adding a test](#adding-a-new-integration-test-the-runbook).

---

## Why this exists (the problem)

Cargo compiles **each** `tests/*.rs` file into its **own** integration-test
binary, and every one of those binaries statically links the whole server (+ the
790-crate dependency graph). Before consolidation the backend carried ~330
integration-test binaries (api-server ~201, db ~96, reality-server ~32). The
workspace was **link-dominated**: `cargo test --workspace` paid the full
server-link cost hundreds of times and ran ~107–119 min — right at the CI
`timeout … 150m` ceiling, wedging every open backend PR on the shared `test`
gate.

Two independent changes fixed this. They compose but solve different halves:

| Change | Layer | What it fixes |
|--------|-------|---------------|
| **A. nextest archive + per-test partition** (#2459) | CI (`backend.yml`, `nextest.toml`) | Compile the test surface **once**, then fan the *run* across 4 parallel legs. Kills the "compile the workspace 5×" waste of the old hash-sharded model. |
| **B. 206→8 suite consolidation** (#2461, #2487) | Source layout (`tests/`) | Collapse hundreds of one-file binaries into a handful of multi-module **suite harnesses**, so the expensive server link is paid ~8× instead of ~206×. |

Consolidation is the root-cause lever; nextest is what makes it *safe*
(per-test process isolation means bundling many tests into one binary doesn't
let them stomp each other's global state).

---

## Part A — nextest archive + per-test partition (#2459)

### The CI shape (`.github/workflows/backend.yml`)

```
test-build ──▶ upload "nextest-archive"  (compile ALL test binaries ONCE + doctests)
     │
     ├──▶ test-shard [1/4]  download archive, run --partition hash:1/4
     ├──▶ test-shard [2/4]  download archive, run --partition hash:2/4
     ├──▶ test-shard [3/4]  download archive, run --partition hash:3/4
     └──▶ test-shard [4/4]  download archive, run --partition hash:4/4
                    │
                    ▼
              test (REQUIRED context) — thin aggregator, red iff any leg failed
```

- **`test-build`** runs `cargo nextest archive --workspace --archive-file
  nextest-archive.tar.zst`. This compiles every workspace test target exactly
  once and packs the binaries into a zstd archive uploaded as the
  `nextest-archive` artifact (retention 1 day). Doctests are **not** in
  nextest's scope, so `cargo test --doc --workspace` runs here too — once —
  keeping doctest coverage from being silently dropped.
- **`test-shard`** (matrix `shard: [1,2,3,4]`) downloads the archive and runs
  only its slice: `cargo nextest run --archive-file … --workspace-remap .
  --profile ci --partition 'hash:${shard}/4'`. **No compilation** happens here
  — wall clock is pure test runtime + Postgres/Redis service startup.
  Partitioning is **per-test** (`hash:m/n`), finer than the old per-binary
  split, so the slices are well-balanced regardless of how tests cluster into
  binaries.
- **`test`** is the single required status context (branch protection needs
  exactly `check` + `test` — see #1672). It's a thin aggregator that fails red
  iff `check`, `test-build`, or any `test-shard` leg did not succeed. Coverage
  is exactly-once with no gap: `{4 shards} ∪ {doctests}` = full workspace
  surface.

> **`--workspace-remap .` is load-bearing.** The archive holds binaries built on
> the `test-build` runner; the shard runners re-anchor them to *their* checkout
> so fixture files, migration dirs, and `include_str!` paths resolve. Both
> `test-build` and `test-shard` still `actions/checkout` for this reason.

### The `ci` profile (`backend/.config/nextest.toml`)

The partition legs run `--profile ci`. Local runs use the default profile
unless you pass `--profile ci` yourself.

```toml
[profile.ci]
fail-fast = false                                        # report every failure in one run (PAP-123)
test-threads = 4                                         # #[sqlx::test] provisions a DB per test; >4 exhausts the CI PG pool → silent hang (BIT-334)
retries = 1                                              # one auto-retry; a pass-on-retry is reported FLAKY (visible, not silent)
slow-timeout = { period = "120s", terminate-after = 15 } # per-test hang guard: killed individually after 30 min, leg keeps going

[profile.ci.junit]
path = "junit.xml"
```

Every knob here replaces a coarser whole-leg mechanism (the old
`--no-fail-fast`, the `timeout 120m` SIGKILL that took down the entire leg,
etc.). If a test is flaky under CI concurrency, fix the test — do **not** bump
`test-threads` up; the cap protects the shared Postgres pool.

### The two special cases

1. **Doctests** — run only in `test-build` (`cargo test --doc --workspace`).
   nextest can't run them. Observed 0 executable doctests today; if one ever
   needs Postgres it will fail loudly there.
2. **Redis-gated WS integration test** — `ws_integration_tests.rs` is kept as a
   **standalone binary** (not folded into a suite) so CI can target it by
   `binary_id`. It's `#[ignore]`-d (needs live Redis) and runs once on shard 1
   via `--run-ignored ignored-only --test-threads=1 -E
   'binary_id(=api-server::ws_integration_tests)'`.

### Running a partition locally

You rarely need this — `just verify` picks the right scope — but to mirror a CI
leg:

```bash
cd backend
cargo nextest run --workspace --profile ci --partition 'hash:1/4'   # slice 1 of 4
cargo nextest run --workspace --profile ci                          # all, CI semantics
cargo nextest run -p api-server                                     # default profile, one crate (inner loop)
```

---

## Part B — the 206→8 suite consolidation (#2461, #2487)

### The pattern

Each former top-level `tests/<name>.rs` (one binary each) now lives in
`tests/suites/<name>.rs` as a plain module, and a small number of
`tests/suite_N.rs` **harness binaries** pull them in via `#[path]`:

```rust
// backend/servers/api-server/tests/suite_1.rs
// Consolidated integration-test harness 1/8 …

mod common;                                                  // shared TestApp helpers

#[path = "suites/accounting_invoices_tests.rs"]
mod accounting_invoices_tests;
#[path = "suites/admin_mfa_disable_tests.rs"]
mod admin_mfa_disable_tests;
// … ~26 modules per suite …
```

`#[path]` is required because the files live in a *subdirectory* of `tests/`;
without it cargo's auto-discovery would never compile them (that's exactly the
[orphan-test](#guardrail-the-orphan-test-guard) failure mode). Each `suite_N.rs`
compiles to **one** binary linking ~26 test modules — one server link instead of
26.

### Current layout (as landed)

| Crate | Harnesses | Files in `tests/suites/` | Notes |
|-------|-----------|--------------------------|-------|
| `servers/api-server` | `suite_1.rs` … `suite_8.rs` (8) | 208 | + `ws_integration_tests.rs` standalone (Redis `binary_id` target); `mod common;` → `tests/common/mod.rs` |
| `crates/db` | `suite_1.rs` … `suite_4.rs` (4) | 98 | + `rls_smoke_tests.rs` / `rls_penetration_tests.rs` standalone (CI runs them by `--test <name>` with special DB roles) |
| `servers/reality-server` | `suite_1.rs`, `suite_2.rs` (2) | — | `mod common;` → `tests/common.rs` |

Files are distributed across suites **alphabetically and roughly evenly**
(~26 per suite). There is **no generator script** — the `suite_N.rs` files are
hand-maintained.

Standalone binaries that were deliberately **not** consolidated: the two db RLS
suites (they need a non-superuser Postgres role and run in their own CI jobs —
`rls-smoke-test`, `security-tests`) and api-server's `ws_integration_tests`
(Redis `binary_id` targeting). Leave these alone.

---

## Adding a new integration test (the runbook)

You wrote `foo_happy_path_tests.rs` for, say, api-server. Do this:

1. **Place the file in the `suites/` subdirectory**, not at the top level:

   ```bash
   backend/servers/api-server/tests/suites/foo_happy_path_tests.rs
   ```

2. **Register it in one `suite_N.rs` harness.** Pick the suite by alphabetical
   fit and keep the list sorted (it makes review diffs clean and keeps the
   suites balanced). Add both lines:

   ```rust
   #[path = "suites/foo_happy_path_tests.rs"]
   mod foo_happy_path_tests;
   ```

   Any suite works for correctness — the alphabetical/balanced convention is for
   humans, not the compiler. If the suites have drifted badly out of balance,
   put it in the smallest one.

3. **Use the shared harness helpers**, don't re-bootstrap. Each suite already
   does `mod common;`. Your module gets `TestApp` / JWT-minting / `AppState`
   construction from there (`tests/common/mod.rs` for api-server & db,
   `tests/common.rs` for reality-server). Reference it as `crate::common::…`
   from inside a suite module. Do **not** add a fresh env-`Once` guard or a new
   `TestApp` clone — that was the class of bug the consolidation surfaced (a
   pubsub test only passed when a sibling in the same process had initialized
   the env guards first).

4. **Verify locally against one crate** (fast inner loop — the full gate runs in
   CI):

   ```bash
   cd backend
   cargo nextest run -p api-server -E 'test(foo_happy_path)'   # just your tests
   cargo nextest run -p api-server                             # the whole crate's suites
   ./scripts/check-no-orphan-tests.sh                          # prove it's wired in
   ```

5. **Run the gate** before the PR: `just verify` (from the repo root). It scopes
   automatically to the crate you touched + reverse dependents.

### Do NOT

- ❌ Create `tests/foo_happy_path_tests.rs` at the top level (a new binary —
  reintroduces the link-domination the consolidation removed; also trips no
  guard, so it silently regresses build time).
- ❌ Put a bare `.rs` in `tests/suites/` without a `#[path]` line in some
  `suite_N.rs` (orphan — never compiled, never runs; the guard fails CI).
- ❌ Fold `ws_integration_tests.rs`, `rls_smoke_tests.rs`, or
  `rls_penetration_tests.rs` into a suite — they're standalone on purpose.
- ❌ Add per-test `std::env::set_var` / duplicate env `Once` init — use
  `common`.

### Moving / renaming a suite file

Move the file inside `tests/suites/` and update its `#[path]` line in the
harness. Keep the `mod` name in sync with the filename (the guard checks
per-file reachability once a `#[path]` for that subdir exists).

---

## Guardrail: the orphan-test guard

`backend/scripts/check-no-orphan-tests.sh` (run in the required `check` job, GH
#2158) fails the build if any `.rs` under a crate's `tests/` subdirectory is
**never compiled** — i.e. no top-level `tests/*.rs` reaches it.

It understands the consolidated-harness pattern (added 2026-07-23 alongside the
consolidation): for a subdir referenced by `#[path]`, reachability is checked
**per file** — every `.rs` directly under `tests/suites/` must be
`#[path]`-referenced by some `suite_N.rs`. A single file you forgot to register
is flagged even though its siblings compile. That's stricter than the old
whole-dir `mod` rule and is exactly what keeps a dropped `#[path]` line from
silently disabling a test.

Its own regression test is `check-no-orphan-tests.test.sh` (a pure-shell gate
with no test can rot — same rationale as the migration-collision guard).

---

## Gotchas / FAQ

- **My test passes alone but fails in the suite.** Almost always shared global
  state (env vars, a `Once`, a static). Route all setup through `common`;
  nextest gives each test its own *process*, but modules in one binary still
  share statics if a test reaches for them directly. This is a test bug, not a
  harness bug.
- **CI is ENOSPC on a shard.** The archive extracts the full
  `target/debug/deps` test-binary set (tens of GB) into `/tmp`. Both
  `test-build` and `test-shard` carry a "Free disk space" step that reclaims
  the preinstalled toolchains (dotnet/android/ghc/swift/CodeQL, ~25 GB). If you
  add a job that extracts the archive, copy that step (Dependabot #2471 hit this
  on a shard that lacked it).
- **Why 4 shards / 8 suites?** Independent numbers. 4 = CI partition legs
  (matrix + the `/4` in `--partition`; keep them in sync). 8 = api-server
  harness binaries. Changing one doesn't require changing the other.
- **PDF font failures in a shard.** `test-shard` installs `fonts-liberation`
  (the voting PDF-report path loads Liberation TTFs). Not a suite concern.
- **Don't rename the `check` / `test` contexts.** They're literal
  required-status-check names in `dev` branch protection; a rename permanently
  wedges merges until an admin prunes the old name (#1672, and the hazards list
  in the build-experience report).

---

## References

- `.research/build-experience-report-2026-07-22.md` — roadmap items 7 (nextest)
  & 8 (consolidation); the "why", the adjudicated verdicts, and the repo hazards.
- `.github/workflows/backend.yml` — `test-build` / `test-shard` / `test` jobs
  (heavily commented with the #1672 / #2222 / #2459 history).
- `backend/.config/nextest.toml` — the `ci` profile.
- `backend/scripts/check-no-orphan-tests.sh` (+ `.test.sh`) — the guard.
- `backend/servers/api-server/tests/suite_1.rs` — a live harness to copy from.
- PRs: #2459 (nextest archive/partition), #2461 (api-server 206→8), #2487 (db +
  reality-server).
- nextest docs: <https://nexte.st/docs/ci-features/archiving/> ·
  <https://nexte.st/docs/ci-features/partitioning/>
