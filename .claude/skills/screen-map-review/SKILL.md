---
name: screen-map-review
description: Spawn the local Visual Review server (Hono + Preact SPA) on 127.0.0.1, open the browser, and walk every screen-map for a product with OK / Note checkboxes and a per-screen Save & Next button. Notes persist directly into Agent Log + Notes > Specific. Use when the user runs `/screens review` or asks to "review screens", "walk the screen-map", "do a redesign review".
---

# Screen-Map Review Skill

Drives the per-screen Visual Review UI. Saves user feedback directly into the markdown files (Agent Log + Notes), never mutates statuses.

## When to use

- User invokes `/screens review` (with optional `--product` / `--filter` / `--preview`).
- User asks to "walk through all screens", "review the new redesign", "go through screens one by one".

## Inputs

- `--product=ppt|reality` (optional).
- `--filter=<frontmatter-query>` (optional, e.g. `redesignStatus:in-progress`). Applied client-side after server fetches all matching screens.
- `--preview=local|staging|design` (optional, defaults to `local` if `pnpm dev` is running, else `staging`).
- `--root <path>` (optional).

## Implementation

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
pnpm -C "$REPO_ROOT/frontend" --filter @ppt/screen-map cli review \
  --root "$REPO_ROOT" \
  ${PRODUCT:+--product "$PRODUCT"} \
  ${PREVIEW:+--preview "$PREVIEW"}
```

The CLI runs in the foreground, opens the browser at `http://127.0.0.1:5179/?session=<token>`, and waits. The user closes the tab or the SPA hits "Save & Next" past the last screen → server `POST /api/session/finish` → process exits.

## Output handling

- On startup: print "Review server at http://127.0.0.1:<port>?session=<short-token-prefix>". Don't echo the full token (it's already in the browser URL).
- On graceful exit: print a tally — how many screens reviewed, how many had notes.
- On Ctrl-C: server shuts down; print "Review session interrupted".
