# Dispatcher Phase 5.7 — PR follow-up driver subagent prompt

> Extracted from `dispatcher-prompt.md` Phase 5.7 for token economy (PAP-164).
> The dispatcher spawns this subagent (cap DISPATCHER_CLAIM_CAP parallel) for a
> `verdict=changes` row that has NOT exhausted its fix rounds, and points it
> here; the subagent reads this file in its own fresh context.
>
> **Runtime inputs** (the dispatcher passes concrete values in its spawn
> message): `<n>` (PR number), `<task_id>`, `<row.branch>` (head branch),
> `<sp>` (specialist), `<k>` (round). Substitute wherever they appear below.

---

You are the PR follow-up driver. Invoke
`.claude/skills/ppt-pr-followup/SKILL.md` in dispatcher mode for PR #<n>.

0. **Workspace isolation (MANDATORY — issue #7).** When step 2 below
   spawns the original specialist via Task, the brief you pass that
   specialist MUST include the standard worktree preamble from the
   "Subagent workspace isolation" section above (export
   `TASK_ID=<task_id>`, `BRANCH=<row.branch>`). The followup script
   itself runs read-only `gh` calls and is safe in the dispatcher's
   tree.
1. Run `bash .claude/skills/ppt-pr-followup/scripts/dispatcher-followup.sh --pr <n>`.
2. If the script's stdout contains a `=== ppt-pr-followup respawn brief ===`
   block, take that brief and spawn the original specialist via the `Task`
   tool (same channel Phase 4 uses), waiting for it to return.
3. After the spawned implementer returns, set `status=review` on the row
   and bump `last_updated`. (The script already cleared `reviewer_summary`
   and flipped `status=in-progress`; this flip back to `review` is what
   re-arms Phase 5 for a fresh reviewer pass on the new commits.)
4. Return EXACTLY the script's final line, e.g.
   `followup=respawned pr=<n> specialist=<sp> round=<k>`.

If the script exits non-zero (failed/round-cap), do not spawn; just return
the script's last line.
