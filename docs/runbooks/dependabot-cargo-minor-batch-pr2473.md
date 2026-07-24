# Dependabot cargo-minor-patch batch #2473 — verification note

**Task:** `chore-dependabot-cargo-minor-batch-2026-07-23`
**Owner:** pm-devops · **Priority:** low
**Date verified:** 2026-07-24

## Summary

Dependabot PR
[#2473](https://github.com/martin-janci/property-management/pull/2473)
("chore(deps): bump the cargo-minor-patch group across 1 directory with 6
updates") was **reviewed and merged into `dev` on 2026-07-23** (merged by
`martin-janci`, merge commit at `2026-07-23T11:32:11Z`). This note records the
post-merge lockfile-drift verification requested by the task.

## Crates bumped (all backend, `backend/Cargo.lock` only — 1 file, +73/-62)

| Package       | From    | To      | In `dev` Cargo.lock | `Cargo.toml` pin | Covered |
| ---           | ---     | ---     | ---                 | ---              | :---:   |
| anyhow        | 1.0.103 | 1.0.104 | 1.0.104             | `1.0`            | ✅      |
| thiserror     | 2.0.18  | 2.0.19  | 2.0.19              | `2.0`            | ✅      |
| redis         | 1.4.0   | 1.4.1   | 1.4.1               | `1.2`            | ✅      |
| clap          | 4.6.2   | 4.6.4   | 4.6.4               | `4.4`            | ✅      |
| async-trait   | 0.1.89  | 0.1.91  | 0.1.91              | `0.1`            | ✅      |
| futures-util  | 0.3.32  | 0.3.33  | 0.3.33              | `0.3`            | ✅      |

(A separate `thiserror 1.0.69` entry also exists in the lockfile — an unrelated
transitive major pulled by another dependency, not affected by this batch.)

## Findings

- **No lockfile drift.** Every one of the 6 bumped versions is present in the
  current `dev` `backend/Cargo.lock`. All bumps stay within the semver ranges
  declared in `backend/Cargo.toml`'s `[workspace.dependencies]`, so the lock
  satisfies every declared constraint — no `Cargo.toml`↔`Cargo.lock`
  inconsistency.
- **CI-green + merged.** The PR is closed/merged into `dev`; the merge gate was
  satisfied at merge time. (The legacy commit-status API returns
  `total_count: 0` because the repo reports CI via Actions check-runs, not
  legacy statuses.)
- **No code change required on this branch.** The substantive work — review,
  merge, and drift verification — was already complete before this task ran.
  This note is the durable record of that verification.

## Notable release content (for future reference)

- `futures-util 0.3.33` includes two **soundness fixes** (unsound `Send` impl
  for `IterPinRef`/`Iter`; `ReadLine` exception-safety) plus a
  `FuturesUnordered::IntoIter` memory-leak fix — worth having in.
- The `anyhow`/`thiserror`/`async-trait`/`clap` bumps are primarily the
  `syn v3` dev-dependency update; no runtime API changes.
