# `just verify` Gate — Adoption Check-in (2026-07-24)

**Owner lens:** pm-qa · **Subject:** the impact-scoped verify gate introduced in #2444
(`just verify` → `scripts/verify-impact.sh`).

## Question

Has every open PR run `just verify` locally before pushing? Is the gate **enforced**
or **advisory-only**?

## Finding — the gate is ADVISORY-ONLY

`just verify` is a documented convention, not an enforced status check. Nothing in CI
or in a required git hook runs `scripts/verify-impact.sh` or checks a PR for its output.

| Enforcement surface | Runs `verify-impact.sh` / checks `VERIFY OK`? | Notes |
| --- | --- | --- |
| **CI** (`.github/workflows/*`) | **No** | No workflow invokes `just verify` / `scripts/verify-impact.sh`, and none greps the PR body for a `VERIFY-PLAN` / `VERIFY OK` block. CI runs its own independent per-stack gates (`backend.yml`, `frontend.yml`, `mobile-native.yml`, `backend-dev-push-gate.yml`) whose coverage overlaps the verify gate but is separate from it. |
| **Git hooks** (`scripts/pre-commit`, `scripts/pre-push`) | **No** | Both run their own ad-hoc checks (pre-commit: fmt / spotless / biome / typecheck; pre-push: backend fmt + clippy, path-gated). Neither calls `verify-impact.sh`. Not installed by default (require `scripts/install-hooks.sh`) and bypassable with `git commit/push --no-verify`. |
| **Agent skills** (`ppt-implement`, `ppt-pr-create`) | **Yes (convention)** | The skills require running `just verify` and pasting the `VERIFY-PLAN base=… files=…` + `VERIFY OK <hash>` block into the PR body. This is the *only* place the gate is exercised — and only for agent-authored PRs that go through the skill. |

Net: the gate binds agent-authored PRs by convention; human-authored and Dependabot PRs
are not gated by it at all.

## Per-PR check-in (the five named PRs)

| PR | Kind | State | Verify-gate evidence in body |
| --- | --- | --- | --- |
| **#2490** | backend security IDOR (agent, `ppt-implement`) | merged | **Yes** — full `VERIFY-PLAN base=… files=4` + per-step clippy/test results. Canonical evidence. |
| **#2478** | layout hardening (human-authored, `martin-janci`) | open | **No canonical block.** Has a hand-written "Verification" section describing `cargo clippy`/tests + `pnpm check`/typecheck run manually — not the impact-scoped gate, no `VERIFY OK` line. |
| **#2482** | docs-only repo-map tidy (agent) | open | **No** — body explicitly states *"the `just verify` / `scripts/verify-impact.sh` gate is not present at this `dev` snapshot"* and falls back to `git diff --check`. |
| **#2481** | docs-only screen-map tidy (agent) | merged | **No** — uses the Band A/B/C convention (`pnpm … validate`), not the verify gate. Docs-only → empty plan anyway. |
| **#2491** | Dependabot npm-minor bump | open | **No** — automated Dependabot PR; no verify evidence, not routed through any skill. |

So **1 of 5** carries the canonical `VERIFY OK` evidence. The rest either ran equivalent
checks by hand, are docs-only (empty verify plan), or are automated dependency bumps.
This is consistent with an advisory gate: adherence tracks *who/what* authored the PR,
not a required check.

## Recommendation

Make the existing, already-deterministic gate an actual required check. In rough
order of value / effort:

1. **Run `scripts/verify-impact.sh` as a required CI status check on PRs to `dev`.**
   It is already deterministic and impact-scoped (empty plan → exit 0 for docs-only),
   so cost on docs/dep PRs is ~nil. Add it to branch protection's required checks. This
   converts advisory → enforced for *every* author (human, agent, Dependabot) with one
   entry point, rather than relying on N overlapping per-stack workflows plus a
   convention. (Marginal *coverage* over today's CI is small; the win is single-source
   determinism + closing the human/Dependabot gap.)
2. **Cheaper interim:** a tiny PR-lint job that requires a `VERIFY OK <hash>` line in the
   body for non-trivial diffs (skip for `dependencies`/`docs`-labelled PRs). Enforces the
   *evidence* convention without spending CI compute, but is spoofable (author can paste a
   stale hash) — strictly weaker than option 1.
3. **Install the git hooks by default** (or document that `scripts/install-hooks.sh` is
   mandatory in `setup.sh`) so local pre-push at least *mentions* `just verify`. Weakest —
   hooks are `--no-verify`-bypassable and local-only.

Prefer **option 1**: it reuses `verify-impact.sh` as-is, is not bypassable, and treats
every author uniformly.

## Scope note

This is a QA process check-in, not a code change. No enforcement was implemented here to
keep the change low-risk and reviewable; option 1 above is the suggested follow-up task
(add `verify-impact.sh` to `dev` branch protection + a CI job).
