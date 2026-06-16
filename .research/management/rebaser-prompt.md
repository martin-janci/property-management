# Dispatcher Phase 5.6 — stale-approved-PR rebaser subagent prompt

> Extracted from `dispatcher-prompt.md` Phase 5.6 for token economy (PAP-164).
> The dispatcher spawns this subagent (cap 1 parallel) for an approved PR that
> is `CONFLICTING` and stale, and points it here; the subagent reads this file
> in its own fresh context.
>
> **Runtime inputs** (the dispatcher passes concrete values in its spawn
> message): `<n>` (PR number), `<head_ref>` / `<branch>` (head branch).
> Substitute wherever they appear below. NOTE: the subagent MUST NOT touch
> `rebase_attempts` — the dispatcher owns that counter (single-writer rule).

---

You are a PR rebaser for a stale approved PR. Inputs: `pr_number=<n>`,
`branch=<head_ref>`, `base=dev`. Do this exactly:

0. **Workspace isolation (MANDATORY — issue #7).** Run the standard
   worktree preamble injected into your brief (the "Subagent workspace
   isolation" preamble; canonical source: the "Subagent workspace
   isolation" section of `.research/dispatcher-prompt.md`) — export
   `TASK_ID=pr-<n>`, `BRANCH=<head_ref>`. All subsequent
   git operations run inside `/tmp/ppt-worktrees/pr-<n>/`. NEVER
   `gh pr checkout` in the dispatcher's working tree — it displaces
   `dev` and breaks Phase 6 of this run.
1. `gh pr checkout <n> --repo martin-janci/property-management`
2. `git fetch origin dev`
3. `git rebase origin/dev`
   - If conflicts ONLY in mechanical paths (sqlx, Cargo.lock, generated
     openapi/api-client, pnpm-lock.yaml, VERSION) → resolve per
     `ppt-pr-merge` Step 2 table, `git rebase --continue`.
   - Any other conflict → `git rebase --abort` and
     `gh pr comment <n> --body "Auto-rebase aborted: real code conflict in
     <paths>. Manual rebase required."`, return
     `rebased=false note=conflict-in:<paths>`.
4. `git push --force-with-lease origin <branch>`
5. Return EXACTLY: `rebased=<true|false> pr=<n> note=<short>`
