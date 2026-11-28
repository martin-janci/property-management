#!/usr/bin/env bash
# canonicalize-management.sh — deterministic, idempotent on-disk form for the
# dispatcher's .research/management/*.json STATE files.
#
# Why this script exists (task pm-data-audit-research-management-churn):
#
#   An audit of two consecutive dispatcher runs over .research/management/*.json
#   found the git diff dominated by NON-SEMANTIC churn — the files carried the
#   same logical state but serialized differently run-to-run. Three distinct
#   sources:
#
#     1. ARRAY ORDER. The top-level work arrays are never sorted. The LLM
#        dispatcher re-emits rows in read/rewrite order, so a run that merely
#        touches one row can reorder the whole array — a full-array diff with
#        zero logical change. (assignments.json `.assignments`, action-list.json
#        `.items`, coverage.json `.stories`, risks.json `.items`, and the two
#        append-only archives were all observed unsorted.)
#
#     2. OBJECT KEY ORDER. Row objects are emitted with keys in varying order
#        (`task_id,branch,status,…` one run; a permuted order the next). jq
#        preserves input key order, so this permutation lands on disk verbatim
#        and shows as a per-row diff. (Confirmed: `jq -S` vs `jq .` on the live
#        action-list.json differ today.)
#
#     3. TIMESTAMPS / run-shaped fields. `generated`, `archived_at`,
#        `last_updated`, `status_changed_at`, `claimed_at`, `run_at`, and the
#        `*_at` evidence stamps change every run by design. This is GENUINE,
#        audit-bearing data — it is the IRREDUCIBLE churn and is intentionally
#        left untouched (see "Not normalized" below). A --check run reports it so
#        a reviewer can tell real churn (1 & 2) from expected churn (3) apart.
#
#   This script removes sources (1) and (2): it rewrites each known STATE file
#   into a canonical form — its work array sorted by a stable natural key, then
#   the whole document key-sorted recursively (`jq -S`). Given identical logical
#   content, the output bytes are identical regardless of input ordering, so
#   FUTURE dispatcher writes that didn't change the logical state produce an
#   empty (or timestamp-only) diff instead of a reorder storm.
#
#   Not normalized (deliberately):
#     - timestamp / `*_at` / `generated` / `run_at` values (source 3 — real data)
#     - the archives' append-only history (only reordered, never dropped/merged)
#     - any file not in the registry below (key-sorted only, never array-sorted,
#       and only with --apply-unknown)
#
# Single-writer contract: this is TOOLING for the dispatcher (Phase 6) to adopt;
# it does not run itself and does not pick live files. The caller passes the
# files explicitly. `--apply` is byte-reversible in meaning (same logical set),
# guarded by a per-array count + id-multiset invariant that fails closed.
#
# Usage:
#   .research/canonicalize-management.sh --check  <file> [<file> …]   # read-only; exit 1 if any non-canonical
#   .research/canonicalize-management.sh --apply  <file> [<file> …]   # rewrite in place (count+id guarded)
#   .research/canonicalize-management.sh --diff   <file> [<file> …]   # show the churn a canonicalize would collapse
#   .research/canonicalize-management.sh --check --apply-unknown <file>  # also key-sort unregistered files
#
# Typical Phase 6 wiring (illustrative — not run here):
#   .research/canonicalize-management.sh --apply \
#     .research/management/assignments.json \
#     .research/management/action-list.json
#   git add .research/management/assignments.json .research/management/action-list.json
#
# Exit codes:
#   0  --check: every file already canonical | --apply: all rewrites applied/no-op
#   1  --check: at least one file is NOT canonical (reorder/key churn present)
#   2  --apply: an invariant guard tripped (count or id-multiset changed) — nothing written
#  64  usage error

set -euo pipefail

MODE=""
APPLY_UNKNOWN=0
FILES=()

die() { echo "canonicalize-management: $*" >&2; exit 64; }

while [ $# -gt 0 ]; do
  case "$1" in
    --check) MODE="check" ;;
    --apply) MODE="apply" ;;
    --diff)  MODE="diff" ;;
    --apply-unknown) APPLY_UNKNOWN=1 ;;
    -h|--help)
      sed -n '2,70p' "$0"; exit 0 ;;
    --*) die "unknown flag: $1 (want --check|--apply|--diff)" ;;
    *) FILES+=("$1") ;;
  esac
  shift
done

[ -n "$MODE" ] || die "need a mode: --check | --apply | --diff"
[ "${#FILES[@]}" -gt 0 ] || die "no files given"
command -v jq >/dev/null 2>&1 || die "jq not found on PATH"

# Registry: basename -> "<array-path>\t<sort-key>\t<direction>".
# direction: asc (default) or desc (newest-first, for append-logs).
# A file not listed here is key-sorted only (never array-sorted) and only when
# --apply-unknown is passed; otherwise it is skipped with a notice.
canon_array_filter() {
  # echoes a jq expression that sorts the registered array(s); empty if unknown.
  case "$1" in
    assignments.json|assignments-archive.json)
      echo '(.assignments // []) as $a | .assignments = ($a | sort_by(.task_id // ""))' ;;
    action-list.json|action-list-archive.json)
      echo '(.items // []) as $a | .items = ($a | sort_by(.id // ""))' ;;
    coverage.json)
      echo '(.stories // []) as $a | .stories = ($a | sort_by(.id // ""))' ;;
    risks.json)
      echo '(.items // []) as $a | .items = ($a | sort_by(.id // ""))' ;;
    post-merge-review.json)
      # append-log: newest first, stable chronological order
      echo '(.runs // []) as $a | .runs = ($a | sort_by(.run_at // "") | reverse)' ;;
    *) echo "" ;;
  esac
}

# The array field + its id key, for the fail-closed invariant guard.
canon_guard_spec() {
  case "$1" in
    assignments.json|assignments-archive.json) echo '.assignments|.task_id' ;;
    action-list.json|action-list-archive.json) echo '.items|.id' ;;
    coverage.json) echo '.stories|.id' ;;
    risks.json) echo '.items|.id' ;;
    post-merge-review.json) echo '.runs|.run_at' ;;
    *) echo '' ;;
  esac
}

# Produce the canonical bytes for a file on stdout.
canonical_bytes() {
  local file="$1" base afilter
  base="$(basename "$file")"
  afilter="$(canon_array_filter "$base")"
  if [ -n "$afilter" ]; then
    # sort array(s) first, then recursively key-sort the whole doc (-S).
    jq "$afilter" "$file" | jq -S .
  else
    # unregistered: key-sort only.
    jq -S . "$file"
  fi
}

# Guard: canonicalization must preserve the array length and the id multiset.
# Returns 0 if safe, 1 if it would drop/alter rows (fail closed).
guard_ok() {
  local file="$1" canon="$2" spec arr key before_n after_n before_ids after_ids
  spec="$(canon_guard_spec "$(basename "$file")")"
  [ -n "$spec" ] || return 0   # unregistered file: key-sort only, no rows to guard
  arr="${spec%%|*}"; key="${spec##*|}"
  before_n="$(jq "($arr // []) | length" "$file")"
  after_n="$(jq "($arr // []) | length" "$canon")"
  if [ "$before_n" != "$after_n" ]; then
    echo "  GUARD: array length changed $before_n -> $after_n ($arr)" >&2
    return 1
  fi
  before_ids="$(jq -S "[($arr // [])[] | ($key // \"\")] | sort" "$file")"
  after_ids="$(jq -S "[($arr // [])[] | ($key // \"\")] | sort" "$canon")"
  if [ "$before_ids" != "$after_ids" ]; then
    echo "  GUARD: id multiset changed ($arr / $key)" >&2
    return 1
  fi
  return 0
}

RC=0
TMPS=()
cleanup() { for t in "${TMPS[@]:-}"; do [ -n "$t" ] && rm -f "$t"; done; }
trap cleanup EXIT

for file in "${FILES[@]}"; do
  if [ ! -f "$file" ]; then
    echo "canonicalize-management: skip (missing): $file" >&2
    continue
  fi
  base="$(basename "$file")"
  afilter="$(canon_array_filter "$base")"
  if [ -z "$afilter" ] && [ "$APPLY_UNKNOWN" = "0" ]; then
    echo "canonicalize-management: skip (unregistered, pass --apply-unknown to key-sort): $file" >&2
    continue
  fi

  tmp="$(mktemp)"; TMPS+=("$tmp")
  if ! canonical_bytes "$file" > "$tmp" 2>/dev/null; then
    echo "canonicalize-management: ERROR — $file is not valid JSON (or jq failed)" >&2
    RC=1
    continue
  fi

  if ! guard_ok "$file" "$tmp"; then
    echo "canonicalize-management: ABORT — invariant guard tripped for $file; not writing." >&2
    [ "$MODE" = "apply" ] && exit 2
    RC=1
    continue
  fi

  if cmp -s "$file" "$tmp"; then
    [ "$MODE" = "check" ] && echo "canonical: $file"
    continue
  fi

  case "$MODE" in
    check)
      echo "NON-CANONICAL: $file" >&2
      # Terse churn classification for the reviewer.
      keysort="$(mktemp)"; TMPS+=("$keysort")
      jq -S . "$file" > "$keysort" 2>/dev/null || true
      if ! cmp -s "$file" "$keysort"; then
        echo "  - object key order differs (source 2)" >&2
      fi
      if [ -n "$afilter" ]; then
        spec="$(canon_guard_spec "$base")"; arr="${spec%%|*}"; key="${spec##*|}"
        a_src="$(jq -c "[($arr // [])[] | ($key // \"\")]" "$file" 2>/dev/null || echo '?')"
        a_now="$(jq -c "$afilter | [($arr // [])[] | ($key // \"\")]" "$file" 2>/dev/null || echo '?')"
        [ "$a_now" != "$a_src" ] && echo "  - array order differs (source 1)" >&2
      fi
      RC=1
      ;;
    diff)
      echo "=== churn collapsed by canonicalize: $file ===" >&2
      diff <(jq . "$file") <(cat "$tmp") || true
      ;;
    apply)
      mv "$tmp" "$file"
      # drop the moved tmp from cleanup list
      echo "canonicalized: $file"
      ;;
  esac
done

exit $RC
