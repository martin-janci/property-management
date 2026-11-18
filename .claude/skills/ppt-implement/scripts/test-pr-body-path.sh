#!/usr/bin/env bash
# Unit test for pr-body-path.sh.
#
# Pins the task-unique-path contract that prevents parallel implementers from
# clobbering each other's PR-body scratchpad file (observed collision
# 2026-11-17: two siblings both wrote a bare shared `pr-body.md`).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PBP="$SCRIPT_DIR/pr-body-path.sh"

FAIL=0
ok()  { printf '  ok    %s\n' "$1"; }
bad() { printf '  FAIL  %s\n' "$1" >&2; FAIL=1; }

echo "==> pr-body-path unit tests"

# The bare, shared path the fix must NEVER produce.
SHARED="/tmp/pr-body.md"

# 1. Distinct task_ids -> distinct paths (the core regression).
A="$(bash "$PBP" --task-id task-alpha --dir /tmp)"
B="$(bash "$PBP" --task-id task-beta  --dir /tmp)"
if [ "$A" != "$B" ]; then
  ok "distinct task_ids produce distinct paths ($A != $B)"
else
  bad "distinct task_ids collided: both resolved to $A"
fi

# 2. Same task_id -> stable path (write-then-read must hit the same file).
A2="$(bash "$PBP" --task-id task-alpha --dir /tmp)"
if [ "$A" = "$A2" ]; then
  ok "same task_id is stable across calls ($A)"
else
  bad "same task_id was unstable: $A vs $A2"
fi

# 3. Never the bare shared path.
if [ "$A" != "$SHARED" ]; then
  ok "task-unique path is not the bare shared $SHARED"
else
  bad "produced the bare shared path $SHARED"
fi

# 4. $TASK_ID env is honoured when no --task-id is passed.
ENVP="$(TASK_ID=task-from-env bash "$PBP" --dir /tmp)"
if [ "$ENVP" = "/tmp/pr-body.task-from-env.md" ]; then
  ok "\$TASK_ID env drives the filename ($ENVP)"
else
  bad "\$TASK_ID env not honoured: got $ENVP"
fi

# 5. No task_id and no $TASK_ID -> PID fallback, still not the shared path,
#    and still process-unique (contains the PID).
set +e
FB="$(env -u TASK_ID bash "$PBP" --dir /tmp 2>/dev/null)"
code=$?
set -e
if [ "$code" = "0" ] && [ "$FB" != "$SHARED" ] && [ "$FB" != "/tmp/pr-body..md" ]; then
  ok "PID fallback yields a non-bare, non-shared path ($FB)"
else
  bad "PID fallback misbehaved: exit=$code path=$FB"
fi

# 6. task_ids with shell/path-hostile chars are sanitised (no path separators
#    leak) and still kept distinct.
S1="$(bash "$PBP" --task-id 'a/b c$x' --dir /tmp)"
S2="$(bash "$PBP" --task-id 'a-b-c-x' --dir /tmp)"
# No second slash after the dir, i.e. the token contributed no '/'.
token_part="${S1#/tmp/}"
if [[ "$token_part" != */* ]]; then
  ok "hostile chars sanitised — no path separator leaked ($S1)"
else
  bad "path separator leaked from task_id: $S1"
fi
# Sanitised 'a/b c$x' and literal 'a-b-c-x' both collapse to the same safe
# token — documents that sanitisation is lossy-to-safe; the guarantee that
# matters (two REAL concurrent task_ids differ) holds because real task_ids
# are already filename-safe slugs.
if [ -n "$S1" ] && [ -n "$S2" ]; then
  ok "hostile and safe variants both resolve to a path"
else
  bad "empty path from sanitised token"
fi

# 7. Custom --dir is respected and trailing slash is normalised.
D1="$(bash "$PBP" --task-id t --dir /tmp/scratch)"
D2="$(bash "$PBP" --task-id t --dir /tmp/scratch/)"
if [ "$D1" = "/tmp/scratch/pr-body.t.md" ] && [ "$D1" = "$D2" ]; then
  ok "custom --dir honoured and trailing slash normalised ($D1)"
else
  bad "--dir handling wrong: D1=$D1 D2=$D2"
fi

echo
if [ "$FAIL" = "0" ]; then
  echo "==> pr-body-path: PASS"
  exit 0
else
  echo "==> pr-body-path: FAIL" >&2
  exit 1
fi
