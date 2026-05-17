# .claude/skills/

In-repo, versioned skills the **implementer agent** discovers when it walks
a plan. Each subdirectory is one skill with a `SKILL.md` file using the
same frontmatter format as `~/.claude/skills/*/SKILL.md`.

This directory is auto-discovered by any Claude Code session opened in this
repo (local CLI or cloud routine — the routine clones the repo, so the
skills come with it). Plans, briefs, and state still live in `.research/`.

## Discovery

At session start, the implementer should:

```bash
ls .claude/skills/
```

Then `Read` the `SKILL.md` for each skill whose `tags` or `capabilities`
match the plan's *Required capabilities* section and area.

## Index

| Skill | Mode | Tags | Purpose |
|---|---|---|---|
| [`ppt-research-flow`](ppt-research-flow/SKILL.md) | both | workflow | End-to-end plan → PR → archive choreography |
| [`ppt-research-trigger`](ppt-research-trigger/SKILL.md) | both | workflow, infra | Fire `deep` / `reset` routine triggers via the API |
| [`ppt-next-plan`](ppt-next-plan/SKILL.md) | both | workflow | Pick the top `status: ready` plan from `backlog.json` |
| [`ppt-bridge-mcp`](ppt-bridge-mcp/SKILL.md) | cloud-ok | infra, workflow | Use the ppt-bridge MCP from cloud routines |
| [`ppt-tests`](ppt-tests/SKILL.md) | both | workflow | Pick the right test command per change |
| [`ppt-pr-create`](ppt-pr-create/SKILL.md) | both | workflow | Open a PR in this project's style |
| [`ppt-rust-backend`](ppt-rust-backend/SKILL.md) | both | backend | Cargo workspace navigation |
| [`ppt-frontend`](ppt-frontend/SKILL.md) | both | frontend | pnpm workspace + Vite/Next apps |
| [`ppt-mobile-native`](ppt-mobile-native/SKILL.md) | local | mobile | Kotlin / Gradle / Compose (local-only via ADB) |
| [`ppt-typespec`](ppt-typespec/SKILL.md) | both | backend, frontend, infra | TypeSpec contract authoring |
| [`ppt-dev-stack`](ppt-dev-stack/SKILL.md) | local | infra | `stack up pm-local` declarative dev stack |
| [`ppt-db-migrations`](ppt-db-migrations/SKILL.md) | both | backend, infra | SQLx migrations + seed gap |
| [`ppt-deploy`](ppt-deploy/SKILL.md) | both | infra, deploy | `pmctl` worktree + staging/prod deploys (predates research routine; see IMPROVEMENT_IDEAS for frontmatter normalization) |

## Verifying the environment

Each skill has a *Smoke check* (sub-30s, exit 0 when the skill's tooling is
present on the host). Run them all in order:

```bash
./.claude/skills/verify-all.sh
```

Exits non-zero if any smoke check fails. Use this as the "environment is
ready to implement anything on this repo" gate.

Some smoke checks may legitimately fail on a given host — e.g.
`ppt-mobile-native` fails if no JDK / no emulator is present, which is
fine if the implementer isn't shipping mobile-native plans from that
host. Read the skill's own *Smoke check* note for what counts as
acceptable failure.
