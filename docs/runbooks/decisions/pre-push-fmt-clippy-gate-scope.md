# Decision: Pre-push fmt/clippy gate — scope is "both" (local hook + CI mirror)

| Status | Accepted |
| --- | --- |
| Date | 2026-06-17 |
| Area | DevOps / CI |
| Owners | pm-devops |
| Affected files | `scripts/pre-push`, `.github/workflows/backend-fmt-clippy-gate.yml`, `.github/workflows/backend-dev-push-gate.yml`, `.github/workflows/branch-protection-setup.yml` |

## Context

PR #1431 (gh-1375) added a **local** pre-push git hook (`scripts/pre-push`) that
runs the documented backend pre-flight:

```bash
cargo fmt --all -- --check && cargo clippy --workspace -- -D warnings
```

A local hook is fast feedback, but it has two structural holes:

1. **Bypassable.** `git push --no-verify` skips it entirely, and a contributor
   who never installed the hook (it lives in `scripts/`, not `.git/hooks/`
   unless `core.hooksPath`/`setup.sh` wired it) never runs it at all.
2. **Never runs on the merge commit.** Even when honoured, the hook only checks a
   developer's local tip — not the state of `dev` after the merge.

Those holes are not theoretical. The **#1426 → #1437 episode** is the motivating
incident: a fmt/clippy- and compile-affecting change reached `dev` despite the
local-only hook, and the breakage was then discovered one downstream PR at a
time rather than on the offending merge. Local-only enforcement did not catch
it because nothing on the GitHub side re-ran the gate on the merge path.

The question this decision settles:

> Should the pre-push fmt/clippy gate be **local hook only**, **mirrored as a CI
> status check**, or **both**?

## Decision

**Both.** Keep the local hook for instant feedback *and* mirror it as an
unbypassable CI status check on the merge path. The two are defence-in-depth,
not redundancy:

| Layer | Where it runs | Trigger | Bypassable? | Purpose |
| --- | --- | --- | --- | --- |
| `scripts/pre-push` (#1431) | Developer machine | `git push` | Yes (`--no-verify`, or not installed) | Instant local feedback — catch lint debt before it ever leaves the laptop |
| `backend-fmt-clippy-gate.yml` (#1455) | GitHub Actions | `pull_request` on `backend/**` | No | Unbypassable lint gate on the merge path — the safety net the local hook can't be |

"Local-only" is rejected because it is exactly the configuration that let
#1426 → #1437 through. "CI-only" is rejected because it throws away the
fast-feedback value of catching a `cargo fmt` miss before pushing — a CI round
trip for a one-character whitespace fix is wasteful.

### What the CI mirror does (already implemented)

The CI half of this decision is already wired and merged — **no new workflow is
required by this ADR.** `backend-fmt-clippy-gate.yml` (PR #1455):

- runs on `pull_request` for `backend/**` (and self-path edits) so feedback
  lands **before** merge;
- runs `cargo fmt --all -- --check` then
  `cargo clippy --workspace --all-targets -- -D warnings` — note `--all-targets`,
  which the local hook omits, so the CI gate is the **stricter** mirror and
  matches the documented `cargo clippy --workspace --all-targets -- -D warnings`
  pre-flight in `CLAUDE.md`;
- uses `SQLX_OFFLINE=true` to stay DB-free and fast;
- is a small, independently-named status check so a fmt/clippy break surfaces in
  seconds as a dedicated red check rather than buried inside the heavyweight
  `backend.yml` `check` job.

### Relationship to the adjacent gates (do not duplicate)

Three backend gates exist and are deliberately distinct:

- **`backend-fmt-clippy-gate.yml`** (this decision's CI half) — *lint* (fmt +
  clippy), fires on **pull_request**, lands before merge.
- **`backend-dev-push-gate.yml`** (#1452) — *compile* smoke
  (`cargo check --workspace --tests`), fires on **push to `dev`**, catches a
  non-compiling merge (including test code) on dev's tip. This is the other half
  of the #1426 → #1437 fix: `backend.yml`'s `cargo check --workspace` does **not**
  compile test targets, so a test-only compile break slipped past it.
- **`backend.yml`** `check` job — the thorough PR gate (fmt + clippy + RLS
  enforcement scripts + Docker-Postgres boot + `cargo check`). Correct but slow;
  the dedicated lint gate gives faster, isolated signal.

## Consequences

- The local hook stays as-is. No change to `scripts/pre-push`.
- Backend PRs now get a fast, unbypassable lint signal independent of the slow
  `backend.yml` job.
- Documented pre-flight (`cargo fmt && cargo clippy` in `backend/`) is now
  enforced on the server side, so a `--no-verify` push or an un-installed hook no
  longer lets lint debt reach `dev`.

## Follow-up (open gap)

`backend-fmt-clippy-gate.yml` is currently **advisory**: a red check is visible
but does not *block* merge unless it is registered as a **required status check**
in the `dev` branch-protection rule. Today `branch-protection-setup.yml` only
registers `security-gate-conclusion`. To make this gate truly unbypassable, add
its check context (`fmt-clippy`, the job name in `backend-fmt-clippy-gate.yml`)
to the required-status-checks list via that workflow. Tracking this as a
separate pm-devops task keeps the present change tight (doc + decision only) and
avoids editing live branch protection from an automated PR.
