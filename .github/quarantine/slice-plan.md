# Quarantine burn-down slice plan — 201 → 0 by 2027-02-19

A week-by-week **quota schedule** and **wave plan** that turns the single shared
deadline (`2027-02-19`) into twelve dated checkpoints. Pairs with the operating
rhythm in [`cadence.md`](cadence.md) (the *how*) and the enforced contract in
[`docs/quarantine-policy.md`](../../docs/quarantine-policy.md) (the *rule*).

> Owner: **pm-scrum-master** (metric owned by pm-devops; per-wave delivery owned
> by pm-backend / pm-devops / pm-security). This file is the schedule; keep it
> consistent with `cadence.md`. Recompute from the live metric, not from this
> table, whenever a batch lands.

## The number

On **2026-11-21** 201 tests were grandfathered in under one shared 90-day
deadline (`2027-02-19`). As of **2026-11-28** the census is **201 rows** — 198
live `#[ignore]`s plus 3 stale registry rows already released in code. Two weeks
after rollout the burn-down was still **0 rows closed**: a one-shot deadline with
no interim target is a cliff, not a ramp.

| Input | Value |
|---|---|
| Census (registry rows) | **201** (198 live + 3 stale) |
| Deadline (`nearest_due`) | **2027-02-19** |
| As-of | 2026-11-28 |
| Weeks remaining | ~11.9 → **12 weekly checkpoints** |
| **Weekly target** | **~17 / week** (201 ÷ 12 = 16.75, rounded up to bank slack) |
| Target state | **0** — a quarantine is a holding pen, not a graveyard |

`17 × 11 + 14 = 201`. The quota rounds up (17, not 16.75) so the running total
stays **ahead** of a flat line and the final week (W12) carries a smaller tail
instead of a scramble.

## Weekly quota — cumulative must-clear line

Every checkpoint is a Friday; W12 lands on the deadline itself. The dispatcher
reads `check-quarantine-age.py --summary` weekly (see `cadence.md`): compare live
`total` against the **"remaining ≤"** column. Above the line = behind schedule =
escalate.

| Week | Ending | Close (target) | Cumulative closed | Remaining ≤ |
|---:|---|---:|---:|---:|
| W1  | 2026-12-04 | 17 |  17 | 184 |
| W2  | 2026-12-11 | 17 |  34 | 167 |
| W3  | 2026-12-18 | 17 |  51 | 150 |
| W4  | 2026-12-25 ⛄ | 17 |  68 | 133 |
| W5  | 2027-01-01 ⛄ | 17 |  85 | 116 |
| W6  | 2027-01-08 | 17 | 102 |  99 |
| W7  | 2027-01-15 | 17 | 119 |  82 |
| W8  | 2027-01-22 | 17 | 136 |  65 |
| W9  | 2027-01-29 | 17 | 153 |  48 |
| W10 | 2027-02-05 | 17 | 170 |  31 |
| W11 | 2027-02-12 | 17 | 187 |  14 |
| W12 | 2027-02-19 | 14 | **201** | **0** |

⛄ **Holiday weeks (W4–W5).** Treat Dec 25 and Jan 1 as *carry* weeks: assume
reduced throughput and **front-load W1–W3** so the cumulative line is banked
ahead before the holidays, not recovered after them. If W4/W5 slip, W6 absorbs
the carry — do **not** bump deadlines to paper over a holiday dip.

## Wave plan — the supply that feeds the quota

The quota (17/wk) is *demand*. These waves are *supply*: ordered batches, biggest
leverage first, sized so cumulative supply stays ahead of the quota line. A wave
is "fix-and-delete the registry rows", never "bump the `due`".

| Wave | Cluster | Rows | Owner | Target weeks | Why this slice |
|---|---|---:|---|---|---|
| 0 | **Stale registry rows** — delete | 3 | pm-devops | **W1** | Tests already released; deleting the rows is free burn-down and keeps the census honest. |
| 1 | **BIT-440 "workspace hostage"** | 42 | pm-backend | **W1–W3** | One root cause across 6 suites; a single BIT-440 fix releases all 42 (~2.5 weeks of quota in one PR). Highest leverage — do it first. |
| 2 | **schema/seed mismatch** (`*_not_seeded`) | 72 | pm-backend | **W3–W7** | Largest cluster; gated on the seed-recipe gap (see `ppt-db-migrations`). Batch by suite once the recipe lands. |
| 3 | **needs-postgres / RLS** (`--ignored` DB) | 44 | pm-backend | **W6–W10** | Candidates for a gated integration lane rather than per-test fixes — wire the lane, release the batch. |
| 4 | **misc self-contained** (AI-route 404/500, env-mutating, fixture-builder, BIT-588, #3104 portfolio, …) | 33 | pm-backend | **W4–W11** | Small independent fixes; use as **filler** to hold the quota through the holiday weeks and between the big waves. |
| 5 | **needs-redis** (2) + **needs-docker** (3) | 5 | pm-devops | **W10–W11** | CI already stands up Redis for other jobs; wire these two into that lane. Docker-daemon smoke tests need a runner with dockerd — heavier, hence later. |
| 6 | **pm-security TOCTOU** — implement | 1 | pm-security | **W11** | `admin_mfa_recovery_tests::concurrent_double_consume_skipped` — implement the concurrent test (owner already pm-security). |

**Supply total: 3 + 42 + 72 + 44 + 33 + 5 + 1 = 200.** The 201st row is the
at-risk row below.

### The one at-risk row (not on the quota)

- `backend/.../mfa_e2e_tests.rs::test_mfa_verify_rate_limited` — **blocked on
  issue #487** (rate-limit middleware on `/auth/mfa/verify`). It cannot be
  fix-and-deleted until #487 lands, so it is **excluded from the weekly quota**
  and tracked as a dependency risk. If #487 is not merged by **W10
  (2027-02-05)**, escalate: either expedite #487 or, as a last resort, re-triage
  this single row with a new deadline + a pointer to #487. A deadline bump here
  is a **documented escalation**, not a silent one — and it is the *only* row for
  which a bump is sanctioned.

## How to run the cadence against this plan (weekly)

1. `python3 .github/workflows/scripts/check-quarantine-age.py --summary` →
   capture the `QUARANTINE-BURNDOWN …` line.
2. Find this week's row in the quota table. If live `total` > the **"Remaining ≤"**
   cell, the burn-down is **behind** — escalate in the pm-devops rotation note.
3. Confirm the wave scheduled for this week is in flight and sized to its share.
4. Delete any newly-stale registry rows (the gate's `WARN: … stale registry row`
   lines) — free progress.
5. Paste the metric line into `.research/management/project-state.md` under the
   pm-devops one-liner (per `cadence.md`).

Recompute the quota from the live metric after every batch — `need_per_week` in
the `--summary` output does the division for the *remaining* rows, which is the
real forward target. This table is the opening plan, not a substitute for the
live number.

**Prefer fix-and-delete over bump-the-deadline, every week.** A bump moves the
cliff; it does not burn the backlog down.
