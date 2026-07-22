# Verify rules (shared guardrails for all PPT skills)

Canonical rules for local verification. Referenced by `ppt-implement`,
`ppt-tests`, `ppt-rust-backend`, `ppt-frontend`, `ppt-pr-create`, and
`.research/implementer-prompt.md`. The gate itself is
`scripts/verify-impact.sh` (`just verify` / `just verify-plan`); its scope
is a pure function of the merge-base with `origin/dev`, the sorted changed
paths, and the escalation table documented in the script header.

1. **One gate.** `just verify` is the ONLY pre-PR verify. Never hand-compose
   full-workspace commands (`just check && just test && just build`,
   `cargo clippy --workspace`, `pnpm test` at the root, …) as a substitute —
   the escalation table decides scope, not judgment.
2. **Never `cargo build --workspace` or any `--release` build locally.**
   Test compilation already links the binaries that matter; release-only
   failures are caught on main-push CI. There is no `cargo check` step either
   — clippy is the type gate (check and clippy don't share artifacts).
3. **Don't re-run an unchanged failing command.** If a command in the plan
   fails and your diff hasn't changed since, report `status=partial` with the
   `VERIFY FAIL: <cmd>` line instead of burning another compile on the same
   result.
4. **One heavy compile per host.** Every cargo invocation in the gate is
   wrapped in `flock /tmp/ppt-cargo.lock`. If you run cargo outside the gate
   (inner loop), prefer the same lock for anything heavier than a single-crate
   command; never run two workspace-wide compiles concurrently on one host.
5. **Narrow inner-loop commands are allowed and encouraged**
   (`cargo clippy -p <crate>`, `cargo test -p <crate> -- <filter>`,
   `pnpm -F <pkg> test`, `./gradlew :shared:test`) — but the gate is NOT
   skippable before `ppt-pr-create`. Inner-loop green is not gate green.
6. **Quote the plan + OK hash in the PR body, not logs.** Paste the
   `VERIFY-PLAN base=<sha> files=<n>` block and the `VERIFY OK <hash>` line
   verbatim. Reviewers and the goal gate parse these lines; log dumps are
   noise.
7. **AI review sits on top of green deterministic gates, never replaces
   them.** A reviewer verdict (human or agent) does not waive a red
   `just verify`; conversely a green gate does not waive review.
