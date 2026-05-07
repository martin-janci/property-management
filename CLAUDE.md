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
│  Port 8080 │ UC-01 to UC-32 │ OAuth Provider                       │
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
| Framework | Rust 1.75+, Axum 0.7 |
| Database | PostgreSQL 16+ (RLS), SQLx |
| Cache | Redis (sessions, pub/sub) |
| Auth | JWT (15m access, 7d refresh), Argon2id |
| API | OpenAPI (utoipa), WebSocket |
| Storage | S3-compatible |

### frontend/
| App | Technology |
|-----|------------|
| ppt-web | React 18, Vite 5, TanStack Query |
| reality-web | Next.js 14 (SSR/SSG), next-intl |
| mobile | React Native 0.73, Expo 50 |
| shared | TypeScript 5, @hey-api/openapi-ts (SDK gen) |

### mobile-native/
| Component | Technology |
|-----------|------------|
| Framework | Kotlin Multiplatform |
| UI | Compose (Android), SwiftUI (iOS) |
| SDK | openapi-generator (Kotlin) |

## Project Structure

```
property-management/
├── backend/              # Rust: api-server, reality-server
├── frontend/             # TypeScript: ppt-web, reality-web, mobile
└── mobile-native/        # Kotlin: Reality Portal (Android/iOS)
```

## Git Conventions

### Branch Naming

```
feature/{EPIC-NNN}-{description}
bugfix/{STORY-NNN}-{description}
hotfix/{issue-description}
```

### Commit Messages

```
{type}({scope}): {description}
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

**Examples:**
- `feat(UC-14): implement user registration`
- `fix(api-server): correct tenant context extraction`
- `docs(reality-portal): add i18n documentation`

## Epic & Story Development Workflow

**IMPORTANT: Follow this workflow when implementing epics and stories.**

### Before Starting an Epic

```bash
# Create feature branch from main
git checkout main
git pull origin main
git checkout -b feature/epic-{N}-{description}
```

**Example:** `git checkout -b feature/epic-1-user-authentication`

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

3. **Create Pull Request:**
   ```bash
   git push -u origin feature/epic-{N}-{description}
   gh pr create --title "Epic {N}: {Title}" --body "..."
   ```

### Workflow Summary

```
┌─────────────────────────────────────────────────────────────┐
│  1. Create feature branch: feature/epic-{N}-{description}  │
├─────────────────────────────────────────────────────────────┤
│  2. Implement Story {N}.1 → Commit                          │
│  3. Implement Story {N}.2 → Commit                          │
│  4. Implement Story {N}.{M} → Commit                        │
│     ... repeat for all stories ...                          │
├─────────────────────────────────────────────────────────────┤
│  5. Run /bmad:bmm:workflows:code-review                     │
│  6. Fix issues → Commit fixes                               │
├─────────────────────────────────────────────────────────────┤
│  7. Push branch and create PR                               │
│  8. Merge to main after approval                            │
└─────────────────────────────────────────────────────────────┘
```

## Versioning

Single source of truth: `VERSION` file (semantic versioning X.Y.Z)

```bash
# Patch auto-bumps via CI after merge to main (version-bump.yml workflow)
# Manual bumps:
./scripts/bump-version.sh minor   # 0.1.x -> 0.2.0
./scripts/bump-version.sh major   # 0.x.y -> 1.0.0

# Install hooks (one-time setup)
./scripts/install-hooks.sh
```

Version syncs to: all `package.json` files, `mobile-native/gradle.properties`

## Quick Start

```bash
# Backend
cd backend && cargo build

# Frontend
cd frontend && pnpm install && pnpm dev:ppt

# Reality Portal
cd frontend && pnpm dev:reality

# Mobile Native (Reality)
cd mobile-native && ./gradlew build
```

## Documentation Index

| File | Description |
|------|-------------|
| `docs/CLAUDE.md` | Use cases, PRD/Epic/Story conventions |
| `docs/spec1.0.md` | Original system specification |
| `docs/use-cases.md` | 508 use cases catalog |
| `docs/project-structure.md` | Full directory structure |
| `docs/api/README.md` | API specification index |
