#!/bin/bash
# Orphan integration-test checker.
#
# Fails when a crate ships `.rs` test code in a SUBDIRECTORY of its `tests/`
# directory that cargo's top-level auto-discovery will never compile — i.e.
# dead test coverage that looks live but never runs.
#
# Usage:
#   ./scripts/check-no-orphan-tests.sh              # auto-discover backend crate tests/ dirs
#   ./scripts/check-no-orphan-tests.sh DIR [DIR...] # scan the given crate roots (used by the test)
#
# Exit codes:
#   0  Every `.rs` file under a crate's tests/ is either a top-level test
#      binary or lives in a subdirectory that a top-level test file actually
#      `mod`-includes (a reachable helper module such as `tests/common/`).
#   1  At least one `.rs` file lives in a tests/ subdirectory that no top-level
#      test file declares as a module — it is orphaned and never compiled.
#
# Why this exists (GH #2158):
#   Cargo compiles each `tests/*.rs` file as its own integration-test binary.
#   Files nested in a SUBDIRECTORY (e.g. `tests/integration/foo_tests.rs`) are
#   compiled ONLY if some top-level `tests/*.rs` declares `mod integration;` so
#   the subdir is pulled into a binary's module tree. `tests/common/` works
#   precisely because every top-level test file does `mod common;`.
#
#   The `tests/integration/` subtree had NO such declaration anywhere, so the
#   entire subtree (auth/health/sync/MFA e2e/recovery-code flows — 66 test
#   functions) silently never built and never ran. Deleting or promoting the
#   subtree fixes the immediate leak; THIS guard makes the class of bug
#   un-mergeable so it can't quietly regrow.
#
# How the reachability test works:
#   For each crate `tests/` directory, every immediate subdirectory that
#   contains `.rs` files must be named in a `mod <subdir>;` declaration in at
#   least one sibling top-level `tests/*.rs` file. This mirrors exactly what
#   cargo needs to link the subdir into a test binary — no hardcoded helper-dir
#   allow-list, so a future `tests/helpers/` that IS `mod helpers;`-included
#   passes automatically, while an un-included one is flagged.
#
#   Only Rust crates are scanned: a `tests/` dir counts only when its parent
#   holds a `Cargo.toml`. That deliberately excludes shell-lint fixture trees
#   such as `backend/scripts/lints/tests/**` whose `.rs` files are text inputs
#   for a grep-based lint, not cargo test targets.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(dirname "$SCRIPT_DIR")"

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Crate roots to scan. Explicit args override discovery (the self-test passes
# temp fixture crate roots). Otherwise auto-discover every crate (a directory
# with a Cargo.toml) that has a `tests/` subdir, anywhere under the backend
# tree, excluding build output.
if [[ "$#" -gt 0 ]]; then
    CRATE_ROOTS=("$@")
else
    mapfile -t CRATE_ROOTS < <(
        find "$BACKEND_DIR" -type d -name tests -not -path '*/target/*' \
            -printf '%h\n' | sort -u
    )
fi

had_violation=false

for crate in "${CRATE_ROOTS[@]}"; do
    tests_dir="$crate/tests"
    [[ -d "$tests_dir" ]] || continue

    # Only Rust crates: the parent of tests/ must carry a Cargo.toml. This
    # skips shell-fixture trees (e.g. backend/scripts/lints/tests) whose .rs
    # files are lint inputs, not cargo targets.
    [[ -f "$crate/Cargo.toml" ]] || continue

    rel_crate="${crate#"$BACKEND_DIR/"}"

    # Collect the set of module names declared by top-level test files, i.e.
    # `mod <name>;` (optionally `pub`) in any `tests/*.rs`. These are the
    # subdirectories cargo will actually link into a test binary.
    declared_mods="$(
        for f in "$tests_dir"/*.rs; do
            [[ -e "$f" ]] || continue
            grep -hoE '^[[:space:]]*(pub[[:space:]]+)?mod[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*;' "$f" 2>/dev/null \
                | sed -E 's/^[[:space:]]*(pub[[:space:]]+)?mod[[:space:]]+([A-Za-z_][A-Za-z0-9_]*)[[:space:]]*;/\2/'
        done | sort -u
    )"

    # Inspect each immediate subdirectory of tests/ that holds .rs files.
    while IFS= read -r subdir; do
        [[ -n "$subdir" ]] || continue
        name="$(basename "$subdir")"

        # Does any top-level test file declare `mod <name>;`? If so the subdir
        # is reachable and its contents compile.
        if grep -qxF "$name" <<< "$declared_mods"; then
            continue
        fi

        # Consolidated-harness pattern (2026-07-23, build-experience roadmap
        # item 8): suite harness binaries include individual subdir files via
        #   #[path = "<subdir>/<file>.rs"] mod <file>;
        # In that case reachability is checked PER FILE — every .rs directly
        # under the subdir must be #[path]-referenced by some top-level
        # tests/*.rs, which is stricter than the whole-dir `mod` rule (a
        # single dropped file is flagged even when its siblings compile).
        path_refs="$(
            for f in "$tests_dir"/*.rs; do
                [[ -e "$f" ]] || continue
                grep -hoE "#\[path[[:space:]]*=[[:space:]]*\"${name}/[^\"]+\"\]" "$f" 2>/dev/null \
                    | sed -E "s|.*\"${name}/([^\"]+)\".*|\1|"
            done | sort -u
        )"
        if [[ -n "$path_refs" ]]; then
            unreferenced="$(
                while IFS= read -r rs; do
                    [[ -n "$rs" ]] || continue
                    rel="${rs#"$subdir/"}"
                    grep -qxF "$rel" <<< "$path_refs" || printf '%s\n' "$rs"
                done < <(find "$subdir" -maxdepth 1 -type f -name '*.rs' | sort)
            )"
            if [[ -z "$unreferenced" ]]; then
                continue
            fi
            had_violation=true
            echo -e "${RED}✗ Orphaned files in ${rel_crate}/tests/${name}/ (harness pattern):${NC}"
            echo -e "  ${RED}no top-level ${rel_crate}/tests/*.rs carries \`#[path = \"${name}/<file>\"]\` for:${NC}"
            while IFS= read -r rs; do
                [[ -n "$rs" ]] || continue
                echo "    - ${rs#"$BACKEND_DIR/"}"
            done <<< "$unreferenced"
            continue
        fi

        # Orphan: no top-level file pulls this subdir into a binary.
        had_violation=true
        echo -e "${RED}✗ Orphaned test subtree in ${rel_crate}/tests/${name}/:${NC}"
        echo -e "  ${RED}no top-level ${rel_crate}/tests/*.rs declares \`mod ${name};\`, so cargo never compiles:${NC}"
        while IFS= read -r rs; do
            [[ -n "$rs" ]] || continue
            echo "    - ${rs#"$BACKEND_DIR/"}"
        done < <(find "$subdir" -type f -name '*.rs' | sort)
    done < <(
        find "$tests_dir" -mindepth 1 -maxdepth 1 -type d | sort | while IFS= read -r d; do
            # Only report subdirs that actually contain .rs files.
            if find "$d" -type f -name '*.rs' -print -quit | grep -q .; then
                printf '%s\n' "$d"
            fi
        done
    )
done

if [[ "$had_violation" == true ]]; then
    echo >&2
    echo -e "${RED}Orphaned (never-compiled) test code detected.${NC}" >&2
    echo "Either PROMOTE each file to a top-level tests/<name>.rs binary (the" >&2
    echo "standard 'mod common;' pattern), or DELETE it if a live top-level" >&2
    echo "equivalent already supersedes it. A test that never compiles proves" >&2
    echo "nothing. See GH #2158." >&2
    exit 1
fi

echo -e "${GREEN}✓ No orphaned test subtrees — every tests/ subdirectory is mod-included.${NC}"
exit 0
