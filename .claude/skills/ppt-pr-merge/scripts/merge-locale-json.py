#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
merge-locale-json.py — auto-resolve JSON-locale merge conflicts by key-union.

Context (dispatcher stall #3357): the Phase 5.6 auto-rebaser in ppt-pr-merge
only resolves *mechanical* conflicts (sqlx offline data, Cargo.lock, generated
clients, lockfiles). The recurring non-mechanical conflict that dead-ends at
the rebase cap is two PRs adding *different* keys to the same i18n locale file
(`frontend/apps/*/src/locales/*.json`, `next-intl` message bundles, etc.).
Those are semantically trivial — the union of both sides' keys — but git sees
the whole JSON object as conflicting.

This helper performs a *structural* recursive three-way merge of two sides
against their common base:

  * A key added by exactly one side is taken (key-union).
  * A key both sides left equal is taken unchanged.
  * A key one side changed and the other left at the base value takes the
    changed value.
  * A key deleted by one side and untouched by the other is deleted.
  * A key both sides changed to *different* values — or changed-vs-deleted, or
    object-vs-scalar — is a GENUINE conflict. The script refuses (exit 2) and
    names the conflicting dotted key paths. It NEVER silently picks a winner,
    preserving ppt-pr-merge's "real code conflicts always abort" contract.

Ordering is deterministic. Default ("base-first") preserves base key order,
then appends ours-only keys (ours order), then theirs-only keys (theirs order)
— minimal diff churn. `--sort-keys` emits strict alphabetical order (side
independent). Output is UTF-8, 2-space indented, with a trailing newline, and
non-ASCII characters are preserved verbatim (ensure_ascii=False) to match how
these bundles are checked in.

Modes
-----
Git-index mode (use inside a conflicted `git merge`):
    merge-locale-json.py --git <path-in-repo>
  Reads the three conflict stages from the index (`:1:` base, `:2:` ours,
  `:3:` theirs) and writes the merged result back to <path-in-repo>. A missing
  base stage (added-on-both-sides, no common ancestor blob) is treated as {}.
  On success the caller still has to `git add <path>` + continue the merge.

File mode (unit-testable, no git required):
    merge-locale-json.py --ours O.json --theirs T.json \
        [--base B.json] [--output OUT.json]
  Writes to --output, or stdout if omitted.

Exit codes
----------
  0  merged cleanly (key-union succeeded)
  2  genuine conflict (same key changed differently, or structural mismatch)
  1  usage / IO / parse error
"""

import argparse
import json
import subprocess
import sys

_MISSING = object()  # sentinel: key absent on a side


def _eprint(*a):
    print(*a, file=sys.stderr)


def merge3(base, ours, theirs, path=""):
    """Recursive three-way merge of two JSON values against a common base.

    Returns (merged_value, conflicts) where conflicts is a list of dotted key
    paths that could not be auto-resolved. When conflicts is non-empty the
    merged_value is a best-effort partial and MUST NOT be written.
    """
    conflicts = []

    # Identical sides (incl. both-deleted / both-equal object) — trivial.
    if ours == theirs:
        return ours, conflicts

    # One side unchanged relative to base → take the other side's value.
    if base is not _MISSING:
        if ours == base:
            return theirs, conflicts
        if theirs == base:
            return ours, conflicts

    # Both sides are objects → recurse key-by-key (this is where key-union
    # happens: disjoint added keys merge without conflict).
    if isinstance(ours, dict) and isinstance(theirs, dict):
        base_obj = base if isinstance(base, dict) else {}
        merged = {}

        def _take(key):
            b = base_obj.get(key, _MISSING)
            o = ours.get(key, _MISSING)
            t = theirs.get(key, _MISSING)
            kp = f"{path}.{key}" if path else key

            # Deletion handling: a side that dropped a base key.
            if o is _MISSING or t is _MISSING:
                present = o if o is not _MISSING else t
                other_missing_side = t if o is _MISSING else o
                # If the key never existed in base, it's an add on the
                # present side → take it (pure key-union).
                if b is _MISSING:
                    return present, []
                # Key existed in base and one side deleted it.
                if other_missing_side == b:
                    # other side kept it unchanged, this side deleted → delete.
                    return _MISSING, []
                # other side changed it AND this side deleted → real conflict.
                return _MISSING, [kp]

            sub, sub_conf = merge3(b, o, t, kp)
            return sub, sub_conf

        # Deterministic key order: base order, then ours-new, then theirs-new.
        ordered = []
        seen = set()
        for k in base_obj:
            if (k in ours or k in theirs) and k not in seen:
                ordered.append(k)
                seen.add(k)
        for k in ours:
            if k not in seen:
                ordered.append(k)
                seen.add(k)
        for k in theirs:
            if k not in seen:
                ordered.append(k)
                seen.add(k)

        for k in ordered:
            val, conf = _take(k)
            conflicts.extend(conf)
            if val is not _MISSING:
                merged[k] = val
        return merged, conflicts

    # Scalars (or object-vs-scalar) that differ and neither equals base:
    # genuine conflict. Report this node.
    conflicts.append(path or "<root>")
    return ours, conflicts


def _reorder_sorted(value):
    """Recursively re-key objects into strict alphabetical order."""
    if isinstance(value, dict):
        return {k: _reorder_sorted(value[k]) for k in sorted(value)}
    return value


def _load(fp):
    with open(fp, "r", encoding="utf-8") as fh:
        return json.load(fh)


def _git_stage(path, stage):
    """Return parsed JSON for a conflict stage, or _MISSING if absent."""
    try:
        out = subprocess.run(
            ["git", "show", f":{stage}:{path}"],
            capture_output=True, check=True, text=True,
        ).stdout
    except subprocess.CalledProcessError:
        return _MISSING
    return json.loads(out)


def _dump(value, indent, sort_keys):
    if sort_keys:
        value = _reorder_sorted(value)
    return json.dumps(value, ensure_ascii=False, indent=indent) + "\n"


def main(argv=None):
    ap = argparse.ArgumentParser(
        description="Key-union three-way merge for i18n locale JSON conflicts.",
    )
    ap.add_argument("--git", metavar="PATH",
                    help="resolve an in-index conflict: read :1:/:2:/:3: "
                         "stages for PATH and write merged result to PATH")
    ap.add_argument("--base", metavar="FILE",
                    help="common-ancestor JSON (file mode); omit for 2-way")
    ap.add_argument("--ours", metavar="FILE", help="our side JSON (file mode)")
    ap.add_argument("--theirs", metavar="FILE",
                    help="their side JSON (file mode)")
    ap.add_argument("--output", metavar="FILE",
                    help="write merged JSON here (file mode; default stdout)")
    ap.add_argument("--indent", type=int, default=2,
                    help="output indent width (default: 2)")
    ap.add_argument("--sort-keys", action="store_true",
                    help="emit strict alphabetical key order instead of the "
                         "default minimal-churn base-first order")
    args = ap.parse_args(argv)

    try:
        if args.git:
            path = args.git
            base = _git_stage(path, 1)
            ours = _git_stage(path, 2)
            theirs = _git_stage(path, 3)
            if ours is _MISSING or theirs is _MISSING:
                _eprint(f"merge-locale-json: {path}: missing ours/theirs "
                        f"stage in index — not a both-sides-present conflict")
                return 1
        else:
            if not args.ours or not args.theirs:
                _eprint("merge-locale-json: file mode needs --ours and "
                        "--theirs")
                return 1
            base = _load(args.base) if args.base else _MISSING
            ours = _load(args.ours)
            theirs = _load(args.theirs)
    except (OSError, json.JSONDecodeError) as exc:
        _eprint(f"merge-locale-json: read/parse error: {exc}")
        return 1

    merged, conflicts = merge3(base, ours, theirs)

    if conflicts:
        _eprint("merge-locale-json: GENUINE conflict — same key changed on "
                "both sides, cannot key-union. Conflicting paths:")
        for c in sorted(set(conflicts)):
            _eprint(f"  {c}")
        return 2

    text = _dump(merged, args.indent, args.sort_keys)

    try:
        if args.git:
            with open(args.git, "w", encoding="utf-8") as fh:
                fh.write(text)
            _eprint(f"merge-locale-json: {args.git}: key-union resolved "
                    f"(remember to `git add`)")
        elif args.output:
            with open(args.output, "w", encoding="utf-8") as fh:
                fh.write(text)
        else:
            sys.stdout.write(text)
    except OSError as exc:
        _eprint(f"merge-locale-json: write error: {exc}")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
