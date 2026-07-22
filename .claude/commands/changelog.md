# Generate Changelog

Generate a changelog from git commits.

## Instructions

### Step 1: Determine Range
```bash
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
```

If $ARGUMENTS provided, use as range:
- `/changelog v0.1.0..v0.2.0` - Between two tags
- `/changelog v0.1.0` - From tag to HEAD
- `/changelog 10` - Last 10 commits

### Step 2: Get Commits
```bash
if [[ -n "$LAST_TAG" ]]; then
  COMMITS=$(git log --oneline "$LAST_TAG"..HEAD)
else
  COMMITS=$(git log --oneline -50)
fi
```

### Step 3: Categorize Commits
Parse commit messages by conventional commit type:

**Features** (`feat`):
```bash
echo "$COMMITS" | grep -E "^[a-f0-9]+ feat"
```

**Bug Fixes** (`fix`):
```bash
echo "$COMMITS" | grep -E "^[a-f0-9]+ fix"
```

**Documentation** (`docs`):
```bash
echo "$COMMITS" | grep -E "^[a-f0-9]+ docs"
```

**Refactoring** (`refactor`):
```bash
echo "$COMMITS" | grep -E "^[a-f0-9]+ refactor"
```

**Tests** (`test`):
```bash
echo "$COMMITS" | grep -E "^[a-f0-9]+ test"
```

**Chores** (`chore`):
```bash
echo "$COMMITS" | grep -E "^[a-f0-9]+ chore"
```

**Other** (remaining):
```bash
echo "$COMMITS" | grep -vE "^[a-f0-9]+ (feat|fix|docs|refactor|test|chore)"
```

### Step 4: Format Output
Generate markdown:

```markdown
# Changelog

## [Unreleased] - YYYY-MM-DD

### Features
- {feat commits with scope and description}

### Bug Fixes
- {fix commits}

### Documentation
- {docs commits}

### Other Changes
- {remaining commits}

### Contributors
{unique commit authors}
```

### Step 5: Output Options
Based on context:
- Display in terminal (default)
- If `/changelog > CHANGELOG.md` requested, append to file

### Step 6: Announce
```bash
.claude/hooks/play-tts.sh "Changelog generated with X commits"
```

## Usage
- `/changelog` - Generate from last tag to HEAD
- `/changelog v0.1.0` - Generate from specific tag
- `/changelog 20` - Last 20 commits only
