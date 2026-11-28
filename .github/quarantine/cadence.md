# Quarantine burn-down cadence

A weekly operating rhythm for driving the quarantined-test backlog to **zero**
before the shared deadline, so the [`quarantine-age-gate`](../workflows/quarantine-age-gate.yml)
never hits the fail-cliff.

- **Policy (the contract):** [`docs/quarantine-policy.md`](../../docs/quarantine-policy.md)
- **Registry (the backlog):** [`registry.toml`](registry.toml)
- **Metric (this cadence's number):** `check-quarantine-age.py --summary`
- **Slice plan (the schedule):** [`slice-plan.md`](slice-plan.md) — the 12-week
  ~17/week quota + wave plan this cadence drives to zero by `2027-02-19`.

> Owner: **pm-devops** (with pm-tech-lead). This file is the operating cadence;
> the policy file is the enforced contract. Keep them consistent.

## Why a cadence (not just a gate)

The age-gate makes a rotting quarantine *impossible to ignore* — but only at the
deadline. On **2026-11-21** 201 tests were grandfathered in with one shared
90-day deadline (`2027-02-19`). Two weeks later the burn-down was **0 rows
closed**. At zero rows/day the gate does not warn gradually; it flips the whole
backlog red in one day when the deadline passes. A gate without a cadence just
schedules a cliff. This cadence turns the one-shot deadline into a weekly quota
with an owner on the hook each week.

## The weekly dispatcher-visible metric

`check-quarantine-age.py --summary` prints a grep-friendly token line plus a
markdown table. It never fails (it is a report, not a gate), and it counts the
**live `#[ignore]` tests in the tree** — the true, enforceable backlog — not the
registry row count (which can drift ahead of the code; see "Registry hygiene").

```bash
python3 .github/workflows/scripts/check-quarantine-age.py --summary
# QUARANTINE-BURNDOWN total=198 pm-backend=190 pm-devops=6 pm-security=2 \
#   nearest_due=2027-02-19 days_left=87 weeks_left=12.4 need_per_week=15.9 target=0
```

**Where it surfaces (dispatcher-visible):**

1. **CI step summary — automatic, weekly.** The `quarantine-age-gate` workflow
   runs `--summary` on its daily schedule and appends the table to
   `$GITHUB_STEP_SUMMARY`. The dispatcher reads workflow runs over MCP
   (`mcp__github__actions_*` / `get_job_logs`), so the number is visible without
   anyone editing a doc.
2. **pm-devops rotation — weekly.** When the pm-devops role comes up in the
   project-management rotation, lift the `QUARANTINE-BURNDOWN …` token line
   verbatim into `.research/management/project-state.md` under the pm-devops
   one-liner, replacing the previous week's. One line, diff-stable (owners are
   emitted sorted), so week-over-week movement is a clean diff.

**Reading the metric:**

| Field | Meaning |
|---|---|
| `total` | live quarantined tests remaining (the burn-down number) |
| `pm-*=<n>` | remaining rows per accountable owner |
| `nearest_due` | soonest deadline across all rows (the cliff date) |
| `days_left` / `weeks_left` | time to that deadline |
| `need_per_week` | rows/week the whole backlog must close to reach `target` by `nearest_due` |
| `target` | always `0` — a quarantine is a holding pen, not a graveyard |

If `need_per_week` rises week over week, the backlog is losing ground: either
rows aren't closing or deadlines are being bumped instead of fixed.

## Per-owner weekly quota

Spread each owner's share evenly across the weeks remaining to the deadline.
At rollout (87 days / ~12.4 weeks to `2027-02-19`):

| Owner | Open rows | Rows/week to clear | First-wave target |
|---|---:|---:|---|
| pm-backend | 190 | ~16 | BIT-440 cluster (42 rows, one root cause) |
| pm-devops | 6 | ~1 | the 2 Redis-dependency tests |
| pm-security | 2 | <1 | the TOCTOU test (the other is blocked on #487) |
| **total** | **198** | **~16** | |

The global `need_per_week` (~16) is almost entirely pm-backend's — the other two
owners are a rounding error on the quota but still need an owner on the hook so
they don't become the forgotten tail that trips the gate on the last day.

Recompute the quota whenever a batch lands (the metric does the division for
you): `need_per_week` is the live rows/week for the *remaining* rows.

## First-wave batch per owner

The first wave is the highest-leverage, lowest-friction batch each owner can
claim now. Land these first; they move the metric the most per unit of work.

### pm-backend — the BIT-440 "workspace hostage" cluster (42 rows)

42 of the 190 rows share one root cause: BIT-440 ("fails on dev — workspace
hostage under the parallel runner"). They span `disputes_happy_path`,
`documents_forms_happy_path`, `faults_workflow_happy_path`,
`messaging_attachments_authz`, `package_visitor_happy_path`, and
`rentals_core_backfill_batch2`. One fix to BIT-440 releases the whole cluster —
~2.5 weeks of quota in a single PR. This is wave 1; after it, the next waves are
the reason-clustered groups:

- **schema/seed mismatch (~73):** `*_not_seeded` suites — needs the seed-recipe
  gap closed (see `ppt-db-migrations`); batch by suite.
- **needs-postgres (~49):** `--ignored` DB tests — candidates for a gated
  integration lane rather than per-test fixes.
- **#3104 portfolio (3)** and **RLS (3)** — small, self-contained.

### pm-devops — the 2 Redis-dependency tests (2 of 6 rows)

- `notification_preference_realtime_sync_tests::preference_update_publishes_realtime_event`
- `ws_integration_tests::ws_pubsub_fanout_delivers_to_correct_subscriber`

Both are quarantined only because they need a live Redis (`REDIS_URL` +
`--include-ignored`). CI already stands up Redis for other jobs; wiring these
two into that lane releases both without touching product code. The remaining 4
pm-devops rows are the `deploy-server` smoke/docker tests, which need a docker
daemon in the runner — heavier, deferred to wave 2.

### pm-security — the TOCTOU recovery-code test (1 of 2 rows)

- `admin_mfa_recovery_tests::concurrent_double_consume_skipped` — "concurrent
  TOCTOU test not yet implemented"; implement it (owner already pm-security).
- `mfa_e2e_tests::test_mfa_verify_rate_limited` is **blocked** on issue **#487**
  (rate-limit middleware on `/auth/mfa/verify`). Leave quarantined with a
  pointer to #487; do not bump its deadline silently — escalate if #487 slips
  past the shared deadline.

## Registry hygiene (free burn-down)

The metric counts live `#[ignore]`s (198), but the registry has **201** rows —
**3 stale rows** whose tests have already been released (the gate reports these
as `WARN: … stale registry row`). Deleting them is free backlog reduction and
keeps the registry an honest census:

- `backend/crates/db/tests/suites/rag_similarity_search_tests.rs::rag_retrieval_correctness`
- `backend/crates/db/tests/suites/rag_similarity_search_tests.rs::search_documents_pgvector_model_filter_provenance`
- `backend/servers/api-server/tests/suites/integrations_marketplace_reviews_tests.rs::get_feature_stats_returns_200`

Regenerate the registry skeleton after any batch lands:

```bash
python3 .github/workflows/scripts/check-quarantine-age.py --census
```

## The burn-down series (machine-readable history)

The `--summary` metric above is a *snapshot* — it keeps no history, so
week-over-week movement is only ever two step-summaries eyeballed side by side.
The **burn-down series** turns that into an append-only time series:

- **Series (the history):** [`burndown-series.jsonl`](burndown-series.jsonl) —
  one JSON object per **workstream** (accountable `pm-*` owner) per run, with the
  four ways that workstream's backlog moved since the previous run:

  ```jsonl
  {"date": "2026-11-28","workstream": "pm-backend","rows_total": 190,"rows_deleted": 0,"rows_converted_to_gated_lane": 0,"due_bumps": 0}
  ```

  | Field | Meaning |
  |---|---|
  | `date` | run date (YYYY-MM-DD) |
  | `workstream` | accountable `pm-*` owner (mirrors the `--summary` owner breakdown) |
  | `rows_total` | OPEN registry rows for that owner (gated-lane rows excluded) |
  | `rows_deleted` | rows open last run, gone from the registry now (fix-and-delete) |
  | `rows_converted_to_gated_lane` | rows open last run, now moved to a gated integration lane |
  | `due_bumps` | rows open in both runs whose deadline moved *later* (a cadence failure) |

- **Snapshot (for the diff):** `burndown-state.json` — the previous run's
  per-test `{owner, due, gated}`, rewritten each run. This is what the deltas are
  computed against; do not hand-edit it.
- **Generator:** `check-quarantine-age.py`'s sibling
  [`burndown-series.py`](../workflows/scripts/burndown-series.py). Run it to
  append today's rows (idempotent per date):

  ```bash
  python3 .github/workflows/scripts/burndown-series.py            # append today's rows
  python3 .github/workflows/scripts/burndown-series.py --dry-run  # preview, write nothing
  python3 .github/workflows/scripts/burndown-series.py --self-test
  ```

A row is counted **gated** (out of the open `rows_total`, counted once as a
conversion) when its registry row has `lane = "gated…"`, a truthy `gated` field,
or a reason mentioning `gated lane` — the convention the pm-devops
"needs-postgres → gated lane" wave adopts. No rows are gated today.

> **Cadence wiring (TODO).** The daily `quarantine-age-gate` schedule surfaces
> today's series rows to the step summary, but does **not** yet commit the append
> — `dev` is protected and rejects CI pushes (GH006). Persisting the daily row
> needs the `version-bump.yml` pattern (a PAT that pushes a branch, opens a PR and
> arms squash auto-merge); see the TODO in `quarantine-age-gate.yml`. Until then,
> run the generator and commit the series file in a normal PR as part of this
> weekly cadence.

## Cadence checklist (weekly)

1. Run `--summary`; capture the `QUARANTINE-BURNDOWN …` line.
2. Compare `total` and `need_per_week` to last week. Flat/rising = escalate.
3. Confirm each owner has a wave in flight sized to their quota.
4. Clean up any new stale registry rows (WARN lines).
5. Append today's row to the series and paste the metric line into the pm-devops
   rotation note: `python3 .github/workflows/scripts/burndown-series.py`.

Prefer **fix-and-delete** over **bump-the-deadline** every time. A deadline bump
is a cadence failure, not a cadence tool — it only moves the cliff.
