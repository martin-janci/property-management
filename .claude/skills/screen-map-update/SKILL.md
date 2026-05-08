---
name: screen-map-update
description: Detect drift between code (sitemap, use-cases, epics) and screen-maps. Reports unmapped sitemap entries, unknown endpoints, missing components/use-cases/epics, and orphan screens. Interactive prompt to apply patches. Use when the user runs `/screens update` or asks to "check screen-map drift", "screens stale", "what's out of date".
---

# Screen-Map Update Skill

Detects drift between source-of-truth artefacts (`@ppt/sitemap`, `docs/use-cases.md`, `docs/epics/`) and the `docs/screens/` tree, then helps the user reconcile.

## When to use

- User invokes `/screens update` (with or without `--strict`).
- User asks "is the screen-map stale", "any drift in screens", "what changed since last screen-map sync".
- After a sprint that touched routes or use cases — to catch screens that need updates.

## Inputs (forwarded to the CLI)

- `--root <path>` (defaults to repo root).
- `--strict` (exit non-zero on any drift; CI-friendly).

## Workflow

1. Resolve repo root via `git rev-parse --show-toplevel`.
2. Run `pnpm --filter @ppt/screen-map cli update --root <repoRoot>`.
3. If output is `No drift detected.` → reply: "Screen-map is in sync."
4. Otherwise, parse the issue lines (`<kind> :: <screenId> :: <detail>`) and present them in chat as a grouped report:
   ```
   Drift detected:
   - Unmapped sitemap entries (3): ppt-buildings-list, ppt-faults-list, mobile-fault-detail-screen
   - Unknown endpoints in screens (1): ppt/foo references mystery_endpoint
   - Orphan screens (1): ppt/dead refers to deleted sitemap "ppt-dead-route"
   ```
5. Ask the user how to handle each group:
   - For unmapped sitemap: "create new screen-maps for these IDs?" → invoke `/screens init` with `--add` flags.
   - For unknown endpoints: "remove the references or fix the ID?" → edit screen-map markdown via `/screens edit`.
   - For orphans: "delete these screen-maps or rebind them?" → user decides.
6. After applying changes, run `/screens validate` to confirm tree is clean.

## Implementation

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
pnpm -C "$REPO_ROOT/frontend" --filter @ppt/screen-map cli update --root "$REPO_ROOT"
```

## Output handling

- All-clean → reply: "Screen-map is in sync."
- Drift detected → echo CLI output verbatim, then propose grouped resolution actions per drift kind.
- `--strict` exits non-zero — surface the count to the caller.
