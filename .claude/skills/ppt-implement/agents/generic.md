# Specialist: generic (fallback)

Use when no other specialist scored ≥+3 — typically: docs-only changes,
top-level config (`.gitignore`, `.editorconfig`, `package.json` workspace
edits), CI tweaks, shell scripts, screen-map markdown.

## You own
- `docs/**/*.md` (except `docs/api/typespec/` → `typespec` specialist)
- `docs/screens/**/*.md` (screen-map self-management; see CLAUDE.md)
- `.github/workflows/*.yaml` — CI changes (handle with care)
- Top-level config files
- Repo-wide scripts under `scripts/`

## Conventions
- Markdown: GitHub-flavored; one sentence per line in long-form prose (better diffs).
- Screen-map frontmatter is schema-validated by `/screens validate` — match existing fields.
- CI changes: small, reversible; never disable a passing check without explicit issue/PR justification.
- Scripts: `#!/usr/bin/env bash` + `set -euo pipefail`. Quote variables.

## Step-by-step
1. Make the smallest possible change.
2. If a config change might affect builds: run the relevant per-stack verify (yes, even from the generic specialist).
3. If touching `docs/screens/`: run `pnpm screens validate` (or the documented validate command).

## Verify (MANDATORY)
```bash
git diff --check                               # whitespace / merge-marker hygiene
# If .pre-commit-config.yaml exists:
pre-commit run --files $(git diff --name-only HEAD~1)
# If you touched docs/screens/:
pnpm screens validate     # or the documented equivalent
# If you touched .github/workflows/:
# - Run `actionlint .github/workflows/*.yaml` if available, otherwise note "syntax-only check" in PR body
```
Quote at least one verify command's exit code.

## Common pitfalls
- Touching CI without local validation → broken main for everyone.
- Editing generated files (openapi.yaml, sqlx offline data) here instead of in the responsible specialist → next regeneration wipes the change.
- Adding a screen-map markdown without matching frontmatter fields → validate fails.

## Return-line examples
- `pr=520 status=done specialist=generic note=updated docs/screens/ppt/announcements.md buildStatus; validate clean`
- `pr=none status=partial specialist=generic note=actionlint flagged 2 issues in deploy.yaml — left as TODO`
