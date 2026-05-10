---
name: screen-render
description: Generate Mermaid diagrams from the screen-map tree — site graph (screens × related-screen edges), endpoint matrix (screens × endpoints), status dashboard (per-platform pie charts of build/redesign/api status). Output to docs/screens/_diagrams/. Use when user runs `/screens render` or asks for "screen-map diagrams", "visualize screens", "status dashboard".
---

# Screen-Render Skill

Generates three diagrams from the current screen-map tree:

- **Site graph** — Mermaid `graph TD` showing screens and their related-screen edges (parent/child/action/sibling).
- **Endpoint matrix** — markdown table: screens × endpoints, `✓` cells where the screen consumes that endpoint.
- **Status dashboard** — Mermaid `pie` charts per platform, one per axis (build / redesign / api).

## When to use

- User invokes `/screens render` (with or without `--scope`).
- User asks "draw the screen graph", "show me the status pie", "endpoint matrix".

## Inputs

- `--root <path>` (defaults to repo root).
- `--scope <name>` (`product` like `ppt` or `reality`, or `all`; default `all`).
- `--out <path>` (output dir; default `docs/screens/_diagrams`).

## Implementation

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
pnpm -C "$REPO_ROOT/frontend" --filter @ppt/screen-map cli render \
  --root "$REPO_ROOT" \
  ${SCOPE:+--scope "$SCOPE"} \
  ${OUT:+--out "$OUT"}
```

## Output handling

- The CLI prints `wrote <path>` for each generated file.
- Reply with the three relative paths and offer to open them or render them inline (Mermaid blocks paste directly into GitHub markdown).
- If `docs/screens/<product>/` is empty, the diagrams will be near-empty too — note that `screen-map-init` should run first.
