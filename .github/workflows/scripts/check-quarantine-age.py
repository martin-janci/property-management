#!/usr/bin/env python3
"""Quarantine-aging gate — every `#[ignore]`d test must have an owner + deadline.

Why this exists
---------------
`ignore-reason-gate.yml` already enforces that every Rust ``#[ignore]`` carries a
*reason* string (issue #2032 — no silent, unexplained quarantine). A reason is
necessary but not sufficient: a quarantine with a reason but no owner and no
deadline still rots forever. PR #3675 ("release 248 quarantined tests") released
~226 tests in one pass precisely because the backlog of reason-only quarantines
had grown unbounded with nobody on the hook to burn it down.

This gate adds the missing half of the contract:

    Every quarantined test MUST have an OWNER and a DEADLINE, and a quarantine
    whose deadline has passed FAILS CI until it is fixed or re-triaged.

Two places can carry the owner+deadline for a test:

1. Inline, in the ignore reason string itself::

       #[ignore = "BIT-440: workspace hostage (owner=pm-backend; due=2027-02-19)"]

   The reason must contain ``owner=<role>`` and ``due=YYYY-MM-DD`` tokens.

2. The quarantine registry (``.github/quarantine/registry.toml``), keyed by
   ``<relative-path>::<test-fn>``. This keeps owner/deadline for the large
   grandfathered set in one reviewable census instead of scattering invented
   metadata across 40 test files. Adding a quarantine without an inline token
   means adding a registry row — so a new quarantine is still visible in the
   diff and can never be silent.

A test needs *exactly one* source of owner+deadline (inline wins if both exist).

Checks (all over the committed tree, not just the diff)
-------------------------------------------------------
  FAIL  a quarantined test has neither an inline owner+due token nor a registry
        row -> "unregistered quarantine"
  FAIL  an effective deadline (inline or registry) is in the past -> "aged out"
  FAIL  an owner is not a known pm-* role
  FAIL  a registry row is malformed (missing owner/due, bad date, bad owner)
  WARN  a registry row has no matching quarantined test -> "stale row" (clean up)

Modes
-----
  (default)     scan the tree, print findings, exit 1 on any FAIL
  --summary     print the weekly burn-down metric (never fails) — a one-line
                QUARANTINE-BURNDOWN token line for the dispatcher plus a
                markdown table (owner breakdown, deadline, required rows/week).
                See .github/quarantine/cadence.md.
  --census      print the discovered quarantine census as TOML rows (used to
                regenerate the registry after releasing/adding quarantines)
  --self-test   run the built-in fixture suite over the parser + policy logic
  --today DATE  override "today" (YYYY-MM-DD) — used by --summary and --self-test

The scan is DB-free and fast: pure text, no cargo, no network.
"""
from __future__ import annotations

import datetime as _dt
import re
import sys
from pathlib import Path

try:  # Python 3.11+ stdlib; GitHub ubuntu runners have it.
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - very old runners
    tomllib = None  # type: ignore[assignment]

REPO_ROOT = Path(__file__).resolve().parents[3]
REGISTRY_PATH = REPO_ROOT / ".github" / "quarantine" / "registry.toml"
SCAN_ROOTS = ["backend"]

# Known owner roles (mirrors .research/management/roles/*.md). A quarantine
# owner must be one of these so "owner" is an accountable team, not a typo.
KNOWN_OWNERS = {
    "pm-backend",
    "pm-data",
    "pm-devops",
    "pm-frontend",
    "pm-integration",
    "pm-qa",
    "pm-scrum-master",
    "pm-security",
    "pm-tech-lead",
}

# An attribute line that is a real `#[ignore = "..."]` (or
# `#[cfg_attr(.., ignore = "..")]`). Anchored on `^\s*#[` so doc comments
# (`/// #[ignore]`), commented-out attrs (`// #[ignore]`) and string-literal
# mentions never match — same anchor the sibling bash gate uses.
IGNORE_ATTR_RE = re.compile(r'^\s*#\[(?:cfg_attr\(.*,\s*)?ignore\s*=')
# A function item, possibly `pub`, `async`, with generics/visibility noise.
FN_RE = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)')
# Inline owner / due tokens inside a reason string.
OWNER_TOKEN_RE = re.compile(r'owner\s*=\s*([A-Za-z0-9_.-]+)')
DUE_TOKEN_RE = re.compile(r'due\s*=\s*(\d{4}-\d{2}-\d{2})')


class Finding:
    __slots__ = ("level", "key", "msg")

    def __init__(self, level: str, key: str, msg: str) -> None:
        self.level = level  # "FAIL" | "WARN"
        self.key = key
        self.msg = msg


def parse_date(s: str) -> _dt.date | None:
    try:
        return _dt.date.fromisoformat(s)
    except ValueError:
        return None


def find_quarantines(lines: list[str]) -> list[tuple[int, str, str | None]]:
    """Return (line_index, reason_text, fn_name) for each `#[ignore=..]` in file.

    `reason_text` is best-effort: the text between the first pair of quotes on
    the attribute line (sufficient for the inline owner/due tokens, which the
    convention places at the end of the reason — on the same source line for a
    single-line reason). `fn_name` is the first `fn` at or below the attribute,
    skipping intervening attribute / blank / comment lines.
    """
    out: list[tuple[int, str, str | None]] = []
    for i, line in enumerate(lines):
        if not IGNORE_ATTR_RE.match(line):
            continue
        # Find where the attribute ends. A single-line reason has a complete
        # `"..."` on this line; a Rust `\`-continued reason spans several source
        # lines, closed by the line carrying the matching `"`.
        attr_end = i
        single = re.search(r'ignore\s*=\s*"([^"]*)"', line)
        if single:
            reason = single.group(1)
        else:
            reason_parts = [line]
            k = i + 1
            while k < len(lines):
                reason_parts.append(lines[k])
                if '"' in lines[k]:  # closing quote of the reason string
                    attr_end = k
                    break
                k += 1
            joined = " ".join(p.strip().rstrip("\\").strip() for p in reason_parts)
            jm = re.search(r'ignore\s*=\s*"(.*?)"', joined)
            reason = jm.group(1) if jm else joined
        # First `fn` at/below the (possibly multi-line) attribute, skipping
        # intervening attribute / comment / blank lines.
        fn_name = None
        j = attr_end + 1
        while j < len(lines):
            nxt = lines[j]
            fm = FN_RE.match(nxt)
            if fm:
                fn_name = fm.group(1)
                break
            stripped = nxt.strip()
            if stripped and not (
                stripped.startswith("#[")
                or stripped.startswith("//")
                or stripped.startswith("/*")
                or stripped.startswith("*")
            ):
                break
            j += 1
        out.append((i, reason, fn_name))
    return out


def scan_tree(root: Path) -> dict[str, dict]:
    """Map `relpath::fn` -> {path, line, reason, inline_owner, inline_due}."""
    census: dict[str, dict] = {}
    for scan_root in SCAN_ROOTS:
        base = root / scan_root
        if not base.exists():
            continue
        for rs in sorted(base.rglob("*.rs")):
            if "/target/" in str(rs):
                continue
            try:
                lines = rs.read_text(encoding="utf-8", errors="replace").splitlines()
            except OSError:
                continue
            rel = rs.relative_to(root).as_posix()
            for idx, reason, fn in find_quarantines(lines):
                key = f"{rel}::{fn or '?line%d' % (idx + 1)}"
                om = OWNER_TOKEN_RE.search(reason)
                dm = DUE_TOKEN_RE.search(reason)
                census[key] = {
                    "path": rel,
                    "line": idx + 1,
                    "reason": reason,
                    "fn": fn,
                    "inline_owner": om.group(1) if om else None,
                    "inline_due": dm.group(1) if dm else None,
                }
    return census


def load_registry(path: Path) -> dict[str, dict]:
    if not path.exists():
        return {}
    if tomllib is None:  # pragma: no cover
        raise SystemExit("ERROR: tomllib unavailable; need Python 3.11+ to read the registry")
    with path.open("rb") as fh:
        data = tomllib.load(fh)
    rows: dict[str, dict] = {}
    for row in data.get("quarantine", []):
        test = row.get("test")
        if test:
            rows[test] = row
    return rows


def evaluate(census: dict[str, dict], registry: dict[str, dict], today: _dt.date) -> list[Finding]:
    findings: list[Finding] = []

    for key, info in sorted(census.items()):
        owner = info["inline_owner"]
        due_str = info["inline_due"]
        source = "inline"
        if owner is None or due_str is None:
            row = registry.get(key)
            if row is None:
                findings.append(Finding(
                    "FAIL", key,
                    f"unregistered quarantine ({info['path']}:{info['line']}) — "
                    f"add an inline (owner=..; due=YYYY-MM-DD) token or a "
                    f".github/quarantine/registry.toml row for '{key}'",
                ))
                continue
            owner = owner or row.get("owner")
            due_str = due_str or row.get("due")
            source = "registry"

        if not owner:
            findings.append(Finding("FAIL", key, f"missing owner ({source})"))
        elif owner not in KNOWN_OWNERS:
            findings.append(Finding("FAIL", key, f"unknown owner '{owner}' ({source})"))

        if not due_str:
            findings.append(Finding("FAIL", key, f"missing deadline ({source})"))
        else:
            due = parse_date(str(due_str))
            if due is None:
                findings.append(Finding("FAIL", key, f"invalid deadline '{due_str}' ({source}) — want YYYY-MM-DD"))
            elif due < today:
                findings.append(Finding(
                    "FAIL", key,
                    f"quarantine aged out: due {due_str} < today {today.isoformat()} "
                    f"({info['path']}:{info['line']}, owner={owner}) — fix the test or re-triage with a new deadline",
                ))

    # Stale registry rows (test no longer quarantined) -> advisory.
    for key in sorted(set(registry) - set(census)):
        findings.append(Finding("WARN", key, "stale registry row — no matching quarantined test; remove it"))

    return findings


def resolve_effective(
    census: dict[str, dict], registry: dict[str, dict]
) -> list[tuple[str, str | None, _dt.date | None]]:
    """Yield (key, owner, due_date) per quarantine, inline-wins-then-registry.

    Mirrors the source-selection in ``evaluate`` but keeps only the two fields
    the burn-down metric needs. ``owner``/``due`` may be ``None`` when a test is
    unregistered — the gate (``evaluate``) is what FAILs on that; the metric
    merely counts it under ``(unassigned)`` so a hole in the registry is still
    visible in the weekly number rather than silently dropped.
    """
    out: list[tuple[str, str | None, _dt.date | None]] = []
    for key, info in census.items():
        owner = info["inline_owner"]
        due_str = info["inline_due"]
        if owner is None or due_str is None:
            row = registry.get(key, {})
            owner = owner or row.get("owner")
            due_str = due_str or row.get("due")
        out.append((key, owner, parse_date(str(due_str)) if due_str else None))
    return out


def summarize(
    census: dict[str, dict], registry: dict[str, dict], today: _dt.date
) -> dict:
    """Compute the weekly burn-down metric from the current census + registry.

    Returns a dict with the headline total, a per-owner breakdown, the nearest
    (soonest) deadline across all rows, the days/weeks left until it, and the
    rows-per-week burn rate required to reach the target of zero by then.
    """
    eff = resolve_effective(census, registry)
    total = len(eff)

    by_owner: dict[str, int] = {}
    for _key, owner, _due in eff:
        label = owner or "(unassigned)"
        by_owner[label] = by_owner.get(label, 0) + 1

    dues = [d for _k, _o, d in eff if d is not None]
    nearest = min(dues) if dues else None
    days_left = (nearest - today).days if nearest is not None else None
    weeks_left = round(days_left / 7, 1) if days_left is not None else None

    # Required rows/week to hit zero by the nearest deadline. If the deadline is
    # already here or past, the whole remaining backlog is "due now".
    if total == 0:
        need_per_week = 0.0
    elif days_left is not None and days_left > 0:
        need_per_week = round(total / (days_left / 7), 1)
    else:
        need_per_week = float(total)

    return {
        "total": total,
        "by_owner": by_owner,
        "nearest_due": nearest.isoformat() if nearest else None,
        "days_left": days_left,
        "weeks_left": weeks_left,
        "need_per_week": need_per_week,
        "target": 0,
    }


def format_summary(metric: dict) -> str:
    """Render the burn-down metric as a one-line token string + a markdown table.

    The first line is grep-friendly for the dispatcher (a single
    ``QUARANTINE-BURNDOWN ...`` token line it can lift verbatim into the
    pm-devops rotation / project-state). The table below it is for humans and
    for the workflow step summary. Per-owner tokens/rows are emitted in a
    deterministic (sorted) order so the line is diff-stable week to week.
    """
    owners = sorted(metric["by_owner"].items())
    owner_tokens = " ".join(f"{o}={n}" for o, n in owners)
    line = (
        f"QUARANTINE-BURNDOWN total={metric['total']} {owner_tokens} "
        f"nearest_due={metric['nearest_due']} days_left={metric['days_left']} "
        f"weeks_left={metric['weeks_left']} need_per_week={metric['need_per_week']} "
        f"target={metric['target']}"
    )

    rows = "\n".join(f"| {o} | {n} |" for o, n in owners)
    table = (
        "\n### Quarantine burn-down\n\n"
        f"- **Open quarantined tests:** {metric['total']} (target {metric['target']})\n"
        f"- **Nearest deadline:** {metric['nearest_due']} "
        f"({metric['days_left']} days / {metric['weeks_left']} weeks left)\n"
        f"- **Required burn rate:** {metric['need_per_week']} rows/week to clear by the deadline\n\n"
        "| Owner | Open rows |\n|---|---|\n"
        f"{rows}\n"
    )
    return line + "\n" + table


def run_summary(today: _dt.date) -> int:
    census = scan_tree(REPO_ROOT)
    registry = load_registry(REGISTRY_PATH)
    print(format_summary(summarize(census, registry, today)))
    return 0


def print_census(census: dict[str, dict]) -> None:
    print("# Generated by check-quarantine-age.py --census")
    print("# Fill in owner (a pm-* role) and due (YYYY-MM-DD) for each row.\n")
    for key, info in sorted(census.items()):
        reason = info["reason"].replace('"', "'")
        if len(reason) > 140:
            reason = reason[:137] + "..."
        print("[[quarantine]]")
        print(f'test = "{key}"')
        print('owner = "pm-backend"')
        print('due = "2027-02-19"')
        print(f'reason = "{reason}"')
        print()


def run_scan(today: _dt.date) -> int:
    census = scan_tree(REPO_ROOT)
    registry = load_registry(REGISTRY_PATH)
    findings = evaluate(census, registry, today)

    fails = [f for f in findings if f.level == "FAIL"]
    warns = [f for f in findings if f.level == "WARN"]

    for f in warns:
        print(f"WARN: {f.key}: {f.msg}")

    if fails:
        print()
        print(f"FAIL: {len(fails)} quarantine-policy violation(s).")
        print()
        print("  Every `#[ignore]`d test must have an OWNER (a pm-* role) and a")
        print("  DEADLINE (due=YYYY-MM-DD), and must not be past its deadline.")
        print("  Provide them inline in the reason:")
        print()
        print('    #[ignore = "<reason> (owner=pm-backend; due=2027-02-19)"]')
        print()
        print("  or add a row to .github/quarantine/registry.toml. Regenerate the")
        print("  census with:  python3 .github/workflows/scripts/check-quarantine-age.py --census")
        print()
        print("  Violations:")
        for f in fails:
            print(f"    {f.key}: {f.msg}")
        print()
        return 1

    n = len(census)
    print(f"PASS: {n} quarantined test(s); every one has an owner + an unexpired deadline.")
    if warns:
        print(f"      ({len(warns)} stale registry row(s) to clean up — see WARN lines above.)")
    return 0


# --------------------------------------------------------------------------
# Self-test
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

    # ---- parser: find_quarantines over a representative fixture ----
    fixture = '''
#[test]
fn not_ignored() {}

#[ignore = "BIT-440: workspace hostage (owner=pm-backend; due=2027-02-19)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn inline_ok(pool: PgPool) {}

/// `#[ignore]` in a doc comment — must NOT match
// #[ignore] commented out — must NOT match
let s = "#[ignore]"; // string literal — must NOT match

#[ignore = "needs DB"]
fn registry_only() {}

#[ignore = "#3104: multi-line reason spanning \\
            two source lines"]
#[tokio::test]
async fn multiline_reason() {}
'''.splitlines()
    q = find_quarantines(fixture)
    names = [fn for _, _, fn in q]
    check("parser finds exactly 3 ignores", len(q) == 3)
    check("parser maps inline_ok fn", "inline_ok" in names)
    check("parser maps registry_only fn", "registry_only" in names)
    check("parser maps multiline_reason fn", "multiline_reason" in names)
    check("parser skips doc/comment/string ignores", "not_ignored" not in names)

    # inline tokens parsed off the first line
    inline = next(r for r in q if r[2] == "inline_ok")
    check("inline owner token parsed", bool(OWNER_TOKEN_RE.search(inline[1])))
    check("inline due token parsed", bool(DUE_TOKEN_RE.search(inline[1])))

    # ---- policy: evaluate() verdicts ----
    today = _dt.date(2026, 11, 21)
    census = {
        "a.rs::inline_future": {"path": "a.rs", "line": 1, "reason": "", "fn": "inline_future",
                                 "inline_owner": "pm-backend", "inline_due": "2027-02-19"},
        "a.rs::inline_past": {"path": "a.rs", "line": 2, "reason": "", "fn": "inline_past",
                               "inline_owner": "pm-backend", "inline_due": "2026-01-01"},
        "a.rs::inline_bad_owner": {"path": "a.rs", "line": 3, "reason": "", "fn": "inline_bad_owner",
                                    "inline_owner": "nobody", "inline_due": "2027-02-19"},
        "a.rs::unregistered": {"path": "a.rs", "line": 4, "reason": "", "fn": "unregistered",
                                "inline_owner": None, "inline_due": None},
        "a.rs::from_registry": {"path": "a.rs", "line": 5, "reason": "", "fn": "from_registry",
                                 "inline_owner": None, "inline_due": None},
        "a.rs::registry_past": {"path": "a.rs", "line": 6, "reason": "", "fn": "registry_past",
                                 "inline_owner": None, "inline_due": None},
    }
    registry = {
        "a.rs::from_registry": {"test": "a.rs::from_registry", "owner": "pm-qa", "due": "2027-02-19"},
        "a.rs::registry_past": {"test": "a.rs::registry_past", "owner": "pm-qa", "due": "2025-01-01"},
        "a.rs::stale": {"test": "a.rs::stale", "owner": "pm-qa", "due": "2027-02-19"},
    }
    findings = evaluate(census, registry, today)
    fk = {f.key: f for f in findings if f.level == "FAIL"}
    wk = {f.key: f for f in findings if f.level == "WARN"}

    check("inline future deadline passes", "a.rs::inline_future" not in fk)
    check("inline past deadline fails (aged out)", "a.rs::inline_past" in fk and "aged out" in fk["a.rs::inline_past"].msg)
    check("unknown owner fails", "a.rs::inline_bad_owner" in fk and "unknown owner" in fk["a.rs::inline_bad_owner"].msg)
    check("unregistered quarantine fails", "a.rs::unregistered" in fk and "unregistered" in fk["a.rs::unregistered"].msg)
    check("registry-backed future deadline passes", "a.rs::from_registry" not in fk)
    check("registry-backed past deadline fails", "a.rs::registry_past" in fk and "aged out" in fk["a.rs::registry_past"].msg)
    check("stale registry row warns", "a.rs::stale" in wk)

    # ---- metric: summarize() over the same census/registry ----
    # Deadlines: inline_future 2027-02-19, inline_past 2026-01-01,
    # inline_bad_owner 2027-02-19, from_registry 2027-02-19,
    # registry_past 2025-01-01. Nearest across all = 2025-01-01 (past).
    metric = summarize(census, registry, today)
    check("metric counts every quarantine", metric["total"] == 6)
    check(
        "metric owner breakdown sums to total",
        sum(metric["by_owner"].values()) == metric["total"],
    )
    check(
        "metric buckets unregistered under (unassigned)",
        metric["by_owner"].get("(unassigned)") == 1,
    )
    check(
        "metric picks the nearest (soonest) deadline",
        metric["nearest_due"] == "2025-01-01",
    )
    check(
        "metric past-deadline burn rate = whole backlog",
        metric["days_left"] < 0 and metric["need_per_week"] == 6.0,
    )

    # Future-only deadline -> a finite positive per-week rate.
    future = {
        "x.rs::a": {"path": "x.rs", "line": 1, "reason": "", "fn": "a",
                    "inline_owner": "pm-devops", "inline_due": "2027-02-19"},
        "x.rs::b": {"path": "x.rs", "line": 2, "reason": "", "fn": "b",
                    "inline_owner": "pm-backend", "inline_due": "2027-02-19"},
    }
    fmetric = summarize(future, {}, today)
    check("metric future deadline yields positive need_per_week", fmetric["need_per_week"] > 0)
    check("metric line carries the token header", format_summary(fmetric).startswith("QUARANTINE-BURNDOWN total=2"))
    check(
        "metric owner tokens are sorted (pm-backend before pm-devops)",
        format_summary(fmetric).index("pm-backend=") < format_summary(fmetric).index("pm-devops="),
    )

    # Empty census -> zero target met, no crash.
    empty = summarize({}, {}, today)
    check("metric empty census has total 0 and need 0", empty["total"] == 0 and empty["need_per_week"] == 0.0)

    if failures:
        print(f"\n{failures} self-test assertion(s) failed", file=sys.stderr)
        return 1
    print("\nAll self-test assertions passed.")
    return 0


def main(argv: list[str]) -> int:
    today = _dt.date.today()
    args = argv[1:]
    if "--today" in args:
        i = args.index("--today")
        today = parse_date(args[i + 1]) or today
        del args[i:i + 2]
    if args and args[0] == "--self-test":
        return self_test()
    if args and args[0] == "--summary":
        return run_summary(today)
    if args and args[0] == "--census":
        print_census(scan_tree(REPO_ROOT))
        return 0
    if args and args[0] in ("-h", "--help"):
        print(__doc__)
        return 0
    return run_scan(today)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
