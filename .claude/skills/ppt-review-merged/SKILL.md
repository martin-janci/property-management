---
name: ppt-review-merged
description: Post-merge code review with issue creation. Reviews merged PRs in a time window, spots better approaches / missed edge cases / security holes / perf issues / test gaps, and opens GitHub issues with concrete improvement proposals.
when_to_use: You are asked to do a post-merge review pass — e.g. weekly retro, post-release sweep, or the dispatcher's daily Phase 2.5. You have a time window (default 14 days) and a repo. The skill spawns per-stack reviewer subagents, persists findings to `.research/management/post-merge-review.json`, and creates labeled follow-up issues.
mode: both
capabilities: [C6]
tags: [workflow, review, post-merge]
---

# PPT Review (Merged)

A second pair of eyes on already-shipped code. The implementer review (in
`ppt-implement` + dispatcher Phase 5) catches "should this merge?". This skill
catches "now that it's in dev, is it really the right shape?" — and turns
findings into trackable issues, not Slack noise.

## When to invoke

- Dispatcher routine Phase 2.5 (every ~24h via marker file).
- Manual: "review what merged this week", "do a post-merge sweep on epic-7a", etc.
- After a release: scoped to a release tag window.

## Inputs

| Input        | Default                        | Notes                                            |
| ---          | ---                            | ---                                              |
| `repo`       | `martin-janci/property-management` | Full GH slug                                 |
| `window`     | `14d`                          | "Nd" or RFC3339 date; passed as `merged:>=<d>` to gh |
| `base`       | `dev`                          | Only PRs merged into this base                   |
| `scope`      | (none)                         | Optional path glob to narrow, e.g. `backend/**`  |
| `max_prs`    | `15`                           | Hard cap — never spawn more reviewer subagents   |
| `label`      | `follow-up,from-merged-review` | Comma-separated GH labels for created issues     |

## Step 1 — Enumerate merged PRs

```bash
WINDOW_DATE=$(date -u -d "<window>" +%Y-%m-%d)   # for "14d" → 14 days ago
gh pr list \
  --repo "<repo>" --base "<base>" --state merged \
  --search "merged:>=$WINDOW_DATE" \
  --json number,title,mergedAt,mergedBy,headRefName,baseRefName,labels,additions,deletions,files \
  --limit "<max_prs>"
```

For each PR, also note:
- `body` (for original task context — usually pasted by `ppt-implement`)
- `commits[].messageHeadline` (to spot stacking)

Skip PRs whose label set already contains any of `{post-merge-reviewed, no-review-needed}` (set by past runs of this skill, or by humans).

## Step 2 — Pick a reviewer specialist per PR

Inspect each PR's `files[].path`. Score against the specialist matrix from
`ppt-implement/SKILL.md` (same rules). For PRs touching multiple stacks, pick
the dominant stack by total changed lines; record `secondary_specialists[]`
for the others (the reviewer will read those agents/*.md files too).

If no specialist scores ≥+3: use `generic`.

## Step 3 — Spawn reviewer subagents (PARALLEL, capped)

For each PR, spawn one subagent via the Task tool. **Concurrency cap: 5 in
parallel.** Subagent prompt (verbatim, substitute placeholders):

> You are a post-merge reviewer for PR #<n> on `<repo>` (already merged to
> `<base>`). Your job is NOT to block — the change is live. Your job is to
> spot whether the shape we shipped is the right shape, and to file a
> *constructive* follow-up issue if it isn't.
>
> Context loading (do this first, in order):
> 1. `gh pr view <n> --json title,body,labels,files,commits,mergedAt`
> 2. `gh pr diff <n>` — read the full diff (it's already merged; this works on the merge commit).
> 3. Read `.claude/skills/ppt-implement/agents/<primary-specialist>.md` and each `<secondary-specialist>.md` to refresh the conventions the implementer was expected to follow.
> 4. If the PR body references a task id (`gap-…`, `pm-…`), also read `.research/management/action-list.json` and `.research/management/assignments.json` to see what was originally asked.
>
> Review checklist (one pass per category; skip categories that obviously don't apply):
> - **Correctness vs the original action** — does the diff actually deliver what was asked, or did scope drift?
> - **Better approach available** — a simpler design, existing helper not used, reinvented wheel, missed framework feature.
> - **Security** — auth/authz holes (especially if owner_role=pm-security), input validation, secret handling, RLS for new tables, SQL injection, unsafe `.unwrap()` on user input.
> - **Performance** — N+1 queries, missing indexes, sync work in async handlers, full-table scans, hot-path allocations.
> - **Tests** — coverage of the change; was a failing-on-main test added (IG3)? Are there integration tests for cross-boundary changes?
> - **Regressions** — adjacent files that should have been updated (callers, types, screen-map frontmatter, generated clients).
> - **Conventions** — does the diff match the specialist's agents/*.md? (Naming, error types, RLS pattern, etc.)
> - **JSON-key-case sanity** — when the diff touches Rust integration tests, run:
>
>   ```bash
>   # Find DTOs in the diff that declare rename_all = camelCase:
>   rg -nP '#\[serde\(.*rename_all\s*=\s*"camelCase"' backend/ --type rust | head -20
>   # Find snake_case JSON accessors added to test code:
>   gh pr diff <n> | rg -nP '^\+.*json\[\s*"[a-z]+_[a-z_]+"\s*\]' | head -20
>   ```
>
>   If both lists overlap on the same DTO type → file a follow-up issue
>   under **category: correctness** (this is the bug class that bit
>   PR #473 on 2026-05-24 — `auth_tests.rs` read `json["access_token"]`
>   against a `#[serde(rename_all="camelCase")]` `LoginResponse` and every
>   test that called `create_authenticated_user` would have panicked).
>
> Findings: zero-or-many. Each finding has `category` (one of the above),
> `severity` ∈ {high, medium, low}, `description` (1-3 sentences), and a
> `proposal` (the concrete better approach, with file path + sketch).
>
> If you find ANYTHING actionable, draft a GitHub issue and create it:
>
> ```bash
> gh issue create --repo <repo> \
>   --title "Follow-up: <one-line summary> (PR #<n>)" \
>   --label "<label>" \
>   --body "$(cat <<'EOF'
> ## Context
> Spotted during post-merge review of PR #<n> (merged <mergedAt>).
> Original task: <task_id or 'unknown'>
>
> ## Findings
> ### <category> — severity: <high|medium|low>
> <description>
>
> **Proposed improvement:**
> <proposal>
> <... repeat for each finding ...>
>
> ## Suggested specialist
> <specialist>
>
> ---
> _Created by `ppt-review-merged` skill._
> EOF
> )"
> ```
>
> One issue per PR, grouping all findings — not one issue per finding.
>
> **ALWAYS label the PR after review, regardless of verdict** — this is what prevents the next run from re-reviewing and creating duplicate issues.
>
> ```bash
> # After issue creation OR if no findings:
> gh pr edit <n> --repo <repo> --add-label post-merge-reviewed
> ```
>
> If an issue was opened, also leave a one-line breadcrumb on the PR pointing at it (so the link is visible in the PR's conversation tab):
>
> ```bash
> gh pr comment <n> --repo <repo> --body "Post-merge review filed follow-up: #<issue-number>"
> ```
>
> Return EXACTLY one line:
> `pr=<n> verdict=<clean|issues> issue=<issue-number|none> findings=<count> note=<short>`

Capture each line.

## Step 4 — Persist findings

Append to `.research/management/post-merge-review.json` (create if missing):

```json
{
  "generated": "<iso>",
  "window": "<window>",
  "base": "<base>",
  "scope": "<scope or null>",
  "runs": [
    {
      "run_at": "<iso>",
      "prs_scanned": <int>,
      "clean": [<pr-numbers>],
      "with_issues": [{"pr": <int>, "issue": <int>, "findings": <int>, "specialist": "<name>", "note": "<short>"}]
    }
  ]
}
```

Merge logic: keep all prior `runs`; the latest run goes at the head of the
array. Cap to last 20 runs to keep the file small.

## Step 5 — Update marker + push

```bash
date -u +%Y-%m-%dT%H:%M:%SZ > .research/management/last-merged-review.txt
git add .research/management/post-merge-review.json .research/management/last-merged-review.txt
git commit -m "chore(research): post-merge review <date> — scanned N, opened M follow-ups"
git push origin <base>     # only these two files direct-to-base
```

## Step 6 — Return summary

```
Scanned: N PRs
Clean:   K PRs (labeled post-merge-reviewed)
Issues:  M follow-ups opened (links: <gh issue URLs>)
Skipped: J PRs already labeled
```

## Hard rules

- **Never edit code** in this skill. You spawn issues, not commits (except the marker + JSON in Step 5).
- **Never re-review** a PR labeled `post-merge-reviewed` or `no-review-needed`.
- **Cap** reviewer subagents at 5 parallel; if `max_prs` exceeds the cap, queue serially.
- **One issue per PR**, not per finding.
- **Be constructive** — every issue must include a `Proposed improvement` block, not just complaints.

## Install (local user)

```bash
bash .claude/skills/ppt-review-merged/install.sh
```

The cloud routine reads this skill directly from the repo checkout.
