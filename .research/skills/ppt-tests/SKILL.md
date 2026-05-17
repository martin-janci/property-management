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
quote `just check` and `just test` outputs in the PR body — but during the
loop you usually want a narrower command. This skill is that map.

## When to invoke

You touched code and want to know "what's the fastest signal here?".

## What it gives you

- Stack → command lookup
- Filter syntax per runner
- IG3 stash/pop dance (failing test on `main` proof)

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

### Frontend (Nuxt / pnpm workspace)

```bash
just test-frontend                           # all apps + packages
cd frontend && pnpm -F ppt-web test          # single app
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

## IG3 — failing-on-main test (stash/pop dance)

For `bug`, `revert`, `risky-churn`, `test-gap` vectors:

```bash
# 1. write the test, run it, confirm it PASSES with the fix applied
cargo test -p <crate> -- <test_name>

# 2. stash the FIX (not the test) and re-run — confirm FAIL
git stash push -m "ig3-fix" -- <fix-paths>
cargo test -p <crate> -- <test_name>   # expected: FAIL

# 3. restore and re-confirm pass
git stash pop
cargo test -p <crate> -- <test_name>   # expected: PASS
```

Quote both runs in the PR body under `## IG3 — failing test on main`.

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
just check && just test    # quote tail of each in PR body (IG7)
```

## Cross-references

- [`.research/implementer-prompt.md`](../../implementer-prompt.md)
  § *Verification before opening the PR* and IG3, IG7
- [`ppt-rust-backend`](../ppt-rust-backend/SKILL.md), [`ppt-nuxt-frontend`](../ppt-nuxt-frontend/SKILL.md),
  [`ppt-mobile-native`](../ppt-mobile-native/SKILL.md) — per-stack detail
