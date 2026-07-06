#!/usr/bin/env bash
# Fails when a Rust `#[ignore]` attribute carries no reason string.
#
# Why this exists
# ---------------
# PR #1980 (BIT-416 admin happy-path backfill) bundled bare, unexplained
# `#[ignore]` attributes onto tests in nine unrelated suites — including
# authz/security regression tests (see issue #2032). A bare `#[ignore]`
# silently quarantines coverage with no ticket, no reason, and no reviewer
# signal. The contract this gate enforces:
#
#   Every `#[ignore]` MUST be written as `#[ignore = "<reason>"]`.
#
# A machine-readable reason string means the quarantine is visible in the
# diff, greppable in CI, and un-ignorable-without-explanation. A trailing
# `//` comment does NOT satisfy the contract — the reason must live in the
# attribute itself so tooling (and this gate) can see it.
#
# Two modes (mirrors detect-test-file.sh):
#   - Default: scans Rust files (args, or every tracked `*.rs` when none)
#     for bare `#[ignore]`. Prints each offender as `file:line:content`
#     and exits 1 if any are found; exits 0 (with a PASS line) otherwise.
#   - --self-test: runs the built-in fixture suite over the line classifier
#     and exits 0 iff every fixture produces the expected verdict. Used by
#     the `self-test` job in the workflow to catch matcher regressions.
#
# What counts as "bare"
# ---------------------
#   flagged     #[ignore]                       (no reason)
#   flagged         #[ignore]                   (indented)
#   flagged     #[ignore] // some comment       (comment is not a reason string)
#   flagged     #[ ignore ]                     (inner whitespace, still bare)
#   flagged     #[test] #[ignore]               (bare ignore after another attr)
#   flagged     #[cfg_attr(test, ignore)]       (conditional but still reason-free)
#   OK          #[ignore = "requires DB"]       (has a reason string)
#   OK          #[ignore = "BIT-440: ..."]      (reason + tracking id)
#   OK          #[cfg_attr(test, ignore = "…")] (cfg-gated ignore with a reason)
#   ignored     /// `#[ignore]` ...             (doc comment — starts with `/`)
#   ignored     //! marked #[ignore] ...        (doc comment)
#   ignored     // #[ignore]                    (commented-out attribute)
#   ignored     let s = "#[ignore]";            (string literal — not at line start)
#
# The leading `#[` anchor (`^[[:space:]]*#\[`) is what excludes doc comments,
# commented-out attributes, and in-string mentions: those lines start with
# `/`, `l`, etc. — never with `#[` after optional indentation. Once a line is
# known to be an attribute line, the matcher looks for a bare `#[ignore]`
# token anywhere on it (so `#[test] #[ignore]` is caught, not just a
# line-leading one) and for a reason-free `ignore` inside `cfg_attr(...)`
# (`ignore` followed by `,` or `)` rather than `=`). The valid
# `#[ignore = "..."]` and `#[cfg_attr(test, ignore = "...")]` forms have an
# `=` after `ignore`, so they do not match. A `#[ignore]` buried inside a
# block comment or a multi-line string is out of scope by design — the
# matcher keys on attribute lines, not on every mention of the token.

set -euo pipefail

# Single source of truth for the matcher. Built from named parts (POSIX ERE
# so plain `grep -E` and bash `=~` agree — no `\b`, no PCRE backreferences):
#
#   BARE_TOKEN     a reason-free `#[ignore]` token (optional inner whitespace).
#   CFG_ATTR_BARE  a reason-free `ignore` inside `cfg_attr(...)` — preceded by
#                  `(`/`,` and followed by `,`/`)`, i.e. NOT `ignore = "..."`.
#
# The composed pattern requires the line to be an attribute line
# (`^[[:space:]]*#\[` — the anchor that excludes doc comments, commented-out
# attrs, and in-string mentions) and then flags it when the ignore sits
# right after the leading `#[` (plain `#[ignore]`), or a bare `#[ignore]`
# token / bare cfg_attr ignore appears anywhere later on the line
# (`#[test] #[ignore]`, `#[cfg_attr(test, ignore)]`).
BARE_TOKEN='#\[[[:space:]]*ignore[[:space:]]*\]'
CFG_ATTR_BARE='cfg_attr\(.*[(,][[:space:]]*ignore[[:space:]]*[,)]'
BARE_IGNORE_ERE="^[[:space:]]*#\[([[:space:]]*ignore[[:space:]]*\]|.*(${BARE_TOKEN}|${CFG_ATTR_BARE}))"

# ---- classifier (single line) --------------------------------------

is_bare_ignore() {
  # Returns 0 (true) when the line is a bare `#[ignore]` attribute.
  [[ "$1" =~ $BARE_IGNORE_ERE ]]
}

# ---- scan mode ------------------------------------------------------

scan() {
  local -a files=("$@")
  if [ "${#files[@]}" -eq 0 ]; then
    # No paths given → scan every tracked Rust file. Fall back to `find`
    # (excluding build output) when not inside a git work tree.
    if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
      mapfile -t files < <(git ls-files '*.rs')
    else
      mapfile -t files < <(find . -name '*.rs' -not -path '*/target/*')
    fi
  fi
  [ "${#files[@]}" -eq 0 ] && return 0
  # -H forces the filename prefix even for a single file so the output is
  # always `file:line:content`. `|| true` keeps `set -e` happy on no-match
  # (grep exits 1 when nothing matches — that is the success case here).
  grep -HnE "$BARE_IGNORE_ERE" "${files[@]}" 2>/dev/null || true
}

# ---- self-test fixtures --------------------------------------------

self_test() {
  local failures=0
  local label expected line got
  run() {
    label="$1"; expected="$2"; line="$3"
    if is_bare_ignore "$line"; then got="bare"; else got="ok"; fi
    if [ "$got" != "$expected" ]; then
      printf 'FAIL: %s — expected=%s got=%s\n' "$label" "$expected" "$got" >&2
      failures=$((failures + 1))
    else
      printf 'ok:   %s\n' "$label"
    fi
  }

  # Positive — bare, must be flagged.
  run "plain bare"                 "bare" '#[ignore]'
  run "indented bare"              "bare" '    #[ignore]'
  run "tab-indented bare"          "bare" $'\t#[ignore]'
  run "bare + trailing comment"    "bare" '#[ignore] // requires database'
  run "inner whitespace"           "bare" '#[ ignore ]'
  run "bare then attr on same idx" "bare" '    #[ignore] // run with --ignored'
  run "cfg_attr bare ignore"       "bare" '#[cfg_attr(test, ignore)]'
  run "cfg_attr bare, indented"    "bare" '    #[cfg_attr(test, ignore)]'
  run "cfg_attr bare, nested cfg"  "bare" '#[cfg_attr(all(unix, test), ignore)]'
  run "same-line bare after attr"  "bare" '#[test] #[ignore]'
  run "same-line bare, indented"   "bare" '    #[test] #[ignore]'

  # Negative — must NOT be flagged.
  run "reason string"              "ok"   '#[ignore = "requires DB"]'
  run "reason + ticket, indented"  "ok"   '    #[ignore = "BIT-440: workspace hostage"]'
  run "reason with metachars"      "ok"   '#[ignore = "flakes; run --test-threads=1"]'
  run "cfg_attr with reason"       "ok"   '#[cfg_attr(test, ignore = "requires DB")]'
  run "cfg_attr reason, indented"  "ok"   '    #[cfg_attr(test, ignore = "BIT-440: db")]'
  run "cfg_attr no ignore"         "ok"   '#[cfg_attr(feature = "x", test)]'
  run "doc comment ///"            "ok"   '/// `#[ignore]`; run them with:'
  run "doc comment //!"            "ok"   '//! marked #[ignore] until DB is seeded'
  run "commented-out attribute"    "ok"   '// #[ignore]'
  run "string literal mention"     "ok"   'let s = "#[ignore]";'
  run "unrelated attribute"        "ok"   '#[test]'
  run "cfg attribute"              "ok"   '#[cfg(test)]'
  run "ignore-list form"           "ok"   '#[ignore(note = "x")]'
  run "empty line"                 "ok"   ''

  # ---- file-based scan check (drives the real grep engine) ------------
  # The fixtures above only exercise the bash `[[ =~ ]]` classifier via
  # is_bare_ignore(). CI, however, scans the tree through `grep -HnE` in
  # scan() — a different regex engine. The header comment asserts the two
  # "agree", but nothing proved it, so a pattern edit that stays valid for
  # bash `=~` while breaking `grep -E` (or vice versa) could pass self-test
  # and silently mis-scan the tree (issue #2081, follow-up to PR #2062).
  # These cases run scan() over temp fixtures and assert the exact grep
  # output CI relies on.
  local scan_tmp got_lines expected_lines neg_out
  scan_tmp="$(mktemp -d)"

  # Positive fixture: the three bare forms (lines 2, 4, 6) surrounded by the
  # annotated / doc-comment / commented-out / string-literal negatives.
  cat >"$scan_tmp/positive.rs" <<'RS'
fn quarantined_suite() {}
#[ignore]
#[ignore = "requires DB"]
#[cfg_attr(test, ignore)]
#[cfg_attr(test, ignore = "requires DB")]
#[test] #[ignore]
/// `#[ignore]` — run with --ignored
// #[ignore]
let s = "#[ignore]";
RS

  # All-negative fixture: every line is a valid or ignored form → no offenders.
  cat >"$scan_tmp/negative.rs" <<'RS'
#[ignore = "requires DB"]
#[cfg_attr(test, ignore = "BIT-440: db")]
#[test]
/// `#[ignore]` doc
// #[ignore]
let s = "#[ignore]";
RS

  # scan() emits `path:line:content`; mktemp paths carry no `:`, so field 2 is
  # the line number. Expect exactly the three bare offenders on lines 2,4,6.
  got_lines="$(scan "$scan_tmp/positive.rs" | cut -d: -f2 | paste -sd, -)"
  expected_lines="2,4,6"
  if [ "$got_lines" = "$expected_lines" ]; then
    printf 'ok:   scan(positive fixture) → offenders on lines %s\n' "$expected_lines"
  else
    printf 'FAIL: scan(positive fixture) — expected lines=%s got=%s\n' \
      "$expected_lines" "${got_lines:-<none>}" >&2
    failures=$((failures + 1))
  fi

  neg_out="$(scan "$scan_tmp/negative.rs")"
  if [ -z "$neg_out" ]; then
    printf 'ok:   scan(all-negative fixture) → no offenders (exit 0)\n'
  else
    printf 'FAIL: scan(all-negative fixture) — expected empty output, got:\n%s\n' \
      "$neg_out" >&2
    failures=$((failures + 1))
  fi

  rm -rf "$scan_tmp"

  if [ "$failures" -gt 0 ]; then
    printf '\n%s self-test fixture(s) failed\n' "$failures" >&2
    return 1
  fi
  printf '\nAll self-test fixtures passed.\n'
  return 0
}

# ---- entry ----------------------------------------------------------

case "${1:-}" in
  --self-test)
    self_test
    ;;
  -h | --help)
    printf 'usage: %s [--self-test] [PATH ...]\n' "$0"
    printf '  no args     scan every tracked *.rs file for bare #[ignore]\n'
    printf '  PATH ...    scan the given Rust files/paths instead\n'
    printf '  --self-test run the built-in matcher fixture suite\n'
    ;;
  *)
    offenders="$(scan "$@")"
    if [ -n "$offenders" ]; then
      count=$(printf '%s\n' "$offenders" | grep -c '' || true)
      echo ""
      echo "FAIL: found $count bare \`#[ignore]\` attribute(s) with no reason string."
      echo ""
      echo "  Every quarantined test must explain itself at the point of ignore:"
      echo ""
      echo "    -  #[ignore]"
      echo "    +  #[ignore = \"<why it is disabled> (+ tracking id, e.g. BIT-440)\"]"
      echo ""
      echo "  A bare #[ignore] silently drops coverage — including authz/security"
      echo "  regression tests — with no reviewer signal (see issue #2032). A"
      echo "  trailing // comment does NOT count: the reason must live in the"
      echo "  attribute so CI and future agents can see it."
      echo ""
      echo "  Offending lines:"
      printf '%s\n' "$offenders" | sed 's/^/    /'
      echo ""
      exit 1
    fi
    echo "PASS: no bare \`#[ignore]\` attributes — every ignore carries a reason string."
    ;;
esac
