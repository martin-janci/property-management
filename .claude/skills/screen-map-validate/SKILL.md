---
name: screen-map-validate
description: Validate every screen-map under docs/screens/ against frontmatter schema, @ppt/sitemap IDs, related-screen IDs, and diagram refs. Use when the user runs `/screens validate` or asks to check screen-map consistency. Triggers also include "validate screens", "screen-map drift", "is the screen-map clean".
---

# Screen-Map Validate Skill

Runs the `@ppt/screen-map` CLI in validate mode against the repo's `docs/screens/` tree.

## When to use

- User invokes `/screens validate` (with or without `--strict`).
- User asks to "validate the screen-map", "check screen-map consistency", or to confirm screen-maps are not stale.
- After editing any frontmatter in `docs/screens/<product>/*.md`.

## Inputs

- Optional `--strict` flag (forwarded to the CLI; non-zero exit on any error).
- Optional `--root <path>` flag (forwarded; defaults to repo root).

## What it does

1. Resolves the repo root (`git rev-parse --show-toplevel`).
2. Runs `pnpm --filter @ppt/screen-map cli validate --root <repoRoot> [--strict]`.
3. Reports the number of files validated and any issues.
4. If issues are returned, summarises them per file with `path :: message`. Offers to open the offending file.

## Implementation

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
pnpm -C "$REPO_ROOT/frontend" --filter @ppt/screen-map cli validate --root "$REPO_ROOT"
```

## Output handling

- All-clean → reply: "Screen-map clean: validated N screen-maps."
- Has errors → echo the CLI output verbatim, then ask the user whether to open the first failing file for editing.
- `--strict` exits non-zero — surface that to the caller (the slash command will pass it along).
