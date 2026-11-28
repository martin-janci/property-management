#!/usr/bin/env python3
"""Hardening burn-down series — one machine-readable row per workstream per run.

Why this exists
---------------
``check-quarantine-age.py --summary`` prints a *point-in-time* snapshot of the
quarantine backlog (total, per-owner, nearest deadline, required burn rate). It
answers "where are we today" but keeps no history, so week-over-week movement is
only ever a diff of two step-summaries someone has to eyeball. The cadence
(``.github/quarantine/cadence.md``) is explicitly a *burn-down* — a quota that
must trend to zero before the shared deadline — and a burn-down needs a series,
not just a latest number.

This script generates that series. On each run it reads the quarantine registry
(``.github/quarantine/registry.toml``), diffs it against the last committed
snapshot, and appends one JSONL row **per workstream** to a committed series
file. Each row carries the four ways a workstream's backlog moved since the
previous run — the exact three levers cadence.md names (fix-and-delete,
convert-to-a-gated-lane, bump-the-deadline) plus the running total:

    date, workstream, rows_total, rows_deleted,
    rows_converted_to_gated_lane, due_bumps

* **workstream** — the accountable ``pm-*`` owner (mirrors the ``by_owner``
  breakdown in ``check-quarantine-age.py --summary``). One row per owner present.
* **rows_total** — OPEN registry rows for that owner (the burn-down-to-zero
  number). Rows moved to a gated integration lane (see below) are *not* open and
  do not count here.
* **rows_deleted** — rows that were open last run and are gone from the registry
  entirely now (fix-and-delete — the preferred lever).
* **rows_converted_to_gated_lane** — rows that were open last run and now carry a
  gated-lane marker (moved into a gated integration lane rather than fixed
  per-test; cadence.md's "needs-postgres -> gated lane" wave). A row is "gated"
  when it has ``lane = "gated"`` (any value containing ``gated``) or its reason
  contains ``gated lane`` / ``[gated-lane]``. Still owner+due'd so the age-gate
  stays green; just no longer part of the open burn-down.
* **due_bumps** — rows open in both runs whose ``due`` date moved *later*
  (deadline bumped — a cadence failure, tracked so it is visible, not silent).

The first run has no prior snapshot, so all deltas are 0 and ``rows_total`` is
the current open count (a clean baseline). The run is idempotent per date: a
second run on the same date is a no-op unless ``--force`` is given, so the daily
schedule firing more than once (schedule + push) never double-appends.

Generated from the registry, as the task specifies. The registry can drift
ahead of the live ``#[ignore]`` set (stale rows); ``check-quarantine-age.py``
remains the source of truth for the *live* count and FAILs on drift. This series
tracks the registry's own burn-down and is deliberately DB-free and fast.

Modes
-----
  (default)       append today's rows to the series file; rewrite the snapshot
  --dry-run       compute + print the rows as JSONL to stdout; write nothing
  --force         append even if the series already has rows for today's date
  --self-test     run the built-in fixture suite over the pure compute functions
  --today DATE    override "today" (YYYY-MM-DD)
  --registry P    registry.toml path (default: .github/quarantine/registry.toml)
  --series P      series JSONL path (default: .github/quarantine/burndown-series.jsonl)
  --state P       snapshot JSON path (default: .github/quarantine/burndown-state.json)
"""
from __future__ import annotations

import datetime as _dt
import json
import sys
from pathlib import Path

try:  # Python 3.11+ stdlib; GitHub ubuntu runners have it.
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - very old runners
    tomllib = None  # type: ignore[assignment]

REPO_ROOT = Path(__file__).resolve().parents[3]
REGISTRY_PATH = REPO_ROOT / ".github" / "quarantine" / "registry.toml"
SERIES_PATH = REPO_ROOT / ".github" / "quarantine" / "burndown-series.jsonl"
STATE_PATH = REPO_ROOT / ".github" / "quarantine" / "burndown-state.json"

UNASSIGNED = "(unassigned)"
# Ordered field list — the series row schema. Kept explicit so every writer uses
# the same key order (diff-stable JSONL) and readers can rely on it.
FIELDS = (
    "date",
    "workstream",
    "rows_total",
    "rows_deleted",
    "rows_converted_to_gated_lane",
    "due_bumps",
)


def parse_date(s: str | None) -> _dt.date | None:
    if not s:
        return None
    try:
        return _dt.date.fromisoformat(str(s))
    except ValueError:
        return None


def is_gated(row: dict) -> bool:
    """True when a registry row has been moved to a gated integration lane.

    Detected via an explicit ``lane`` field whose value mentions ``gated`` (e.g.
    ``lane = "gated"`` / ``lane = "gated-postgres"``), a truthy ``gated`` field,
    or a reason containing ``gated lane`` / ``[gated-lane]``. No such rows exist
    today; the generator understands the convention so pm-devops can adopt it for
    the "needs-postgres -> gated lane" wave without new tooling.
    """
    lane = row.get("lane")
    if isinstance(lane, str) and "gated" in lane.lower():
        return True
    if row.get("gated") in (True, "true", "True"):
        return True
    reason = row.get("reason")
    if isinstance(reason, str):
        low = reason.lower()
        if "gated lane" in low or "[gated-lane]" in low:
            return True
    return False


def parse_registry(data: dict) -> dict[str, dict]:
    """Map ``test`` key -> {owner, due, gated} for every [[quarantine]] row.

    Takes already-parsed TOML (a dict) so the compute path is pure and testable
    without touching the filesystem. Rows without a ``test`` key are skipped
    (they cannot be keyed); a later duplicate ``test`` overwrites an earlier one.
    """
    out: dict[str, dict] = {}
    for row in data.get("quarantine", []):
        test = row.get("test")
        if not test:
            continue
        out[test] = {
            "owner": row.get("owner") or UNASSIGNED,
            "due": row.get("due"),
            "gated": is_gated(row),
        }
    return out


def compute_rows(
    current: dict[str, dict], prior: dict[str, dict], today: _dt.date
) -> list[dict]:
    """Pure core: build the per-workstream series rows for ``today``.

    ``current`` / ``prior`` are ``{test: {owner, due, gated}}`` maps (prior is the
    last run's snapshot; ``{}`` on the first run). One row per workstream that has
    an OPEN (non-gated) row now, or that lost/converted/bumped a row since the
    prior run — so a workstream that went to zero this run still emits a final
    ``rows_total: 0`` row. Rows are returned sorted by workstream for stable diffs.
    """
    open_now = {k: v for k, v in current.items() if not v["gated"]}
    open_prior = {k: v for k, v in prior.items() if not v["gated"]}

    # Every workstream we must emit: has open rows now, or had movement.
    workstreams: set[str] = set()
    for v in open_now.values():
        workstreams.add(v["owner"])
    for k, v in open_prior.items():
        # A prior-open row that left or was gated is movement for its owner.
        if k not in open_now:
            workstreams.add(v["owner"])

    rows: list[dict] = []
    for ws in sorted(workstreams):
        rows_total = sum(1 for v in open_now.values() if v["owner"] == ws)

        deleted = 0
        converted = 0
        bumps = 0
        for key, pv in open_prior.items():
            if pv["owner"] != ws:
                continue
            cur = current.get(key)
            if cur is None:
                deleted += 1
            elif cur["gated"]:
                converted += 1
            else:
                # Still open in both — did its deadline move later?
                pd = parse_date(pv["due"])
                cd = parse_date(cur["due"])
                if pd is not None and cd is not None and cd > pd:
                    bumps += 1

        rows.append({
            "date": today.isoformat(),
            "workstream": ws,
            "rows_total": rows_total,
            "rows_deleted": deleted,
            "rows_converted_to_gated_lane": converted,
            "due_bumps": bumps,
        })
    return rows


def load_registry(path: Path) -> dict[str, dict]:
    if tomllib is None:  # pragma: no cover
        raise SystemExit("ERROR: tomllib unavailable; need Python 3.11+ to read the registry")
    if not path.exists():
        return {}
    with path.open("rb") as fh:
        return parse_registry(tomllib.load(fh))


def load_state(path: Path) -> dict[str, dict]:
    if not path.exists():
        return {}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}
    # Snapshot is {test: {owner, due, gated}}.
    return data if isinstance(data, dict) else {}


def write_state(path: Path, current: dict[str, dict]) -> None:
    # Sorted keys -> diff-stable snapshot.
    path.write_text(
        json.dumps(current, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


def series_has_date(path: Path, date_iso: str) -> bool:
    if not path.exists():
        return False
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            if json.loads(line).get("date") == date_iso:
                return True
        except ValueError:
            continue
    return False


def render_jsonl(rows: list[dict]) -> str:
    # Explicit key order via FIELDS -> every row serialises identically.
    return "".join(
        json.dumps({k: row[k] for k in FIELDS}, separators=(",", ": ")) + "\n"
        for row in rows
    )


def append_series(path: Path, rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as fh:
        fh.write(render_jsonl(rows))


def run(
    today: _dt.date,
    registry_path: Path,
    series_path: Path,
    state_path: Path,
    *,
    dry_run: bool,
    force: bool,
) -> int:
    current = load_registry(registry_path)
    prior = load_state(state_path)
    rows = compute_rows(current, prior, today)

    if dry_run:
        sys.stdout.write(render_jsonl(rows))
        return 0

    if series_has_date(series_path, today.isoformat()) and not force:
        print(
            f"burndown-series: {series_path.name} already has rows for "
            f"{today.isoformat()}; skipping (use --force to re-append).",
            file=sys.stderr,
        )
        # Keep the snapshot fresh so the next distinct-date run diffs correctly.
        write_state(state_path, current)
        return 0

    append_series(series_path, rows)
    write_state(state_path, current)
    total = sum(r["rows_total"] for r in rows)
    print(
        f"burndown-series: appended {len(rows)} workstream row(s) for "
        f"{today.isoformat()} (open rows_total={total}) to {series_path.name}."
    )
    return 0


# --------------------------------------------------------------------------
# Self-test — pure-function fixtures, DB-free, no filesystem.
# --------------------------------------------------------------------------
def self_test() -> int:
    failures = 0

    def check(label: str, cond: bool) -> None:
        nonlocal failures
        if cond:
            print(f"ok:   {label}")
        else:
            print(f"FAIL: {label}", file=sys.stderr)
            failures += 1

    today = _dt.date(2026, 11, 28)

    # ---- parse_registry + is_gated ----
    data = {"quarantine": [
        {"test": "a::t1", "owner": "pm-backend", "due": "2027-02-19"},
        {"test": "a::t2", "owner": "pm-devops", "due": "2027-02-19",
         "lane": "gated-postgres"},
        {"test": "a::t3", "owner": "pm-security", "due": "2027-02-19",
         "reason": "needs live Redis; moved to gated lane"},
        {"owner": "pm-backend"},  # no test key -> skipped
    ]}
    parsed = parse_registry(data)
    check("parse skips row with no test key", len(parsed) == 3)
    check("is_gated via lane field", parsed["a::t2"]["gated"] is True)
    check("is_gated via reason substring", parsed["a::t3"]["gated"] is True)
    check("non-gated default", parsed["a::t1"]["gated"] is False)

    # ---- first run: no prior -> baseline, all deltas 0 ----
    current = {
        "a::t1": {"owner": "pm-backend", "due": "2027-02-19", "gated": False},
        "a::t2": {"owner": "pm-backend", "due": "2027-02-19", "gated": False},
        "b::t3": {"owner": "pm-devops", "due": "2027-02-19", "gated": False},
    }
    rows = compute_rows(current, {}, today)
    by = {r["workstream"]: r for r in rows}
    check("first run emits one row per owner", len(rows) == 2)
    check("first run rows sorted by workstream", [r["workstream"] for r in rows] == ["pm-backend", "pm-devops"])
    check("first run pm-backend rows_total=2", by["pm-backend"]["rows_total"] == 2)
    check("first run pm-devops rows_total=1", by["pm-devops"]["rows_total"] == 1)
    check("first run all deltas zero", all(
        r["rows_deleted"] == 0 and r["rows_converted_to_gated_lane"] == 0 and r["due_bumps"] == 0
        for r in rows))
    check("row carries today's date", by["pm-backend"]["date"] == "2026-11-28")

    # ---- second run: one deleted, one converted, one bumped ----
    prior = dict(current)
    nxt = {
        # t1 deleted (fix-and-delete)
        "a::t2": {"owner": "pm-backend", "due": "2027-02-19", "gated": False},  # unchanged
        "b::t3": {"owner": "pm-devops", "due": "2027-02-19", "gated": True},    # converted to lane
    }
    rows2 = compute_rows(nxt, prior, today)
    by2 = {r["workstream"]: r for r in rows2}
    check("delete counted for pm-backend", by2["pm-backend"]["rows_deleted"] == 1)
    check("pm-backend open total drops to 1", by2["pm-backend"]["rows_total"] == 1)
    check("conversion counted for pm-devops", by2["pm-devops"]["rows_converted_to_gated_lane"] == 1)
    check("pm-devops open total drops to 0", by2["pm-devops"]["rows_total"] == 0)
    check("zeroed workstream still emits a final row", "pm-devops" in by2)

    # ---- due bump: deadline moved later = 1 bump; earlier/equal = 0 ----
    prior_b = {"x::a": {"owner": "pm-backend", "due": "2027-02-19", "gated": False}}
    bumped = {"x::a": {"owner": "pm-backend", "due": "2027-05-01", "gated": False}}
    check("later due counts as a bump", compute_rows(bumped, prior_b, today)[0]["due_bumps"] == 1)
    earlier = {"x::a": {"owner": "pm-backend", "due": "2027-01-01", "gated": False}}
    check("earlier due is not a bump", compute_rows(earlier, prior_b, today)[0]["due_bumps"] == 0)
    same = {"x::a": {"owner": "pm-backend", "due": "2027-02-19", "gated": False}}
    check("unchanged due is not a bump", compute_rows(same, prior_b, today)[0]["due_bumps"] == 0)

    # ---- empty registry -> no rows, no crash ----
    check("empty registry yields no rows", compute_rows({}, {}, today) == [])

    # ---- a gated row never counts toward rows_total ----
    gated_only = {"g::a": {"owner": "pm-devops", "due": "2027-02-19", "gated": True}}
    check("gated-only registry yields no open rows", compute_rows(gated_only, {}, today) == [])

    # ---- render_jsonl: stable key order, valid JSON, one line per row ----
    out = render_jsonl(compute_rows(current, {}, today))
    lines = out.strip().splitlines()
    check("render emits one line per row", len(lines) == 2)
    first = json.loads(lines[0])
    check("rendered row has exactly the schema fields", list(first.keys()) == list(FIELDS))

    if failures:
        print(f"\n{failures} self-test assertion(s) failed", file=sys.stderr)
        return 1
    print("\nAll self-test assertions passed.")
    return 0


def main(argv: list[str]) -> int:
    args = argv[1:]
    today = _dt.date.today()
    registry_path = REGISTRY_PATH
    series_path = SERIES_PATH
    state_path = STATE_PATH
    dry_run = force = False

    def take(flag: str) -> str | None:
        if flag in args:
            i = args.index(flag)
            val = args[i + 1] if i + 1 < len(args) else None
            del args[i:i + 2]
            return val
        return None

    d = take("--today")
    if d:
        today = parse_date(d) or today
    r = take("--registry")
    if r:
        registry_path = Path(r)
    s = take("--series")
    if s:
        series_path = Path(s)
    st = take("--state")
    if st:
        state_path = Path(st)
    if "--dry-run" in args:
        dry_run = True
        args.remove("--dry-run")
    if "--force" in args:
        force = True
        args.remove("--force")

    if args and args[0] == "--self-test":
        return self_test()
    if args and args[0] in ("-h", "--help"):
        print(__doc__)
        return 0
    if args:
        print(f"unknown argument: {args[0]} (see --help)", file=sys.stderr)
        return 2

    return run(
        today, registry_path, series_path, state_path,
        dry_run=dry_run, force=force,
    )


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
