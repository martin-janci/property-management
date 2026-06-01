# Dispatcher self-review reports (TEMP_PHASE_8)

This directory collects post-mortem markdown reports written by **Phase 8 of
the dispatcher routine** — see `.research/dispatcher-prompt.md` section
"Phase 8 — Self-review (TEMPORARY, TEMP_PHASE_8)".

## What's here

One file per dispatcher run, named `<iso8601-utc>.md`. Each report is
≤ 60 lines and contains: run-shape counts, anomalies, trend vs. last 8
runs, improvement ideas, and a self-test snippet. The dispatcher itself
never reads these — they are write-once human-read.

## Status: TEMPORARY

This whole mechanism is instrumented temporarily for self-improvement
baseline-collection. **Target removal date: 2026-06-30.** To remove cleanly:

1. Grep the repo for `TEMP_PHASE_8` — every related artifact is tagged.
2. Delete the Phase 8 section in `.research/dispatcher-prompt.md`
   (between the `<!-- ... TEMP_PHASE_8 -->` markers) + the matching HARD
   RULE bullet.
3. Optionally delete this directory + README, or keep the historical
   reports as a record. They never affect the routine.
4. Land as a single small revert PR.

## Off-switch (per-run)

Set `DISPATCHER_SELF_REVIEW=0` in the routine's invocation environment
to skip Phase 8 without removing it. Useful for cost spikes or debugging
Phase 7 in isolation.

## Cost note

Phase 8 spawns an Opus subagent (~5-10k input tokens per run, ~$0.10-0.30
per Opus invocation at current Anthropic pricing). At ~12 dispatcher runs
per day this adds ~$2-4/day during the instrumentation window. Net cost of
the instrumentation phase is small compared to the savings from
PR #634 + #655 (~$85-115/month), but it adds up if left enabled
indefinitely — hence the deletion date.

## Retention

Reports older than 30 days are candidates for deletion. They aren't
auto-pruned (no cron). If you want, add to your cleanup routine:

```bash
find .research/self-improvement -maxdepth 1 -name '*.md' -mtime +30 -delete
```
