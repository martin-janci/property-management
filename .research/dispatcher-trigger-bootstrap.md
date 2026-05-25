# Dispatcher trigger bootstrap

This is the message body to paste into the live
`ppt-research-dispatcher` trigger (`trig_01RDNN7kYxzr4XULbi4xn5r2`) so
it reads its instructions from the repo instead of carrying an inlined
copy. Mirrors the pattern used by the `airbnb-invoices-sync` trigger.

Update the trigger via the Claude.ai routines UI or
`RemoteTrigger action=update` with the `job_config.ccr.events[0].data.message.content`
replaced with the block below — keep all other fields (`environment_id`,
`session_context`, `mcp_connections`, `cron_expression`) as-is.

---

```
Read .research/dispatcher-prompt.md from the repo root and execute it as your instructions for this run. Today's date is the run date. If $TRIGGER_TEXT is "deep" or "reset", surface that in the run summary; otherwise treat as a normal run.
```

That's the whole prompt. Future behavioural changes ship as normal PRs to
`dev` against `.research/dispatcher-prompt.md` — no further `RemoteTrigger
update` calls needed.
