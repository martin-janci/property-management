---
name: ppt-tests
description: Pick the smallest correct test command for the change at hand — backend, frontend, integration, or mobile.
when_to_use: You have a diff staged and need to verify it without re-running everything; or you need to satisfy IG3 (failing-on-main test) before a PR.
mode: both
capabilities: [C6]
tags: [workflow]
---

# PPT Tests

Map of change → test command. The implementer prompt requires that you
quote the `just verify` output (VERIFY-PLAN + VERIFY OK) in the PR body —
but during the loop you usually want a narrower command. This skill is
that map.

## When to invoke

You touched code and want to know "what's the fastest signal here?".

## What it gives you

- Stack → command lookup
- Filter syntax per runner
- IG3 two-commit TDD pattern (failing-on-main proof; no `git stash`)

## Inputs

- Knowledge of which area you touched (`backend/`, `frontend/`, `mobile-native/`)
- (optional) a test name or path glob

## Steps

### Backend (Rust)

```bash
just test-backend                            # full workspace
cd backend && cargo test -p <crate> -- <filter>   # narrow to one crate / filter
cd backend && cargo test --workspace -- <pattern>  # filter across workspace
just test-integration                        # ignored/integration tests (needs DB)
```

Workspace members: `common`, `api-core`, `admin-core`, `db`, `integrations`,
`tenant-ops`, `api-server`, `reality-server`, `deploy-server`.

### Frontend (pnpm workspace — Vite for `ppt-web`, Next.js for `reality-web`)

```bash
just test-frontend                           # all apps + packages
cd frontend && pnpm -F @ppt/web test         # single app (package name from its package.json, NOT dir name)
cd frontend && pnpm -F @ppt/api-client test  # single package
```

Apps under `frontend/apps/`: `ppt-web`, `reality-web`, `mobile` (RN/Expo).

### Integration

```bash
just test-integration   # cargo test --workspace --test '*' -- --ignored
```

Requires a populated database — bring `postgres` up via `ppt-dev-stack`
first; this repo has **no `just seed` recipe** so seed via the in-crate
helpers (`backend/crates/db/src/seed/`) or per-test factories.

### Mobile-native (Kotlin / Gradle)

```bash
cd mobile-native && ./gradlew test                              # all tests
cd mobile-native && ./gradlew :shared:test                      # one module
cd mobile-native && ./gradlew test --tests "*<filter>*"         # filter
```

ADB-driven UI checks are C5, local-only — see `ppt-mobile-native`.

## IG3 — failing-on-main test (two-commit pattern)

For vector `bug` or `test-gap`, **or** any plan sourced from a `revert-…` /
`risky-churn-…` signal (see `routine-prompt.md` Phase 1 signal table; the
vector for such plans is typically `bug`). **Do not use `git stash`** —
it only isolates uncommitted work, so once the fix is committed (required
to be in the PR) stash can't separate it from the test. See
[`implementer-prompt.md`](../../../.research/implementer-prompt.md#ig3—test-that-would-have-caught-the-bug-exists-and-fails-on-main)
for the canonical rule.

The pattern:

```bash
# 1. Commit the failing test on its own
git add <test-path> && git commit -m "test: add regression for <issue>"
TEST_SHA="$(git rev-parse HEAD)"

# 2. Run the test at this point — confirm it FAILS (proves the bug exists)
cargo test -p <crate> -- <test_name>   # expected: FAIL

# 3. Commit the fix
git add <fix-paths> && git commit -m "fix: <issue>"

# 4. Run the test at HEAD — confirm it PASSES
cargo test -p <crate> -- <test_name>   # expected: PASS
```

Quote both runs in the PR body under `## IG3 — failing test on main`,
along with the two commit SHAs so reviewers can `git checkout` either side.

## Deterministic verification

```bash
# 1. just is on PATH and the recipes resolve
just --list >/dev/null && echo OK
# expected: OK

# 2. the test recipes referenced above exist
just --list | grep -E 'test-backend|test-frontend|test-integration' | wc -l
# expected: 3

# 3. cargo is on PATH (backend)
cargo --version >/dev/null && echo OK
# expected: OK

# 4. pnpm is on PATH (frontend)
pnpm --version >/dev/null && echo OK
# expected: OK
```

## Smoke check (single command)

```bash
just --list | grep -qE '^\s+(test-backend|test-frontend|test-integration)\b'
```

## After-task verification

```bash
just verify    # impact-scoped gate — quote VERIFY-PLAN block + VERIFY OK <hash> in PR body (IG7)
```

Scope is automatic (see `scripts/verify-impact.sh` and
[`_verify-rules.md`](../_verify-rules.md)) — never hand-compose
full-workspace commands as a substitute.

## Cross-references

- [`.research/implementer-prompt.md`](../../../.research/implementer-prompt.md)
  § *Verification before opening the PR* and IG3, IG7
- [`ppt-rust-backend`](../ppt-rust-backend/SKILL.md), [`ppt-frontend`](../ppt-frontend/SKILL.md),
  [`ppt-mobile-native`](../ppt-mobile-native/SKILL.md) — per-stack detail
