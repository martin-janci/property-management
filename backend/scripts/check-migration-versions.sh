#!/bin/bash
# Migration Version Collision Checker
# Detects SQLx migration files that share the same numeric version prefix within
# a single migrations directory.
#
# Usage:
#   ./scripts/check-migration-versions.sh              # auto-discover backend migrations dirs
#   ./scripts/check-migration-versions.sh DIR [DIR...] # scan the given dirs (used by the test)
#
# With no arguments the script auto-discovers every `migrations/` directory
# under the backend tree (so a future server with its own migrations dir is
# guarded automatically, instead of silently un-checked until someone remembers
# to append it). Passing explicit directories overrides discovery — the
# regression test (`check-migration-versions.test.sh`) uses this to point the
# checker at temp fixtures.
#
# Exit codes:
#   0  Every migrations directory has unique numeric version prefixes.
#   1  Two or more migrations in the same directory share a version prefix.
#
# Why this exists:
#   SQLx records applied migrations in `_sqlx_migrations` keyed by the numeric
#   version parsed from each filename's leading digits. Two files with the same
#   prefix in one directory (e.g. `00192_a.sql` and `00192_b.sql`) collide on
#   `_sqlx_migrations_pkey`, so `sqlx migrate run` aborts with a duplicate-key
#   error and `#[sqlx::test]` setups fail. This recurred when two PRs merged
#   independently, each grabbing version 192 (#1688 idempotency + #1750
#   guest-id). This gate makes that collision impossible to merge.
#
#   Uniqueness is enforced PER DIRECTORY: each SQLx migrator instance numbers
#   its own set, so the same prefix in two different directories is fine.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Migrations directories to scan (each enforced independently).
#
# Explicit args override discovery (the test passes temp fixture dirs).
# Otherwise auto-discover every `migrations/` dir under the backend tree so the
# guard stays in lock-step with the actual set of `sqlx::migrate!` loaders and
# can't silently skip a newly added server's migrations. `target/` build output
# is excluded.
if [[ "$#" -gt 0 ]]; then
    MIGRATION_DIRS=("$@")
else
    mapfile -t MIGRATION_DIRS < <(
        find "$BACKEND_DIR" -type d -name migrations -not -path '*/target/*' | sort
    )
fi

had_violation=false

for dir in "${MIGRATION_DIRS[@]}"; do
    if [[ ! -d "$dir" ]]; then
        # Directory may not exist in every checkout layout; skip silently.
        continue
    fi

    rel="${dir#"$BACKEND_DIR/"}"

    # Collect leading numeric prefixes of every .sql migration in this dir.
    prefixes="$(
        for f in "$dir"/*.sql; do
            [[ -e "$f" ]] || continue
            base="$(basename "$f")"
            # Strip everything from the first non-digit onward.
            num="${base%%[!0-9]*}"
            [[ -n "$num" ]] || continue
            printf '%s\n' "$num"
        done
    )"

    [[ -n "$prefixes" ]] || continue

    # Normalize away leading zeros so 00192 and 192 are treated as the same
    # version (SQLx parses the numeric value, not the zero-padded string).
    dupes="$(printf '%s\n' "$prefixes" | sed 's/^0*//; s/^$/0/' | sort -n | uniq -d || true)"

    if [[ -n "$dupes" ]]; then
        had_violation=true
        echo -e "${RED}✗ Duplicate migration version(s) in ${rel}:${NC}"
        while IFS= read -r ver; do
            [[ -n "$ver" ]] || continue
            echo -e "  ${RED}version ${ver}:${NC}"
            for f in "$dir"/*.sql; do
                [[ -e "$f" ]] || continue
                base="$(basename "$f")"
                num="${base%%[!0-9]*}"
                norm="$(printf '%s' "$num" | sed 's/^0*//; s/^$/0/')"
                if [[ "$norm" == "$ver" ]]; then
                    echo "    - $base"
                fi
            done
        done <<< "$dupes"
    fi
done

if [[ "$had_violation" == true ]]; then
    echo -e "${RED}Migration version collision detected.${NC}" >&2
    echo "Renumber the later-merged migration to the next free version prefix" >&2
    echo "within that directory so each migration has a unique numeric version." >&2
    exit 1
fi

echo -e "${GREEN}✓ All migration directories have unique version prefixes.${NC}"
exit 0
