#!/bin/bash
# CI backend test sharder (issue #2222, generalized in #2248)
# -----------------------------------------------------------
# The backend `test` CI job used to run ONE monolithic
# `cargo test --workspace` that took ~107-119 min — right up against the
# `timeout --kill-after=1m 150m` ceiling. When the wrapper SIGKILLed the run,
# cargo reported whichever target was in flight as "failed" (observed:
# `-p common --lib`), even though nothing in that target is slow — it was pure
# wall-clock exhaustion, not a real test failure. That wedged every open
# backend PR on the shared `test` gate.
#
# The server crates under `servers/*` own the bulk of the integration test
# binaries (each statically links its whole server and provisions its own
# `#[sqlx::test]` database), so they dominate the wall time. This script
# deterministically shards ALL server integration test binaries across N
# parallel CI runners so no single runner approaches the timeout.
#
# Issue #2248 follow-up: previously ONLY api-server's ~199 integration binaries
# were sharded, and everything else — including reality-server's 32 top-level
# integration binaries — rode sequentially on the single `rest` leg. That made
# `test-rest` the new un-parallelized bottleneck (and it reintroduced the exact
# #2222 wedge as reality-server grew, since the "bump the matrix" escape hatch
# only relieves the shards, not `rest`). This script now enumerates EVERY
# server crate's integration binaries and partitions them together, so the
# heavy integration load is spread evenly across the shards and `rest` only
# carries lightweight unit/lib/doc tests.
#
# Two invocation modes, so no shard is heavier than the others:
#
#   ci-test-shard.sh <shard_index> <shard_count>
#     Runs THIS shard's slice of the server integration test binaries and
#     NOTHING else. Integration test binaries (top-level `servers/*/tests/*.rs`,
#     one cargo `--test` target each) across all server crates are enumerated as
#     `crate:test` pairs, sorted, and hash-partitioned by sorted position modulo
#     the shard count — new test files (in any server crate) distribute
#     automatically, no list to maintain. EVERY shard (0 included) runs an
#     identically-shaped, roughly equal-sized workload. Selected targets are
#     grouped per crate and run with one `cargo test -p <crate> --test ...`
#     invocation per crate.
#
#   ci-test-shard.sh rest
#     Runs everything that is NOT a server integration test binary, exactly
#     once: all non-server library crates' tests
#     (`--workspace` minus every sharded server crate) plus each server crate's
#     own `--lib`/`--bins`/`--doc` tests (unit tests + doctests). This is a
#     SEPARATE parallel CI job (see backend.yml `test-rest`) — off the shard hot
#     path — and the required `test` aggregator gates on it too, so coverage
#     stays exactly-once with no false green (#1672). `--doc` closes the #2248
#     finding-3 gap where api-server doctests were silently dropped.
#
# Together the two modes partition the full workspace test surface with no
# overlap and no gap: every workspace test runs exactly once across
# {shard 0..N-1} ∪ {rest}.
#
# Env (provided by the CI job): DATABASE_URL, RUST_ENV, plus the usual
# CARGO_PROFILE_*_DEBUG / CARGO_BUILD_JOBS memory/disk mitigations.
#
# Exit codes:
#   0  every cargo invocation on this shard/leg passed
#   1  at least one cargo invocation failed (or timed out)
#   64 usage error
set -uo pipefail

usage() {
  echo "usage: $0 <shard_index> <shard_count>   # run this shard's server integration partition" >&2
  echo "       $0 rest                            # run all non-integration workspace tests exactly once" >&2
}

# Run from the backend/ crate root regardless of caller CWD.
cd "$(dirname "$0")/.." || exit 64

# Per-invocation wall-clock guard. Each shard/leg is a fraction of the old
# monolith, so 120m SIGTERM + 1m-later SIGKILL leaves comfortable margin under
# the job's timeout-minutes while still guaranteeing an unignorable stop if a
# test genuinely hangs (a hung cargo test process ignores SIGTERM; SIGKILL
# cannot be ignored — see the backend.yml history for BIT-334). Semantics kept
# from the monolith: --no-fail-fast (report every failing binary in one run,
# PAP-123) and --test-threads=4 (cap concurrency to avoid Postgres pool
# exhaustion / silent hangs, BIT-334).
run() {
  echo "+ timeout --kill-after=1m 120m cargo test $*"
  timeout --kill-after=1m 120m cargo test "$@" --no-fail-fast -- --test-threads=4
}

# The set of server crates whose integration binaries are sharded. A crate
# belongs here iff it has top-level `tests/*.rs` integration targets that we
# want spread across shards; its `--lib`/`--bins`/`--doc` unit tests stay on the
# `rest` leg. Kept in sync with the `--exclude` list in `rest` mode below so the
# partition covers the workspace exactly once. Deriving the list from the
# filesystem keeps it self-maintaining as servers are added/removed.
# Intersect with actual workspace members: servers/* dirs that are
# workspace-excluded (deploy-server since 2026-07-22 — sqlite feature
# isolation, own workspace + deploy-server.yml CI) still exist on disk but
# cannot be addressed with `-p` here ("package ID specification did not
# match any packages").
workspace_members="$(cargo metadata --no-deps --format-version=1 \
  | jq -r '.packages[].name')" || exit 64
mapfile -t server_crates < <(
  find servers -maxdepth 2 -type d -name tests -printf '%h\n' \
    | while read -r d; do
        # Only crates that actually have top-level integration test targets.
        if find "$d/tests" -maxdepth 1 -name '*.rs' -print -quit | grep -q .; then
          basename "$d"
        fi
      done | sort -u \
    | grep -Fx -f <(printf '%s\n' "$workspace_members")
)

# Enumerate every server integration test binary as a `crate:test` pair.
# Top-level `tests/*.rs` files are each a distinct cargo `--test` target named
# by basename; the `tests/common/` helper subdirectory is a module, not a
# target, and is excluded by -maxdepth 1.
mapfile -t all_tests < <(
  for crate in "${server_crates[@]}"; do
    find "servers/$crate/tests" -maxdepth 1 -name '*.rs' -printf '%f\n' \
      | sed 's/\.rs$//' \
      | while read -r t; do echo "$crate:$t"; done
  done | sort
)

# --- Mode: "rest" — everything that is NOT a server integration binary ---
if [ "$#" -eq 1 ] && [ "$1" = "rest" ]; then
  echo "rest leg: non-server workspace tests + each server crate's lib/bins/doc unit tests"
  rc=0
  # Everything that is NOT a sharded server crate, run exactly once.
  exclude_args=()
  for crate in "${server_crates[@]}"; do
    exclude_args+=(--exclude "$crate")
  done
  run --workspace "${exclude_args[@]}" || rc=1
  # Each server crate's unit tests (lib + bins) and doctests. Its integration
  # binaries live on the shards; --doc closes the #2248 finding-3 gap.
  # cargo forbids mixing --doc with other target selectors, so doctests run
  # as a separate invocation.
  for crate in "${server_crates[@]}"; do
    run -p "$crate" --lib --bins || rc=1
    run -p "$crate" --doc || rc=1
  done
  exit "$rc"
fi

# --- Mode: "<idx> <count>" — this shard's server integration partition ---
if [ "$#" -ne 2 ]; then
  usage
  exit 64
fi
idx="$1"
total="$2"
if ! [[ "$idx" =~ ^[0-9]+$ && "$total" =~ ^[0-9]+$ ]] || [ "$total" -lt 1 ] || [ "$idx" -ge "$total" ]; then
  echo "error: need 0 <= shard_index($idx) < shard_count($total), count >= 1" >&2
  exit 64
fi

# Partition: pair at sorted position P runs on shard (P % total). Group the
# selected `crate:test` pairs per crate so each crate needs at most one cargo
# invocation on this shard.
declare -A sel_by_crate
pos=0
for pair in "${all_tests[@]}"; do
  if [ "$(( pos % total ))" -eq "$idx" ]; then
    crate="${pair%%:*}"
    test="${pair#*:}"
    sel_by_crate["$crate"]+=" --test $test"
  fi
  pos=$(( pos + 1 ))
done

selected_count=0
for crate in "${!sel_by_crate[@]}"; do
  # shellcheck disable=SC2086
  set -- ${sel_by_crate[$crate]}
  selected_count=$(( selected_count + $# / 2 ))
done

echo "shard $idx/$total: ${#all_tests[@]} server integration targets total across ${#server_crates[@]} crates, $selected_count on this shard"

if [ "$selected_count" -eq 0 ]; then
  echo "shard $idx: no server integration targets assigned — nothing to do"
  exit 0
fi

rc=0
for crate in $(printf '%s\n' "${!sel_by_crate[@]}" | sort); do
  # shellcheck disable=SC2086
  run -p "$crate" ${sel_by_crate[$crate]} || rc=1
done
exit "$rc"
