# Create a Release

Automate version bumping, changelog generation, and release creation.

## Instructions

### Step 1: Validate State
```bash
git status --short
git branch --show-current
```

Verify:
- On main branch (or create release branch)
- No uncommitted changes
- All CI checks passing: `gh pr checks` or `gh run list --limit 1`

### Step 2: Determine Version Bump
Parse $ARGUMENTS:
- `patch` (default) - Bug fixes: 0.1.0 -> 0.1.1
- `minor` - New features: 0.1.0 -> 0.2.0
- `major` - Breaking changes: 0.1.0 -> 1.0.0

### Step 3: Get Current Version
```bash
cat VERSION
```

### Step 4: Generate Changelog
Get commits since last tag:
```bash
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
if [[ -n "$LAST_TAG" ]]; then
  git log --oneline "$LAST_TAG"..HEAD
else
  git log --oneline -20
fi
```

Group by type:
- **Features**: `feat(...)` commits
- **Fixes**: `fix(...)` commits
- **Other**: remaining commits

### Step 5: Bump Version
```bash
./scripts/bump-version.sh $TYPE
```

This updates:
- `VERSION` file
- All `package.json` files
- `mobile-native/gradle.properties`

### Step 6: Create Release Commit
```bash
NEW_VERSION=$(cat VERSION)
git add -A
git commit -m "chore(release): v$NEW_VERSION"
```

### Step 7: Create Tag
```bash
git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"
```

### Step 8: Push & Create GitHub Release
```bash
git push origin main --tags

gh release create "v$NEW_VERSION" \
  --title "v$NEW_VERSION" \
  --notes "$(cat <<EOF
## What's Changed

### Features
{list feat commits}

### Bug Fixes
{list fix commits}

### Other Changes
{list other commits}

**Full Changelog**: https://github.com/OWNER/REPO/compare/$LAST_TAG...v$NEW_VERSION
EOF
)"
```

### Step 9: Announce
```bash
.claude/hooks/play-tts.sh "Release v$NEW_VERSION created successfully"
```

## Usage
- `/release` - Create patch release
- `/release minor` - Create minor release
- `/release major` - Create major release
