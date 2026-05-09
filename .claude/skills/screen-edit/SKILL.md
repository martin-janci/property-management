---
name: screen-edit
description: Load focused context for a single screen-map (parent + children + related + sitemap entries + recent agent log + optional Playwright screenshot). Use when the user runs `/screens edit <id>` or asks to "load context for screen X", "show me ppt/foo", "what's the current state of <screen>".
---

# Screen-Edit Skill

Pulls everything an agent needs to work on a single screen-map into a markdown summary printed in chat. Avoids hunting for context across files.

## When to use

- User invokes `/screens edit <id>` (e.g. `/screens edit ppt/building-detail`).
- User asks "load <screen>", "show me ppt/foo", "what's the state of reality/property-detail".
- Before making any change to a screen-map, to load context.

## Inputs

- `<id>` (required) — the screen-map id (`<product>/<slug>`).
- `--root <path>` (optional; defaults to repo root).
- `--playwright` (optional; capture a screenshot if the local app is running).

## Implementation

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
pnpm -C "$REPO_ROOT/frontend" --filter @ppt/screen-map cli edit "$ID" \
  --root "$REPO_ROOT" \
  ${PLAYWRIGHT:+--playwright}
```

## Output handling

- Print the CLI's markdown summary verbatim into chat.
- Then offer next-step actions:
  - "Edit this file?" → opens the screen-map markdown for editing.
  - "Run `/screens validate`?" → verifies cross-references.
  - "Run Playwright?" → if `--playwright` was not supplied initially.
