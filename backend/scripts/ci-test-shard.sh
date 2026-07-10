#!/bin/bash
# CI backend test sharder (issue #2222)
# ------------------------------------
# The backend `test` CI job used to run ONE monolithic
# `cargo test --workspace` that took ~107-119 min — right up against the
# `timeout --kill-after=1m 150m` ceiling. When the wrapper SIGKILLed the run,
# cargo reported whichever target was in flight as "failed" (observed:
# `-p common --lib`), even though nothing in that target is slow — it was pure
# wall-clock exhaustion, not a real test failure. That wedged every open
# backend PR on the shared `test` gate.
#
# `servers/api-server` owns ~197 integration test binaries (each statically
# links the whole server and provisions its own `#[sqlx::test]` database), so
# it dominates the wall time. This script deterministically shards the suite
# across N parallel CI runners so no single runner approaches the timeout:
#
#   * api-server integration test binaries (top-level `tests/*.rs`, one cargo
#     `--test` target each) are hash-partitioned by sorted position modulo the
#     shard count — new test files distribute automatically, no list to
#     maintain.
#   * Shard 0 additionally runs everything ELSE exactly once: all other crates'
#     lib/doc/integration tests (`--workspace --exclude api-server`, i.e.
#     reality-server, deploy-server, accounting-server, and every library
#     crate) plus api-server's own `--lib`/`--bins` unit tests.
#
# Usage:
#   ci-test-shard.sh <shard_index> <shard_count>
#     shard_index : 0-based index of THIS runner (0 <= index < count)
#     shard_count : total number of parallel shards
#
# Env (provided by the CI job): DATABASE_URL, RUST_ENV, plus the usual
# CARGO_PROFILE_*_DEBUG / CARGO_BUILD_JOBS memory/disk mitigations.
#
# Exit codes:
#   0  every cargo invocation on this shard passed
#   1  at least one cargo invocation failed (or timed out)
#   64 usage error
set -uo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <shard_index> <shard_count>" >&2
  exit 64
fi
idx="$1"
total="$2"
if ! [[ "$idx" =~ ^[0-9]+$ && "$total" =~ ^[0-9]+$ ]] || [ "$total" -lt 1 ] || [ "$idx" -ge "$total" ]; then
  echo "error: need 0 <= shard_index($idx) < shard_count($total), count >= 1" >&2
  exit 64
fi

# Run from the backend/ crate root regardless of caller CWD.
cd "$(dirname "$0")/.." || exit 64

# Per-invocation wall-clock guard. Each shard is a fraction of the old
# monolith, so 120m SIGTERM + 1m-later SIGKILL leaves comfortable margin under
# the job's timeout-minutes while still guaranteeing an unignorable stop if a
# test genuinely hangs (a hung cargo test process ignores SIGTERM; SIGKILL
# cannot be ignored — see the backend.yml history for BIT-334).
run() {
  echo "+ timeout --kill-after=1m 120m cargo test $*"
  timeout --kill-after=1m 120m cargo test "$@" --no-fail-fast -- --test-threads=4
}

# Enumerate api-server integration test binaries. Top-level `tests/*.rs` files
# are each a distinct cargo `--test` target named by basename; the `tests/common/`
# helper subdirectory is a module, not a target, and is excluded by -maxdepth 1.
mapfile -t api_tests < <(find servers/api-server/tests -maxdepth 1 -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort)

# Partition: target at sorted position P runs on shard (P % total).
sel=()
pos=0
for t in "${api_tests[@]}"; do
  if [ "$(( pos % total ))" -eq "$idx" ]; then
    sel+=(--test "$t")
  fi
  pos=$(( pos + 1 ))
done

echo "shard $idx/$total: ${#api_tests[@]} api-server integration targets total, $(( ${#sel[@]} / 2 )) on this shard"

rc=0
if [ "$idx" -eq 0 ]; then
  # Everything that is NOT an api-server integration test, run exactly once.
  run --workspace --exclude api-server || rc=1
  # api-server unit tests (lib + bins) + this shard's slice of integration tests.
  run -p api-server --lib --bins "${sel[@]}" || rc=1
else
  if [ "${#sel[@]}" -eq 0 ]; then
    echo "shard $idx: no api-server integration targets assigned — nothing to do"
    exit 0
  fi
  run -p api-server "${sel[@]}" || rc=1
fi

exit "$rc"
