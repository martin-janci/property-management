# Git, Epic/Story & Release Workflow

Full procedural reference for branching, epic/story delivery, releases, hotfixes, and
versioning. The always-loaded `CLAUDE.md` keeps only the branch model, branch naming,
and commit-message conventions; everything below is on-demand detail — read this file
when you are actually cutting a release, running a hotfix, or starting a new epic.

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
