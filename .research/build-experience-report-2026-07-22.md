# Build Experience & Deterministic AI-Agent Verify Loop — Expert Panel Report

Date: 2026-07-22
Method: 4 parallel expert analyses (Rust build-perf, CI architecture, AI-agent workflow, web research 2025–2026 state of the art) + moderator adjudication of conflicts.
Input: external agent's recommendation (agent-loop/CI separation, impact analysis, layered CI, sccache, nextest, AI-review placement) — validated and adapted to this repo.

## TL;DR

The external agent's architecture is directionally right, but three of its specifics are wrong for THIS repo:

1. **`cargo check` before clippy is a pure double-pay** — clippy does not reuse check artifacts (2024 cargo regression, fix is a 2026 project goal). Clippy IS the type gate.
2. **`CARGO_INCREMENTAL=0` + sccache globally is a false dilemma locally** — cargo only does incremental for workspace members, which sccache skips anyway; keep incremental ON locally, sccache caches the 790-crate dep graph.
3. **sccache-on-GHA-cache-backend is mostly hype** (maintainers' own words); on GH-hosted runners the 90% solution is tuned `Swatinem/rust-cache` (+`shared-key`, `save-if` on dev, `CARGO_INCREMENTAL=0` in CI only). sccache pays off with an S3/R2 backend or local disk on fleet hosts.

The root structural cost is not caching at all: **~330 integration-test binaries** (api-server 201, db 96, reality-server 32) each statically link the whole server — the workspace is link-dominated — plus **two 148k-LOC monolith crates (`db`, `api-server`) serializing the dep graph**.

## Current state (verified)

**Good already:** lld linker; `[profile.test] debug=false`; SQLx runtime queries (zero proc-macro DB cost — do NOT add `.sqlx` offline metadata for "speed"); pinned toolchain 1.94.1; CI single-producer required contexts `check`/`test` (#1672 fix — the standout design); hash-sharded CI tests; mold + cargo-nextest already installed on the dev host but unused.

**Problems:**
- No `[profile.dev]` tuning — full debuginfo across an 802-package graph (tonic, aws-sdk, sentry×10, opentelemetry×5).
- 44+ worktrees, each with private `backend/target/` (~cold build per fresh agent worktree) and no sccache/no sharing.
- CI: `check` (15–25 min) serializes in front of all test legs; fmt+clippy runs 3×; `cargo install sqlx-cli` from source in 3 jobs (~5–10 min each); release `build --workspace` on every PR (not required, pure runner burn); rust-cache split across ~10 cache namespaces thrashing the 10 GB cap; `auto-approve` sleep-polls 30 min.
- Agent loop: skills teach `-p` scoping but every terminal gate (implementer-prompt IG7, ppt-pr-create precondition, per-skill "After-task verification") collapses to full 3-stack `just check && just test && just build` — including a release build and a Gradle build for docs-only PRs. Band A/B/C selection is judgment-based, i.e. non-deterministic.
- `deploy-server` is the sole sqlite-feature user → `-p` vs `--workspace` feature-unification thrash.
- `utoipa-swagger-ui` declared in `api-core` but only used in server `main.rs` files — build script on the mid-graph critical path.
- Stale: `.cargo` alias `dev = "run --package auth-server"` (crate doesn't exist), `backend/.autopilot/` fossil, justfile `feature`/`bugfix`/`sync` recipes branch from `main` (model says `dev`).
- `api-validation.yml` never diffs the generated SDK against the committed client — the advertised drift gate does not exist.
- Frontend is effectively ungated (path-filtered workflows can't be required contexts).

## Adjudicated verdicts

| Conflict | Verdict |
|---|---|
| sccache vs shared `CARGO_TARGET_DIR` | **sccache local-disk, per-worktree targets.** Shared target dir serializes every agent on cargo's build lock and cross-branch incremental thrash makes it worse. Keep incremental ON locally. Complement later with a nightly `dev` template `target/` hardlink-cloned (`cp -al`) into fresh worktrees (covers workspace crates + link artifacts sccache can't). |
| Local verify chain | `cargo fmt --all` → `cargo clippy -p X --all-targets -- -D warnings` → `cargo nextest run -p X` (fallback `cargo test -p`). **No `cargo check` step. Never `--release` locally.** |
| Feature-unification fix | **Evict `deploy-server` from the workspace** (own `[workspace]`, root `exclude`, small dedicated CI step). It's a leaf ops tool and the sole sqlite user. Hakari rejected (generated crate agents will forget to regenerate); always-`--workspace` rejected (throws away `-p` scoping). |
| nextest order | **CI first** (archive + `--partition slice:m/n`), local same-time as cheap QoL. Neither substitutes for test-binary consolidation — the real lever. nextest's per-test process isolation is what makes consolidation safe. |
| Release build in PR path | **Move to main-push + nightly.** Test jobs already link hundreds of dev binaries (catches link errors); release-only failures are rare and caught pre-docker on main. Agents never run `just build`. |

## TOP-10 Roadmap (moderator-ranked, impact ÷ risk)

| # | Item | Impact | Effort | Risk | Depends |
|---|------|--------|--------|------|---------|
| 1 | CI: de-serialize `test-shard`/`test-rest` from `check` (`needs: [changes]`; keep `security-tests`/`rls-smoke` edges on `check`) | Very high (15–25 min/PR) | 2 lines | Low, #1672-safe | — |
| 2 | CI: move `cargo build --release --workspace` to main-push + nightly | High (frees a big runner/PR) | Trivial | Low (not required context) | — |
| 3 | Skills: standardize verify chain (fmt → clippy `-p` → nextest `-p`; delete check step; ban local `--release`/`just build`) | High (every agent loop) | Low | None | — |
| 4 | Hosts: sccache local-disk (`RUSTC_WRAPPER=sccache`, generous `SCCACHE_DIR`), keep per-worktree targets + incremental | High (cold dep builds −60–80%) | Low | Low | — |
| 5 | Evict `deploy-server` to own workspace | Med-high (kills `-p`/`--workspace` thrash permanently) | Low-med | Low-med | before 3 fully pays |
| 6 | CI: collapse duplicate `cargo check --workspace` — clippy sole type gate (keep `check` context NAME; internal steps only) | High (~10 min/PR) | Medium | Medium (fragile required context) | 1 |
| 7 | CI: nextest archive + partition for shards (retries `flaky-result="fail"`, JUnit, per-test timeouts; replaces `ci-test-shard.sh`) | High | Medium | Medium | 1, 6 |
| 8 | Consolidate integration-test binaries (api-server 201 → ~4–8 domain harnesses via `mod`; then db) | Very high (root cause of link domination) | High, incremental | Medium | 7 |
| 9 | Warm worktree bootstrap: nightly dev template `target/` + `cp -al` on worktree create | Med-high | Medium | Low | 4 |
| 10 | mold linker locally (already installed; swap lld in `.cargo/config.toml`) + evaluate Depot/Namespace runners (~2× speed, ~half cost) | Medium | Trivial / Medium | Low / Medium | runners after 1-2-6-7 |

## Also do (cheap, from individual experts)

- `[profile.dev] debug = "line-tables-only"`, `split-debuginfo = "unpacked"`; `[profile.dev.package."*"] debug = false` (consider `opt-level = 1` — tradeoff vs cold builds).
- Move `utoipa-swagger-ui` from `api-core` to the three servers.
- Fix `.cargo` aliases (`dev` → api-server; add `verify`/`nt`); delete `backend/.autopilot/`; fix justfile `feature`/`bugfix`/`sync` base branch.
- Replace `cargo install sqlx-cli` with binary install (taiki-e/install-action) in 3 CI jobs (~8 min → ~10 s each).
- Delete `deny-ban-presence` (duplicates `deny-toml-ban-gate.yml`) and backend.yml `lint` job (redundant with `check`+fmt-clippy-gate).
- rust-cache: collapse cache namespaces, `shared-key`, `save-if: ref == 'refs/heads/dev'`, `cache-on-failure: true`, `CARGO_INCREMENTAL=0` in CI.
- Close the api-client drift gate: regenerate SDK + `git diff --exit-code` in api-validation.yml.
- Frontend single-producer gating: replicate backend.yml's `changes`+no-op pattern so frontend contexts can become required (~40 lines, well-precedented).
- Event-driven auto-approve (workflow_run) instead of 30-min sleep-poll.
- Encode invariants as deterministic lints: `clippy.toml` disallowed-methods/types first (e.g. `std::env::set_var`, raw `sqlx::query` outside repositories), dylint for RLS-context invariants; rule: **every agent-fixed bug mints a test or lint** (the repo's #1755 migration-collision guard is the template).

## `just verify` — deterministic impact-scoped gate (design)

Single entry point; plan = pure function of (merge-base with origin/dev, sorted changed paths, escalation table). `just verify-plan` prints the plan; PR body quotes `VERIFY-PLAN` + `VERIFY OK <hash>` (parseable by the Phase-5 reviewer / ppt-goal-gate).

Escalation table (deterministic, replaces judgment-based Band A/B/C):

| Changed path matches | Scope |
|---|---|
| `backend/Cargo.{toml,lock}`, `rust-toolchain*`, `.cargo/**`, any `build.rs` | full backend (clippy+test `--workspace`) |
| `backend/crates/db/migrations/**` | full backend + WARN "SQL truth needs live DB (:5433)" |
| `docs/api/typespec/**` | tsp compile + regen clients + full backend clippy + full frontend typecheck |
| `frontend/pnpm-lock.yaml`, workspace configs, `biome.json*` | full frontend |
| other `backend/**` | owning crate + reverse dependents (one `cargo metadata` call, ~1 s) |
| other `frontend/**` | `pnpm --filter "...[<merge-base>]"` (pnpm-native dependent selection) |
| `mobile-native/gradle*` | full gradle build; other `mobile-native/**` → spotless+test |
| docs/`.research/**` only | **empty plan, exit 0** |

Guardrails for skills (shared `_verify-rules.md`):
1. One gate: `just verify` is the only pre-PR verify; no hand-composed full-workspace commands.
2. Never `cargo build --workspace`/`--release` locally.
3. Don't re-run an unchanged failing command; report `status=partial` instead.
4. flock per-host around cargo invocations (serialize heavy compiles, not the fleet).
5. Narrow inner-loop commands allowed; the gate is not skippable before ppt-pr-create.
6. Quote the plan + OK hash in the PR body, not logs.
7. AI review sits on top of, never replaces, green deterministic gates.

Files to rewire (highest leverage first): `.research/implementer-prompt.md` (IG7 → `just verify`), `ppt-pr-create` precondition, `ppt-implement` Band A/B/C → automatic, `ppt-tests`/`ppt-rust-backend`/`ppt-frontend` "After-task verification", `scripts/pre-push` (path-gate — today it full-clippys docs-only pushes), `scripts/pre-commit` (scope frontend checks). Ship with fixture self-test (`test-verify-impact.sh`) in the repo's established deterministic-script idiom (owner-areas.json / goal-check.sh style).

Adoption: ship script → shadow mode (run both old+new on a few PRs, compare) → flip IG7 → update skills → path-gate hooks.

## Merge queue (medium-term)

GitHub native merge queue IS feasible: all required contexts are produced unconditionally on every PR (exactly the property a queue needs). Requirements: add `merge_group:` trigger to every required-context workflow; `changes` must handle merge_group events; never path-skip a required check (queue deadlocks). It would structurally fix the recurring migration-version-collision dev breakage (tests the speculative merge result), obsolete the App.tsx soft queue, the auto-approve poll loop, and `--admin` merge races. Cost: every queued group pays the full `test` tier — mitigate via fast-tier-on-PR / full-tier-on-merge_group with the same context names.

## Repo-specific hazards (do not violate)

- **Required-context renames wedge the repo**: `check`/`test` are literal names in branch protection; branch-protection-setup merges (existing ∪ new) and never removes — a rename permanently blocks merges until manually pruned. Refactors keep exactly one unfiltered `pull_request` workflow emitting each name.
- The no-op gating is one `if:` away from BIT-346 recurring — consider a CI lint greping that every heavy step in `check`/`test` carries the `changes` guard.
- `quick-xml =0.41.0` pin is load-bearing security posture (XXE, cargo-deny ban, CODEOWNERS) — nothing here may casually touch it; deploy-server eviction must keep deny coverage.
- Do NOT add `.sqlx` offline metadata "for speed" — it adds proc-macro cost + the known schema-drift trap (#1008) for zero build benefit.
- `justfile db-prepare` (`cargo sqlx prepare --workspace`) contradicts the DB-free-compile convention — remove or fence it.

## Key sources (web expert, verified)

- nextest: https://nexte.st/docs/ci-features/archiving/ , /partitioning/ (slice:m/n, v0.9.127), flaky-result="fail" (v0.9.131)
- sccache GHA caveats: mozilla/sccache#1762, #1485; Depot writeup depot.dev/blog/sccache-in-github-actions
- rust-cache best practice: github.com/Swatinem/rust-cache README (shared-key, save-if, workspace-crate pruning)
- rust-lld default on Linux since 1.90: blog.rust-lang.org/2025/09/01/rust-lld-on-1.90.0-stable
- clippy/check cache unification: rust-lang.github.io/rust-project-goals/2026/cargo-cross-workspace-cache.html
- hakari: docs.rs/cargo-hakari (feature-unification, prerequisite for `-p` speed)
- merge-queue path-filter deadlock: github.com/orgs/community/discussions/44490
- Runner benchmarks: runs-on.com/benchmarks/ ; Depot/Namespace/Blacksmith/WarpBuild ≈ $0.003–0.004/min vs GH $0.008 at ~2× perf
- Agent-CI pattern ("AI finds it, deterministic test prevents regression"): anthropic.com/research/claude-code-expertise ; dylint (Trail of Bits); Sonar loop-engineering post
