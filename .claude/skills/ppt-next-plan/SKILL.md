---
name: ppt-next-plan
description: Pick the next plan to ship from .research/backlog.json — highest-score ready row, deterministic tiebreak.
when_to_use: The user is about to start an implementer session and asks "what's next?" or otherwise wants a one-shot lookup of the top ready plan.
mode: both
capabilities: [C6]
tags: [workflow]
---

# PPT Next Plan

Today the README documents the manual flow ("look at `backlog.md`, find a row with `status: ready`, load that plan"). This skill turns that into a one-liner: parse `backlog.json` for `status: "ready"` items, sort by `score` desc + `updated_at` desc, print the top one's `plan` path.

## When to invoke

You're starting an implementer session and want the next thing to ship. The routine has already promoted ≥1 plan today (or in recent runs), and you don't want to read the whole table.

Skip this skill when:
- You already have a specific plan in mind (just open it).
- `backlog.json` is empty or has no `status: "ready"` rows — there's nothing for the implementer agent to do; either wait for the next routine run or fire `text: "deep"` (see `ppt-research-trigger`).

## What it gives you

- A `jq` one-liner that returns the top-ranked ready plan's path
- The deterministic tiebreak (score desc, then `updated_at` desc) so two runs in a row pick the same plan
- The follow-on `cat .research/plans/<slug>.md` that primes you for the implementer flow

## Inputs

- `.research/backlog.json` — must parse as valid JSON (the routine's Quality Gate 3 guarantees this on `main`)

## Steps

1. **Find the top ready row.** Single jq pipeline (sort by score desc, then `updated_at` desc; pick `.plan`):
   ```bash
   jq -r '
     [.items[] | select(.status == "ready" and .plan != null)]
     | sort_by([.score // 0, .updated_at])
     | reverse
     | .[0].plan // "NONE"
   ' .research/backlog.json
   ```
   - Output is `plans/<slug>.md` (relative to `.research/`), or the literal `NONE` if nothing's ready.
   - `sort_by([.score, .updated_at])` sorts ASC by both keys; the trailing `reverse` flips the array so both keys end up DESC — the same order `backlog.md` renders. A simpler `sort_by(.score)` won't tiebreak deterministically.

2. **Open the plan**:
   ```bash
   PLAN=$(jq -r '
     [.items[] | select(.status == "ready" and .plan != null)]
     | sort_by([.score // 0, .updated_at]) | reverse | .[0].plan // "NONE"
   ' .research/backlog.json)
   [ "$PLAN" = "NONE" ] && { echo "no ready plans"; exit 1; }
   cat ".research/$PLAN"
   ```

3. **Hand off to the implementer agent.** From here, follow `ppt-research-flow` — `SLUG=$(basename "$PLAN" .md)`, `git switch -c "impl/$SLUG" main`, etc.

### awk alternative (no jq)

If `jq` isn't available, the same picker in awk against `backlog.md`:

```bash
awk -F'|' '/^\|[[:space:]]*[0-9]/ && $7 ~ /ready/ { gsub(/^ +| +$/, "", $2); print $2 }' \
  .research/backlog.md | head -1
```

`backlog.md` is sorted by score desc already (per the routine), so `head -1` is the top row. Less robust than jq because it depends on the rendered column layout — prefer jq when available.

## Deterministic verification

```bash
# 1. backlog.json parses
jq '.items | length' .research/backlog.json >/dev/null && echo OK
# expected: OK

# 2. the picker either returns a real plan path or NONE — never errors mid-pipeline
jq -re '
  [.items[] | select(.status == "ready" and .plan != null)]
  | sort_by([.score // 0, .updated_at]) | reverse | .[0].plan // "NONE"
' .research/backlog.json >/dev/null && echo OK
# expected: OK

# 3. if a path is returned, it exists on disk
PLAN=$(jq -r '
  [.items[] | select(.status == "ready" and .plan != null)]
  | sort_by([.score // 0, .updated_at]) | reverse | .[0].plan // "NONE"
' .research/backlog.json)
if [ "$PLAN" != "NONE" ]; then
  test -f ".research/$PLAN" && echo "OK: $PLAN"
fi
# expected: OK: plans/<slug>.md  OR  (silent — nothing ready)
```

## Smoke check (single command)

```bash
test -f .research/backlog.json && jq -e '.items' .research/backlog.json >/dev/null
```

## After-task verification

```bash
# After opening the picked plan, confirm the implementer hand-off worked:
[ -n "$PLAN" ] && [ -f ".research/$PLAN" ] && echo "ready to: SLUG=$(basename "$PLAN" .md)"
```

## Cross-references

- [`.research/README.md`](../../../.research/README.md) § *Picking a plan to ship — two execution modes* — manual flow this skill automates
- [`.research/implementer-prompt.md`](../../../.research/implementer-prompt.md) § *Pre-flight* — what the implementer does once given a plan path
- [`ppt-research-flow`](../ppt-research-flow/SKILL.md) — end-to-end choreography from plan → PR → archive
- [`ppt-research-trigger`](../ppt-research-trigger/SKILL.md) — what to do when nothing's ready
