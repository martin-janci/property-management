---
name: screen-map-init
description: Bootstrap or extend docs/screens/ for one product by scanning sitemap, use-cases, epics, an optional design ZIP, and any user-provided candidate names; then walks the user through interactive grouping in chat before bulk-writing the markdown files. Use when the user asks to "init screens", "bootstrap screen-map", "create screen maps for ppt|reality", or runs `/screens init`.
---

# Screen-Map Init Skill

Bulk-scans candidate screens for a product, presents a grouping plan in chat, and writes finalized screen-maps once the user approves.

## When to use

- User invokes `/screens init` (with `--product=ppt` or `--product=reality`).
- User asks to "init screens for <product>" or "bootstrap the screen-map".
- User provides a designs ZIP and wants those frames mapped to screens.

## Inputs (forwarded to the CLI)

- `--product=ppt|reality` (required).
- `--root <path>` (defaults to repo root).
- `--designs=<path-to-zip>` (optional).
- `--add="Name 1" --add="Name 2"` (optional user-added candidates).
- `--decisions=<jsonPath>` (optional; supplies grouping decisions non-interactively).
- `--force` (optional; overwrite existing files).

## Workflow

1. Resolve repo root via `git rev-parse --show-toplevel`.
2. Run an initial scan (no `--decisions`) to enumerate candidates from sitemap + use-cases + epics + designs + user-added, then **present** the candidate list to the user in chat as a markdown table:
   ```
   | Source | Id | Name | Sitemap | Frame | UC/Epic |
   |---|---|---|---|---|---|
   | sitemap | ppt/buildings-list | Buildings List | ppt-buildings-list | – | – |
   | design | ppt/redesign-foo | Redesign Foo | – | foo-v3 | – |
   | use-cases | ppt/uc-12 | UC-12 | – | – | UC-12 |
   ```
3. Propose groupings: identify candidates that look like duplicates ("ppt/buildings-list" + "ppt/uc-15" both relate to the same building screen) and suggest merges. Suggest skips for noise (one-off UC ids that don't match a real screen).
4. Wait for the user's reply. Accept free-form chat replies like:
   - "merge ppt/uc-12 and ppt/uc-15 into ppt/building-management"
   - "skip ppt/uc-99"
   - "rename ppt/foo to ppt/foo-detail"
   - "OK, go" (apply the proposed plan as-is)
5. Translate the user's reply into a `GroupingDecision[]` JSON, write to a temp file under `/tmp`.
6. Re-invoke the CLI with `--decisions=<tmpfile>` to apply the decisions and write the markdown files.
7. Run `/screens validate` to confirm the output is clean.

## Implementation

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"

# Step 1: initial scan (no decisions). Capture candidates as JSON for the chat presentation.
pnpm -C "$REPO_ROOT/frontend" --filter @ppt/screen-map cli init \
  --product "$PRODUCT" \
  --root "$REPO_ROOT" \
  ${DESIGNS:+--designs "$DESIGNS"} \
  ${ADD_FLAGS}
```

(The agent constructs `--add` flags for each user-supplied candidate and the optional `--designs` flag from the user's input.)

## Output handling

- All-clean → reply with the table and proposed grouping; ask user to confirm or correct.
- Validate after writing → if validation fails, surface errors; do NOT auto-revert (user can edit and re-run).
- File conflicts → if `bulkWriteScreenMaps` rejects with "already exists", ask the user whether to use `--force` or skip those screens.
