---
name: ppt-research-trigger
description: Interact with the research routine itself — fire `deep` or `reset` runs from a Claude Code session without crafting curl by hand.
when_to_use: The user wants the daily routine to do a 30-day catch-up sweep (`text: "deep"`) or to wipe cursors and start fresh (`text: "reset"`), and is asking how to trigger it.
mode: both
capabilities: [C6]
tags: [workflow, infra]
---

# PPT Research Trigger

`routine-prompt.md` § *Special trigger payloads* documents three modes:

| `text` payload | Effect |
|---|---|
| `""` (empty) | Normal run — incremental since `last_pr_seen` / `last_commit_sha` / `last_issue_seen`. |
| `"deep"` | 30-day catch-up sweep. Cursors only advance after **all writes succeed** — opportunistic catch-up, not a cursor reset. |
| `"reset"` | Wipe `last_pr_seen`, `last_commit_sha`, `last_issue_seen`, `seen_signals`, and `hotspot_history`. Next run does an initial 14-day sweep. |

Today users open the README, find the routine's API trigger URL, and craft `curl` by hand. This skill makes the payloads discoverable and the curl template copy-pasteable.

## When to invoke

You (or the user) want to fire a non-default routine run. Examples:

- Routine missed a few days while you were on vacation → `text: "deep"`
- `state.json` looks corrupted or you want to re-sweep the last two weeks → `text: "reset"`
- Routine just shipped a hardening change and you want to confirm it converges on real recent activity → `text: "deep"` once, then watch the brief.

Skip this skill for a normal daily run — that's scheduled at 05:00 local in claude.ai/code/routines and needs no manual prod.

## What it gives you

- The exact URL + payload shapes (`{"text": "..."}` JSON body)
- The credentials each trigger needs and where to fetch them
- The expected response shape so you know "fired" vs "failed"
- Cross-reference to where the routine acts on each payload

## Inputs

- The routine's **API trigger URL** — copied from the routine's settings page at `https://claude.ai/code/routines` → `ppt-research` → *API trigger* (only visible if API triggers are enabled on this routine).
- An **API token** scoped to fire that routine. Fetched once from the same settings page; treat as a secret.

## Steps

1. **Fetch the trigger URL + token** (one-time per routine):
   - Open `https://claude.ai/code/routines`
   - Click `ppt-research` → scroll to *API trigger* section
   - Copy the URL (looks like `https://api.anthropic.com/v1/code/routines/<routine-id>/triggers`)
   - Generate a new token if one isn't already present; record it in your local secrets store (`pass`, `op`, whichever you use). **Don't commit it.**
2. **Pick the payload** based on what you want:
   - `{"text": ""}` → identical to the scheduled run
   - `{"text": "deep"}` → 30-day backlog catch-up
   - `{"text": "reset"}` → cursor wipe + next run is a 14-day initial sweep
3. **Fire the trigger** — single `curl`, plain JSON body:
   ```bash
   curl -fsS \
     -X POST "$ROUTINE_TRIGGER_URL" \
     -H "Authorization: Bearer $CLAUDE_ROUTINE_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"text": "deep"}'
   ```
   Response is a small JSON blob with the new run's id; the routine itself starts in the background. Watch `https://claude.ai/code/routines/ppt-research/runs` for progress.
4. **Verify the run committed** — after ~2–5 min, check the latest commit on `main`:
   ```bash
   gh api repos/martin-janci/property-management/commits/main --jq '.commit.message' | head -1
   # expected: "research: <YYYY-MM-DD> brief — …"
   ```
   For `text: "reset"`, today's brief includes the literal line "State reset; cursors cleared." For `text: "deep"`, the brief notes the 30-day window in *Since last run*.

## Deterministic verification

```bash
# 1. trigger URL is set in env (or pulled from a secrets store)
test -n "$ROUTINE_TRIGGER_URL" && echo OK
# expected: OK

# 2. token is set
test -n "$CLAUDE_ROUTINE_TOKEN" && echo OK
# expected: OK

# 3. payload is valid JSON (catches bad-quoting before firing)
echo '{"text": "deep"}' | jq -e '.text' >/dev/null && echo OK
# expected: OK
```

## Smoke check (single command)

```bash
test -f .research/routine-prompt.md && grep -q 'Special trigger payloads' .research/routine-prompt.md
```

## After-task verification

```bash
# After firing a deep / reset trigger, wait ~5 min, then:
gh api repos/martin-janci/property-management/commits/main --jq '.commit.message' | head -1
# For deep: expect "research: <today> brief — N merged PRs, M new vectors, P plans"
# For reset: expect the brief at .research/briefs/<today>.md to contain "State reset"
```

## Cross-references

- [`.research/routine-prompt.md`](../../../.research/routine-prompt.md) § *Special trigger payloads* — canonical payload semantics
- [`.research/README.md`](../../../.research/README.md) § *Activating the cloud routine* — where API triggers fit in the routine setup
- [`ppt-research-flow`](../ppt-research-flow/SKILL.md) — the implementer-side flow once plans are promoted
