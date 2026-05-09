---
name: screen-query
description: Query the screen-map tree by frontmatter filter (e.g. "product:ppt,implementations.ppt-web.redesignStatus:in-progress"). Output as table, JSON, or markdown. Use when user runs `/screens query` or asks "which screens are X", "list all Y screens", "find screens with Z".
---

# Screen-Query Skill

Read-only query against `docs/screens/<product>/*.md` frontmatter using the comma-AND key:value filter syntax (same as `screen-map-review --filter`).

## When to use

- User invokes `/screens query <expr>`.
- User asks "which screens are shipped but not redesigned", "list reality screens", "find screens using endpoint X".

## Inputs

- `<expr>` (positional; optional). Empty → returns all. Examples:
  - `product:ppt`
  - `implementations.ppt-web.redesignStatus:in-progress`
  - `product:ppt,implementations.ppt-web.buildStatus:shipped`
- `--root <path>` (defaults to repo root).
- `--format <fmt>` (`table` default, `json`, `md`).

## Implementation

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
pnpm -C "$REPO_ROOT/frontend" --filter @ppt/screen-map cli query "$EXPR" \
  --root "$REPO_ROOT" \
  ${FORMAT:+--format "$FORMAT"}
```

## Output handling

- Echo CLI output verbatim. Final line shows count: `N screen-maps matched.`
- For `--format=md` the output is paste-friendly; for `--format=json` it's machine-readable.

## Common queries

| Goal | Expression |
|---|---|
| All PPT screens | `product:ppt` |
| Mobile screens still in progress | `implementations.mobile.buildStatus:in-progress` |
| Reality screens with redesign applied | `product:reality,implementations.reality-web.redesignStatus:applied` |
| Shipped but redesign not started | `implementations.ppt-web.buildStatus:shipped,implementations.ppt-web.redesignStatus:not-started` |

Filter syntax limit: comma-AND only (no `OR`). For more complex conditions, run multiple queries and intersect manually, or wait for Phase 3+ to add a richer query DSL.
