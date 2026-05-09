---
description: Gracefully close a worktree dev environment, freeing resources. Caches snapshot for TTL window so reopening within 2 days is fast.
---

# Close worktree

## Steps

1. Detect current worktree name from branch:
   ```bash
   BRANCH=$(git rev-parse --abbrev-ref HEAD)
   NAME=$(echo "$BRANCH" | tr '/_ A-Z' '----a-z')
   # or use the alias the user chose at open time
   ```

2. Call pmctl:
   ```bash
   pmctl close "$NAME" --json
   ```

3. Remove the override line from `frontend/.env.local`:
   ```bash
   # Strip VITE_API_DEFAULT, VITE_API_BASE, VITE_REALITY_API_BASE if they reference this worktree.
   ```

4. Report:
   ```
   Worktree `<name>` closed. Snapshot will be GC'd after TTL (default 2 days).
   ```
