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
#   flagged     #[ignore]                     (no reason)
#   flagged         #[ignore]                 (indented)
#   flagged     #[ignore] // some comment     (comment is not a reason string)
#   flagged     #[ ignore ]                   (inner whitespace, still bare)
#   OK          #[ignore = "requires DB"]     (has a reason string)
#   OK          #[ignore = "BIT-440: ..."]    (reason + tracking id)
#   ignored     /// `#[ignore]` ...           (doc comment — starts with `/`)
#   ignored     //! marked #[ignore] ...       (doc comment)
#   ignored     // #[ignore]                   (commented-out attribute)
#   ignored     let s = "#[ignore]";           (string literal — not at line start)
#
# The leading `#` anchor (`^[[:space:]]*#\[`) is what excludes doc comments,
# commented-out attributes, and in-string mentions: those lines start with
# `/`, `l`, etc. — never with `#[` after optional indentation. The valid
# `#[ignore = "..."]` form has content between `ignore` and `]`, so it does
# not match the bare pattern either.

set -euo pipefail

# Single source of truth for the matcher — a bare `#[ignore]` attribute at
# the start of a line (leading indentation allowed; optional inner
# whitespace around `ignore`). POSIX ERE so plain `grep -E` and bash
# `=~` agree.
BARE_IGNORE_ERE='^[[:space:]]*#\[[[:space:]]*ignore[[:space:]]*\]'

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

  # Negative — must NOT be flagged.
  run "reason string"              "ok"   '#[ignore = "requires DB"]'
  run "reason + ticket, indented"  "ok"   '    #[ignore = "BIT-440: workspace hostage"]'
  run "reason with metachars"      "ok"   '#[ignore = "flakes; run --test-threads=1"]'
  run "doc comment ///"            "ok"   '/// `#[ignore]`; run them with:'
  run "doc comment //!"            "ok"   '//! marked #[ignore] until DB is seeded'
  run "commented-out attribute"    "ok"   '// #[ignore]'
  run "string literal mention"     "ok"   'let s = "#[ignore]";'
  run "unrelated attribute"        "ok"   '#[test]'
  run "cfg attribute"              "ok"   '#[cfg(test)]'
  run "ignore-list form"           "ok"   '#[ignore(note = "x")]'
  run "empty line"                 "ok"   ''

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
