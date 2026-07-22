---
name: ppt-implement
description: Implement one task end-to-end for the PPT project — pick the right per-stack specialist, code, verify per-stack, push a draft PR against `dev`.
when_to_use: You are an implementer agent (spawned by the ppt-research-dispatcher routine or hand-invoked) and you have ONE task to land. The dispatcher gives you task text; you select which specialist runs, then `just verify` (scripts/verify-impact.sh) gates the PR automatically.
mode: both
capabilities: [C6]
tags: [workflow, implementer]
---

# PPT Implement

Dispatcher + specialist roster for shipping one task as a draft PR against
`dev`. Verification is mandatory before the PR is opened. Never ship red.

## When to invoke

You are an implementer agent — either spawned by `ppt-research-dispatcher`
or hand-invoked by a developer who passes you a task description. You have
one task and one branch to land.

## Inputs

| Input          | Source                              | Example                                |
| ---            | ---                                 | ---                                    |
| `task_id`      | action-list.json `id`               | `gap-9-1-mfa-frontend-integration`     |
| `action`       | action-list.json `action`           | "Wire TwoFactorAuthPage to /api/v1/auth/mfa/* …" |
| `owner_role`   | hint, not authoritative             | `pm-security`                          |
| `priority`     | informational                       | `high`                                 |
| `dependency`   | informational                       | `Epic 2B WebSocket infrastructure`     |
| `branch`       | pre-chosen by dispatcher            | `auto-impl/gap-9-1-mfa-frontend-...`   |

## Step 1 — Pick a specialist (deterministic)

Score each specialist; pick the highest. Tie-break by `owner_role` hint.
Specialist prompts live in `agents/<name>.md` — load and follow that file.

| Signal in `action` text / paths / known repo layout                           | Specialist     | Weight |
| ---                                                                            | ---            | :---:  |
| `backend/crates/db/migrations/`, "migration", "schema", "RLS"                 | `db-migration` | +4     |
| `docs/api/typespec/`, "TypeSpec", "OpenAPI", "endpoint contract"              | `typespec`     | +4     |
| `mobile-native/iosApp/`, "SwiftUI", "Info.plist", "xcconfig"                  | `ios-swiftui`  | +4     |
| `mobile-native/` (not iosApp), "KMP", "Compose", "gradle", "Kotlin Multiplatform" | `kotlin-mp` | +3 |
| `frontend/apps/mobile/`, "React Native", "Expo", "RN"                         | `react-native` | +3     |
| `frontend/apps/reality-web/`, "Next.js", "SSR", "ISR", "next-intl"            | `nextjs-web`   | +3     |
| `frontend/apps/ppt-web/` or `admin-web/`, "TanStack", "Vite", "useQuery", "useMutation" | `react-web` | +3 |
| `backend/`, `routes/`, `handlers/`, `services/`, "axum", "cargo", "rust"      | `rust-backend` | +3     |
| `owner_role=pm-frontend` and no other ≥+3 hit                                 | `react-web`    | +1     |
| `owner_role=pm-backend` and no other ≥+3 hit                                  | `rust-backend` | +1     |
| `owner_role=pm-security` and no other ≥+3 hit                                 | `rust-backend` | +1     |
| nothing matches                                                                | `generic`      | 0      |

**Cross-stack actions:** if `action` clearly needs both a frontend AND a backend
slice (e.g. "wire X to new /api/Y endpoint"), prefer the *frontend* specialist
and let it add the backend client/types only. If the backend route itself is
missing, the specialist must say so in its return line and return
`status=blocked dep=missing-backend` so the dispatcher creates a follow-up
backend task instead of merging a half-PR.

## Step 2 — Implement on the branch

1. From `dev` (assumes `git fetch && git pull --ff-only`), create the branch:
   `git checkout -b <branch> dev`
2. Follow the chosen specialist's `agents/<name>.md` for project layout, file
   patterns, and verify commands.
3. Commit message format (CLAUDE.md convention):
   `<type>(<scope>): <one-line>` where type ∈ {feat, fix, docs, style, refactor, test, chore}.

## Step 2.5 — Scope-drift check (deterministic, MANDATORY)

The dispatcher saw on 2026-05-24 that implementers can quietly touch code
outside their owner_role's area (PR #472 dropped `RlsConnection` on 5 MFA
handlers while the task was about WebSocket sync). The check below is
deterministic — file globs map directly to `owner_role` from
`scripts/owner-areas.json`.

```bash
bash .claude/skills/ppt-implement/scripts/scope-check.sh \
  --owner "<owner_role>" --base dev --head HEAD
```

Exit codes:
- `0` — diff stays inside the owner's expected areas. `scope_drift=false`.
- `2` — diff touches files outside the owner's areas. The script prints the
  drifting paths. **Do not abort.** Set `scope_drift=true` and include the
  offending paths in the PR body under `## Scope drift`. The reviewer
  decides whether the drift is justified.
- `1` — unrecognised owner_role (script does not know how to map it).
  `scope_drift=false` (we can't judge) — log the unknown owner and continue.

## Step 2.5b — Commit-scope guard (MANDATORY for stop-hook / auto-commit paths)

The scope-drift check above is **advisory** — it tags off-area changes but
lets the implementer keep going (the reviewer adjudicates). That contract
is right for the normal interactive commit path but wrong for the
**automated** commit path: when the dispatcher's stop hook fires while
parallel agents share a worktree, an "advisory" tag isn't enough — the
hook will go ahead and bundle the off-area files into whichever branch
happens to be checked out. PR #496 (2026-05-25) auto-committed 1088
lines of gap-85-1 + gap-85-2 work onto an ADR-008 doc-edit branch
exactly this way (see #526).

The fix is `commit-scope-guard.sh`, a fail-closed sibling of the
advisory `scope-check.sh`:

```bash
# Stop-hook style (before `git add`, includes untracked):
bash .claude/skills/ppt-implement/scripts/commit-scope-guard.sh \
  --owner "<owner_role>" --worktree
# OR for tasks that don't map cleanly to an owner role, pass an
# explicit allow-list:
bash .claude/skills/ppt-implement/scripts/commit-scope-guard.sh \
  --allow 'docs/architecture.md' \
  --allow '.research/management/**'
```

Exit codes:
- `0` — every changed path is inside the allow-list. Safe to commit.
- `1` — unknown owner_role. **Fail-closed**: an automated codepath
  with no human in the loop should NOT guess scope. Skip the
  auto-commit and surface the work for human resolution.
- `2` — at least one path is outside scope. The guard prints the
  offending paths plus a remediation hint (`git stash push -- <path>`
  or `git reset HEAD <path>`) on stderr. The caller MUST NOT commit
  — instead, stash the off-scope paths, commit the rest, and surface
  the stash ref in the dispatcher log so the next agent picks them up.
- `64` — usage error.

The companion `test-commit-scope-guard.sh` covers the PR #496
regression scenario (ADR doc edit + parallel-agent mobile env-config
+ iOS xcscheme — expected: REFUSE) plus four other fixtures. Run it
locally after any change to the guard or owner-areas.json:

```bash
bash .claude/skills/ppt-implement/scripts/test-commit-scope-guard.sh
```

## Step 2.6 — Code-reuse pre-flight (deterministic, advisory)

The same PR #472 also re-implemented `JWT_VERIFIER` as a private `OnceLock`
duplicating `api_core::extractors::auth::JWT_VERIFIER`. Before adding any
new helper in security/auth/crypto/http-client code paths, grep for an
existing one:

```bash
bash .claude/skills/ppt-implement/scripts/reuse-grep.sh \
  --specialist "<specialist>" --base dev --head HEAD
```

Output: a list of `WARN: <added-symbol> looks like an existing helper at <path>`
lines, or empty.

This is **advisory** — the implementer decides. If the script emits warnings,
set `code_reuse_warn=<short-comma-list>` (max 80 chars; truncate) and include
the full list in the PR body under `## Code-reuse review`. If empty,
`code_reuse_warn=none`.

## Step 3 — Verify gate (MANDATORY before PR)

Run the deterministic gate:

```bash
just verify        # or: ./scripts/verify-impact.sh
just verify-plan   # print the plan without running it
```

**Band selection is automatic** — `scripts/verify-impact.sh`'s escalation
table derives the scope from (merge-base with `origin/dev`, changed paths):
per-crate clippy+tests with reverse dependents for narrow backend diffs,
`pnpm --filter "...[<merge-base>]"` for frontend diffs, full-workspace only
when lockfiles / toolchain / migrations / typespec force it, empty plan for
docs-only diffs. There is no judgment call and no skipping. Guardrails:
[`.claude/skills/_verify-rules.md`](../_verify-rules.md) — notably: no
`cargo build`/`--release` locally, no `cargo check` (clippy is the type
gate), don't re-run an unchanged failing command, flock serializes cargo
across agents on the host.

CI-parity ("Band C") extras apply ONLY when the task explicitly flags
hot-path (security, billing, auth, data integrity): in that case, after
`just verify` is green, additionally replicate the relevant workflow job
(`gh workflow view backend --yaml`, or `act -j <job> pull_request` if
installed) and report it under `## CI parity` in the PR body.

Narrow inner-loop commands during development (`cargo clippy -p <crate>`,
`cargo test -p <crate> -- <filter>`, `pnpm -F <pkg> test`,
`./gradlew :shared:test`, `npx tsp compile .`) remain encouraged — they
just don't substitute for the gate.

### Verify reporting

Paste the `VERIFY-PLAN base=<sha> files=<n>` block and the
`VERIFY OK <hash>` line verbatim in the PR body under `## Verification`.
If the gate fails: do NOT open a PR. Push partial work to the branch
and return `pr=none status=partial note=<the VERIFY FAIL line>`.

## Step 3.5 — IG1–IG8 goal-check (deterministic, MANDATORY before draft PR)

Mirror of the IG1–IG8 success criteria from `.research/implementer-prompt.md`,
extracted into a single runnable script so the implementer doesn't have to
grep the prompt for snippets. Run after Step 3's verify gate and before
`gh pr create`:

```bash
bash .claude/skills/ppt-implement/scripts/goal-check.sh \
  --slug "$SLUG" --base dev \
  --skip IG7        # IG7 = just verify — Step 3 already ran it
  # Add --skip IG8 on pre-final commits (archive move is intentionally the
  # LAST commit before merge). Add --skip IG3 only for vectors that
  # legitimately don't require a regression test (e.g. pure docs/refactor —
  # see implementer-prompt.md IG3 trigger list).
```

Exit codes:
- `0` — all hard checks passed (warnings allowed). Proceed to Step 5.
- `1` — at least one hard check failed. **Do not open a ready-to-review PR.**
  Either fix the goal failure (preferred) or push partial work and return
  `pr=none status=partial note=<which-IG-failed>`.

Each check prints a T-style line so the output is grep-friendly. Findings
fall into three buckets:

- `ok` — verifiable pass.
- `WARN` — advisory: the check needs human confirmation (e.g. IG4 asks
  whether each Suggested-approach step is addressed in the PR body —
  the script can't see the PR body until after `gh pr create`).
- `FAIL` — hard fail. Exit 1.

The script is intentionally narrower than the verify gate: it checks
goal-shape (test:+fix: commit pair, archive move, OOS path respect) rather
than build/test correctness. Step 3 owns the latter.

## Step 4 — (Optional) Remote verify

If the `ppt-bridge` MCP is connected in your session (cloud routine may have
it; local dev usually doesn't), you may run the relevant command against the
mefistos/hetzner dev-stack via `mcp__ppt-bridge__*` tools. Log under
`## Remote-tested` in the PR body. Default behaviour: skip silently. Do not
gate the PR on remote checks.

## Step 5 — Open draft PR

```
gh pr create \
  --base dev --head <branch> --draft \
  --title "<task_id>: <one-line>" \
  --body "$(cat <<'EOF'
## Task
<action verbatim>

## Owner
<owner_role>

## Verification
<paste the VERIFY-PLAN base=<sha> files=<n> block verbatim>
<paste the VERIFY OK <hash> line>

## CI parity (Band C — hot-path only)
<command(s) and exit codes — or "skipped: not a hot-path PR (no security/auth/billing/data-integrity change)">

## Remote-tested
<paste short output if ppt-bridge MCP was used, else "skipped">

## Notes
<anything the reviewer must know>
EOF
)"
```

## Return contract (ONE LINE — dispatcher parses this)

```
pr=<number|none> status=<done|partial|blocked> specialist=<name> scope_drift=<true|false> code_reuse_warn=<short|none> note=<short text>
```

Examples:
- `pr=512 status=done specialist=react-web scope_drift=false code_reuse_warn=none note=wired useMfa hooks; ppt-web typecheck clean`
- `pr=513 status=done specialist=rust-backend scope_drift=true code_reuse_warn=JWT_VERIFIER@api_core note=ws sync (drift: handlers/mfa.rs touched; verifier may duplicate)`
- `pr=none status=partial specialist=rust-backend scope_drift=false code_reuse_warn=none note=VERIFY FAIL: cd backend && cargo clippy -p api-core --all-targets -- -D warnings`
- `pr=none status=blocked specialist=react-web scope_drift=false code_reuse_warn=none note=needs backend /api/v1/auth/mfa/* first`

The dispatcher writes this verbatim into `.research/management/assignments.json`
under `implementer_summary`, parses the `scope_drift` and `code_reuse_warn`
fields into their own columns, and sets `pr_number` / `pr_url` if `pr=<n>`.

## Install (local user)

```bash
bash .claude/skills/ppt-implement/install.sh
```

Copies the skill to `~/.claude/skills/ppt-implement/`. Re-run after pulling
updates to refresh the local copy.

The cloud routine reads this skill directly from the repo checkout — no
install needed there.
