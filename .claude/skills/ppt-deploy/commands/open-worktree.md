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

3. Write `.env.local` files for whichever app(s) the user is running.
   The variable names here MUST match what the apps actually read (otherwise
   the dev panel toggles to "worktree" but the API calls keep going local):
   - **ppt-web** (`frontend/apps/ppt-web/.env.local`):
     ```
     VITE_API_DEFAULT=worktree
     VITE_API_URL=<urls.ppt>
     VITE_API_BASE_URL=<urls.ppt>
     ```
     `VITE_API_URL` is read by the generated OpenAPI client; `VITE_API_BASE_URL`
     is read by the feature-specific fetch clients (auth, buildings, etc.).
     Setting both keeps everything pointed at the worktree backend.
   - **reality-web** (`frontend/apps/reality-web/.env.local`):
     ```
     NEXT_PUBLIC_API_DEFAULT=worktree
     NEXT_PUBLIC_API_URL=<urls.reality>
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
- For dedicated backend (Phase 3): pass `--backend=dedicated --alias=<alias>`. Phase 1 returns 400 for this.
