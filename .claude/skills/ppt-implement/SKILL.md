---
name: ppt-implement
description: Implement one task end-to-end for the PPT project — pick the right per-stack specialist, code, verify per-stack, push a draft PR against `dev`.
when_to_use: You are an implementer agent (spawned by the ppt-research-dispatcher routine or hand-invoked) and you have ONE task to land. The dispatcher gives you task text; you select which specialist runs and which verify command to use before opening a PR.
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

## Step 3 — Verify gate (MANDATORY before PR)

| Specialist     | Minimum verify command(s)                                                                    |
| ---            | ---                                                                                          |
| `rust-backend` | `cd backend && cargo check -p <crate>` then `cargo test -p <crate> -- <filter>`              |
| `db-migration` | `cd backend && cargo sqlx prepare --check` + `cargo test -p db <pattern>`                    |
| `typespec`     | `cd docs/api/typespec && npx tsp compile .` then `pnpm -F @ppt/api-client build`             |
| `react-web`    | `pnpm -F ppt-web typecheck` then `pnpm -F ppt-web lint`                                      |
| `nextjs-web`   | `pnpm -F reality-web typecheck` then `pnpm -F reality-web lint`                              |
| `react-native` | `pnpm -F mobile typecheck` then `pnpm -F mobile lint`                                        |
| `kotlin-mp`    | `cd mobile-native && ./gradlew :shared:compileKotlinJvm :androidApp:assembleDebug`           |
| `ios-swiftui`  | `cd mobile-native && ./gradlew :shared:linkPodReleaseFrameworkIosArm64` (compile only)       |
| `generic`      | `git diff --check` + `pre-commit run --files <changed>` if `.pre-commit-config.yaml` exists  |

Quote command + exit code in the PR body under `## Tested`. If any verify
command fails: do NOT open a PR. Push partial work to the branch and return
`pr=none status=partial note=<command-that-failed>`.

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

## Tested
<command>
<short output / exit 0>

## Remote-tested
<paste short output, or "skipped">

## Notes
<anything the reviewer must know>
EOF
)"
```

## Return contract (ONE LINE — dispatcher parses this)

```
pr=<number|none> status=<done|partial|blocked> specialist=<name> note=<short text>
```

Examples:
- `pr=512 status=done specialist=react-web note=wired useMfa hooks; ppt-web typecheck clean`
- `pr=none status=partial specialist=rust-backend note=cargo check failed on websocket route stub`
- `pr=none status=blocked specialist=react-web note=needs backend /api/v1/auth/mfa/* first`

The dispatcher writes this verbatim into `.research/management/assignments.json`
under `implementer_summary`, and sets `pr_number` / `pr_url` if `pr=<n>`.

## Install (local user)

```bash
bash .claude/skills/ppt-implement/install.sh
```

Copies the skill to `~/.claude/skills/ppt-implement/`. Re-run after pulling
updates to refresh the local copy.

The cloud routine reads this skill directly from the repo checkout — no
install needed there.
