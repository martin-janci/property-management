---
description: Spawn a worktree dev environment for the current branch (or specified branch) and configure the local frontend to talk to it.
---

# Open worktree

## Steps

1. Detect current branch (unless user specified one):
   ```bash
   BRANCH=$(git rev-parse --abbrev-ref HEAD)
   ```

2. Call pmctl:
   ```bash
   pmctl open "$BRANCH" --json
   ```
   Capture output JSON (contains `worktree.urls.ppt`, `worktree.urls.reality`, `worktree.name`).

3. Write `frontend/.env.local` (in the user's worktree, not server side):
   ```
   VITE_API_DEFAULT=worktree
   VITE_API_BASE=<urls.ppt>
   VITE_REALITY_API_BASE=<urls.reality>
   ```

4. Report to user:
   ```
   Worktree `<name>` ready:
   - ppt: <urls.ppt>
   - reality: <urls.reality>
   Backend: shared (default). Frontend mode set to `worktree` in .env.local.
   ```

## Notes

- If pmctl returns 409 conflict, the worktree already exists — call `pmctl status <name>` and report current URLs instead.
- For dedicated backend (Phase 3): pass `--backend=dedicated --as=<alias>`. Phase 1 returns 400 for this.
