# Property Management System (PPT)

## Namespace

**Package namespace:** `three.two.bit.ppt`

| Platform | Package/Bundle ID |
|----------|-------------------|
| Android (Reality Portal) | `three.two.bit.ppt.reality` |
| iOS (Reality Portal) | `three.two.bit.ppt.reality` |
| Android (Property Mgmt) | `three.two.bit.ppt.management` |
| iOS (Property Mgmt) | `three.two.bit.ppt.management` |

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        PROPERTY MANAGEMENT                          │
├─────────────────────────────────────────────────────────────────────┤
│  ppt-web (React SPA)     │  mobile (React Native)                  │
│  - Manager dashboard     │  - Android: three.two.bit.ppt.management│
│  - Building management   │  - iOS: three.two.bit.ppt.management    │
│  - Faults, Voting, etc   │                                         │
├─────────────────────────────────────────────────────────────────────┤
│                         api-server (Rust)                           │
│  Port 8080 │ Management API │ OAuth Provider                        │
└─────────────────────────────────────────────────────────────────────┘
                                  │
                           Shared Database
                                  │
┌─────────────────────────────────────────────────────────────────────┐
│                         reality-server (Rust)                       │
│  Port 8081 │ Public listings │ SSO Consumer                        │
├─────────────────────────────────────────────────────────────────────┤
│  reality-web (Next.js SSR)   │  mobile-native (KMP)                │
│  - Public listings           │  - Android: three.two.bit.ppt.reality│
│  - Search, filters           │  - iOS: three.two.bit.ppt.reality   │
│  - i18n (sk, cs, de, en)     │                                     │
├─────────────────────────────────────────────────────────────────────┤
│                          REALITY PORTAL                             │
└─────────────────────────────────────────────────────────────────────┘
```

## Platform Matrix

| Platform | App | Technology | Backend |
|----------|-----|------------|---------|
| Web | Property Management | React SPA (Vite) | api-server |
| Web | Reality Portal | Next.js (SSR + ISR) | reality-server |
| Mobile | Property Management | React Native | api-server |
| Mobile | Reality Portal | Kotlin Multiplatform | reality-server |

## Tech Stack

### backend/
| Component | Technology |
|-----------|------------|
| Language | Rust (edition 2021, `rust-version = 1.75`) |
| Async runtime | Tokio 1.35 |
| Web framework | Axum 0.8 + axum-extra 0.12, Tower 0.5 / tower-http 0.6 |
| Database | PostgreSQL 16+ (RLS), SQLx 0.9 |
| Cache | Redis 1.2 (sessions, pub/sub) |
| Auth | `jsonwebtoken` 9.2 + `argon2` 0.5 (token TTLs in `docs/api/README.md`) |
| API | OpenAPI via `utoipa` 5 (+ `utoipa-swagger-ui` 9), WebSocket |
| Storage | S3-compatible |

> Source of truth for crate versions: `backend/Cargo.toml` (`[workspace.dependencies]`). Update both there and in this table together.

### frontend/

Workspace toolchain: Node ≥ 20, pnpm 8.14 (`packageManager` pinned), TypeScript 5.3, Biome 2.4 (lint + format).

| App | Technology |
|-----|------------|
| ppt-web | React 19.2, Vite 7, TanStack Query 5, react-router-dom 6, react-i18next 16, Vitest 4 |
| admin-web | React 19.2, Vite 7, TanStack Query 5, react-router-dom 6, react-i18next 16 |
| reality-web | Next.js 16 (SSR/SSG), React 19.2, next-intl 4, TanStack Query 5 |
| mobile | React Native 0.85, Expo 55, React 19.2, react-i18next 16, jest-expo 56 |
| shared | TypeScript 5.3, `@hey-api/openapi-ts` (generates `@ppt/api-client` + `@ppt/reality-api-client`) |

### mobile-native/
| Component | Technology |
|-----------|------------|
| Language | Kotlin 2.3.21 (KMP), AGP 8.7.3, KSP 2.1.0 |
| Android SDK | compileSdk 34, targetSdk 34, minSdk 24 |
| UI | Jetpack Compose (BOM 2024.12) — Android; SwiftUI — iOS |
| Networking | Ktor 3.0 (client-core, content-negotiation, kotlinx-json, logging; android + darwin engines) |
| Kotlin libs | kotlinx-coroutines 1.11, kotlinx-serialization 1.11, kotlinx-datetime 0.8 |
| Imaging | Coil 2.5 (Compose) |
| SDK gen | `openapi-generator` (Kotlin client) |

> Source of truth: `mobile-native/gradle/libs.versions.toml`.

## Project Structure

```
property-management/
├── backend/              # Rust: api-server, reality-server
├── frontend/             # TypeScript: ppt-web, reality-web, mobile
└── mobile-native/        # Kotlin: Reality Portal (Android/iOS)
```

## Git Conventions

### Branch Model

- **`dev`** — default integration branch. All feature/bugfix PRs target `dev`.
- **`main`** — release branch. `dev` is merged into `main` only during releases (version cut + tag).
- **`hotfix/*`** — branched from `main`, merged back into both `main` and `dev`.

### Branch Naming

Lowercase, hyphen-separated, no zero-padding:

```
feature/epic-{n}-{short-description}
bugfix/story-{n}.{m}-{short-description}
hotfix/{short-description}
```

**Examples:**
- `feature/epic-1-user-authentication`
- `bugfix/story-4.3-fault-triage-permissions`
- `hotfix/refresh-token-rotation`

### Commit Messages

Conventional Commits:

```
{type}({scope}): {description}
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

**Scope** — pick the most specific applicable, in this priority order:

1. **Use case** when the commit implements a specific UC: `feat(UC-14): …`
2. **Epic** for cross-cutting epic work: `feat(epic-1): …`
3. **Package / app** for infra, tooling, or single-package changes: `fix(api-server): …`, `chore(reality-web): …`, `docs(mobile-native): …`

**Examples:**
- `feat(UC-14): implement user registration`
- `feat(epic-1): story 1.2 - email/password login`
- `fix(api-server): correct tenant context extraction`
- `docs(reality-web): add i18n documentation`

## Epic & Story Development Workflow

**IMPORTANT: Follow this workflow when implementing epics and stories.**

### Before Starting an Epic

```bash
# Create feature branch from dev (the default integration branch)
git checkout dev
git pull origin dev
git checkout -b feature/epic-{N}-{description}
```

**Example:** `git checkout -b feature/epic-1-user-authentication`

> Do **not** branch from `main`. `main` only moves forward during releases.

### After Completing Each Story

```bash
# Stage and commit with story reference
git add .
git commit -m "feat(epic-{N}): story {N}.{M} - {description}"
```

**Examples:**
- `feat(epic-1): story 1.1 - user registration with email verification`
- `feat(epic-1): story 1.2 - email/password login`
- `feat(epic-4): story 4.3 - fault triage by manager`

### After All Stories in Epic Complete

1. **Run BMAD Code Review Workflow:**
   ```bash
   # Invoke the code-review workflow
   /bmad:bmm:workflows:code-review
   ```

2. **Address Review Findings** - Fix any issues identified

3. **Create Pull Request against `dev`:**
   ```bash
   git push -u origin feature/epic-{N}-{description}
   gh pr create --base dev --title "Epic {N}: {Title}" --body "..."
   ```

### Release Workflow (`dev` → `main`)

Releases are the only path from `dev` to `main`.

```bash
# 1. Make sure dev is green and up to date
git checkout dev
git pull origin dev

# 2. Bump version (writes VERSION + syncs package.json / gradle.properties)
./scripts/bump-version.sh minor   # or major / patch

# 3. Open the release PR
git push origin dev
gh pr create --base main --head dev \
  --title "Release v$(cat VERSION)" \
  --body "Release notes…"

# 4. After merge to main, tag the release
git checkout main && git pull origin main
git tag -a "v$(cat VERSION)" -m "Release v$(cat VERSION)"
git push origin "v$(cat VERSION)"
```

### Hotfix Workflow (`main` → `hotfix/*` → `main` + `dev`)

```bash
# 1. Branch from main (the released code)
git checkout main
git pull origin main
git checkout -b hotfix/{short-description}

# 2. Fix + commit + push
git commit -am "fix({scope}): {description}"
git push -u origin hotfix/{short-description}

# 3. PR into main (release the hotfix)
gh pr create --base main --title "Hotfix: {description}" --body "..."

# 4. After merge, fast-forward the fix into dev too
git checkout dev
git pull origin dev
git merge --no-ff origin/main
git push origin dev
```

### Workflow Summary

```
┌─────────────────────────────────────────────────────────────┐
│  1. Branch from dev: feature/epic-{N}-{description}         │
├─────────────────────────────────────────────────────────────┤
│  2. Implement Story {N}.1 → Commit                          │
│  3. Implement Story {N}.2 → Commit                          │
│  4. Implement Story {N}.{M} → Commit                        │
│     ... repeat for all stories ...                          │
├─────────────────────────────────────────────────────────────┤
│  5. Run /bmad:bmm:workflows:code-review                     │
│  6. Fix issues → Commit fixes                               │
├─────────────────────────────────────────────────────────────┤
│  7. Push branch and open PR against `dev`                   │
│  8. Merge into `dev` after approval                         │
├─────────────────────────────────────────────────────────────┤
│  9. At release time: PR `dev` → `main`, tag, publish        │
└─────────────────────────────────────────────────────────────┘
```

## Versioning

Single source of truth: `VERSION` file (semantic versioning X.Y.Z)

```bash
# Patch auto-bumps via CI after merge to `main` (.github/workflows/version-bump.yml).
# Manual bumps (run before opening the release PR):
./scripts/bump-version.sh patch   # 0.1.0 -> 0.1.1
./scripts/bump-version.sh minor   # 0.1.x -> 0.2.0
./scripts/bump-version.sh major   # 0.x.y -> 1.0.0

# Sync VERSION into all sub-projects without bumping (re-run after manual VERSION edit):
./scripts/update-version.sh

# Install git hooks (one-time setup)
./scripts/install-hooks.sh
```

`update-version.sh` propagates the `VERSION` file into:

- `backend/Cargo.toml` (`workspace.package.version`)
- `frontend/package.json` + every `frontend/apps/*/package.json` and `frontend/packages/*/package.json`
- `mobile-native/gradle.properties` (`versionName` + `versionCode`)
- `docs/api/typespec/main.tsp` (API service version)

`bump-version.sh` calls `update-version.sh` automatically after writing the new `VERSION`.

## Quick Start

### One-shot setup (first clone)

```bash
./scripts/setup.sh        # verifies tools, installs git hooks + deps, prepares .env files
./scripts/health-check.sh # confirm the environment is sane
```

The repo also ships a root `justfile` for common cross-stack tasks — run `just` with no args to list recipes.

### Run dev servers

```bash
# Backend (workspace under backend/)
cd backend && cargo run -p api-server         # port 8080 (Property Management)
cd backend && cargo run -p reality-server     # port 8081 (Reality Portal public API)

# Frontend (pnpm workspace under frontend/)
cd frontend && pnpm install
cd frontend && pnpm dev:ppt                   # Property Management SPA
cd frontend && pnpm dev:reality               # Reality Portal (Next.js)
cd frontend && pnpm dev:mobile                # React Native (Expo)

# Mobile Native (Reality — Kotlin Multiplatform)
cd mobile-native && ./gradlew build
```

For a full local stack (Postgres + Redis + MinIO + servers + web), use the `ppt-dev-stack` skill (`stack up pm-local`) instead of starting each piece by hand.

### Tests

```bash
# Backend
cd backend && cargo test                      # full workspace
cd backend && cargo test -p api-core          # single crate

# Frontend
cd frontend && pnpm test                      # all packages
cd frontend && pnpm typecheck                 # TS check only

# Mobile Native
cd mobile-native && ./gradlew test
```

### Style & Lint

```bash
# Backend
cd backend && cargo fmt --all
cd backend && cargo clippy --workspace --all-targets -- -D warnings

# Frontend (Biome)
cd frontend && pnpm check                     # lint + format check
cd frontend && pnpm check:fix                 # autofix
cd frontend && pnpm format                    # format only
cd frontend && pnpm lint                      # lint only
```

### Environment

Local `.env` files are created by `setup.sh`. Never commit them — they hold local DB creds, S3 keys, and JWT secrets. Templates live at `backend/.env.example` and `frontend/.env.example` where applicable.

## CI Gates & PR Review

Every PR against `dev` must pass these GitHub Actions before merge:

| Workflow | Triggers on PR when… | Verifies |
|----------|----------------------|----------|
| `backend.yml` | Backend code changes | `cargo fmt`, `clippy`, `cargo test` |
| `frontend.yml` | Frontend code changes | `biome check`, `typecheck`, `pnpm test`, build |
| `mobile-native.yml` | `mobile-native/` changes | Gradle build + unit tests |
| `api-validation.yml` | TypeSpec / OpenAPI changes | Spec compiles, generated clients are up to date |
| `screen-map.yml` | `docs/screens/**` changes | `/screens validate` |
| `docker-build.yml` / `docker-frontend.yml` | Dockerfile or release tag | Images build |
| `release.yml` | Push to `main` | Cuts the release, waits on `backend.yml` + `frontend.yml` to be green |
| `version-bump.yml` | Push to `main` | Auto-bumps patch version after merge |
| `approve-pr.yml` / `auto-approve.yml` | PR events | Dependabot / trusted-author auto-approval gate (≥2 min after last push) |

Local pre-flight before pushing: `cargo fmt && cargo clippy && cargo test` in `backend/`, `pnpm check && pnpm typecheck && pnpm test` in `frontend/`.

## Worktrees

The repo is developed with multiple `git worktree`s checked out under `.claude/worktrees/<name>/`. Use the `ppt-deploy` skill to spin up matching `*.dev.ppt.rlt.sk` / `*.dev.rlt.sk` environments per worktree, and the `ppt-dev-stack` skill (or root `justfile`) to bring up a local stack inside one. Don't commit the `.claude/worktrees/` paths themselves — only the code changes inside them.

## Documentation Index

| File | Description |
|------|-------------|
| `docs/CLAUDE.md` | Use cases, PRD/Epic/Story conventions |
| `docs/spec1.0.md` | Original system specification |
| `docs/use-cases.md` | 508 use cases catalog |
| `docs/project-structure.md` | Full directory structure |
| `docs/api/README.md` | API specification index |

## Screen-Map Self-Management Protocol

The repo includes a screen-map system at `docs/screens/<product>/<id>.md`. Agents working on UI/route code should integrate with it. See [the design spec](docs/superpowers/specs/2026-05-07-screen-map-system-design.md) Section 9.

### Rules for agents

**A. On screen-related code changes (before committing):**

1. `/screens edit <id>` to load context for the screen you're modifying.
2. Update frontmatter (`buildStatus`, `apiStatus`) if outcomes changed.
3. Add Agent Log entry: `<date> — agent: <terse summary>`.
4. Update `Notes > Specific (recent)` if the change is relevant for future agents.
5. Run `/screens validate`.

**B. On new route / mobile screen added:**

1. `/screens update` detects drift.
2. Create or attach the new route to a screen-map via `/screens init --add` or by editing an existing one.

**C. On redesign milestone (Figma frame ready):**

1. `/screens review --filter=redesignStatus:not-started` to walk the candidates.
2. After implementation: `/screens edit <id>` to flip `redesignStatus: in-progress → applied`.

**D. Periodically (manual cadence):**

- `/screens query "buildStatus:shipped,redesignStatus:not-started"` — find redesign roadmap candidates.
- `/screens render --scope=ppt` — refresh status dashboard.
