---
name: ppt-dev-review
description: >
  Rotating expert code review for the PPT research routine. Each run reviews ONE scope segment
  (from 8 rotating segments covering the full codebase) using static analysis only — no compilation,
  no test execution. Findings are emitted as `code-review-finding` signals for the research backlog.
  Invoke from Phase 1.5 of the research routine. Segment selection is automatic: churn hotspots
  from Phase 1 get priority, otherwise picks the oldest-unreviewed segment.
  Use this skill whenever the research routine runs its daily Phase 1.5 code review slice.
when_to_use: Called by the research routine Phase 1.5. Also useful standalone when you want a quick
  domain-focused static review of one codebase segment without running the full 10-pass team.
mode: cloud-ok
---

# PPT Dev Review — Research Routine Skill

Single-segment rotating expert review. Designed for the daily cloud research routine:
- **No compilation** (cloud has no local Rust/Node toolchain)
- **Static analysis only** — file reading, grep, `gh` CLI
- **1–2 expert agents** matched to the segment's domain
- **Cap at 3 findings per run** — highest severity wins
- **Output: signals** not fixes — findings feed the backlog via Phase 2

For the full 10-pass fix-it workflow, use the local `ppt-dev-team` skill instead.

---

## Inputs (from the research routine)

The routine provides:
- `CHURN_FILES` — list of churn hotspot file paths from Phase 1 (may be empty)
- `REVIEW_CURSOR` — the `state.review_cursor` object from `state.json` (segment → last-reviewed-date or null)

If invoked standalone (not from the routine), pass these as context or leave empty to default to oldest-unreviewed.

---

## Step 1 — Select segment

### Segment map

| Segment ID | Root path | Domain |
|-----------|-----------|--------|
| `api-handlers` | `backend/api-server/src/handlers/` | Rust |
| `api-core` | `backend/api-server/src/` (excl. handlers/) | Rust |
| `reality-server` | `backend/reality-server/src/` | Rust |
| `ppt-web-ui` | `frontend/ppt-web/src/pages/` + `frontend/ppt-web/src/components/` | Frontend |
| `ppt-web-core` | `frontend/ppt-web/src/` (hooks/, api/, stores/, router) | Frontend |
| `reality-web` | `frontend/reality-web/src/` | Frontend |
| `mobile-rn` | `frontend/mobile/src/` | Frontend |
| `mobile-native-kmp` | `mobile-native/` | Kotlin |

### Selection priority

1. **Churn-aligned:** if any `CHURN_FILES` path starts with a segment's root → pick that segment (hot code, warm context).
2. **Oldest unreviewed:** from `REVIEW_CURSOR`, sort by date ascending, null sorts first → pick the oldest.
3. **Fallback:** `api-handlers` (most security-critical).

If the winning segment already has ≥3 open `code-review-finding` items in `backlog.json` with `status == "open"`, skip it and pick the next in priority order. The backlog is saturated for that segment — no value in adding more until the existing findings are addressed.

Log the selected segment and the reason (churn-aligned | oldest-unreviewed | fallback | saturation-skip).

---

## Step 2 — Assign experts

| Segment | Primary expert | Secondary (optional) |
|---------|---------------|----------------------|
| `api-handlers` | Rust | Security |
| `api-core` | Rust | — |
| `reality-server` | Rust | — |
| `ppt-web-ui` | Frontend | Completeness |
| `ppt-web-core` | Frontend | — |
| `reality-web` | Frontend | — |
| `mobile-rn` | Frontend | Tester |
| `mobile-native-kmp` | Kotlin | Tester |

Spawn only the assigned experts as subagents (via the `Agent` tool). Do not spawn all 8 — token budget is the hard constraint.

---

## Step 3 — Expert review instructions

Give each expert subagent:
1. Their role and the patterns to look for (see below).
2. The segment root path.
3. The hard rule: **read files and grep only — no shell commands that compile or run code.**
4. The cap: **report at most 3 findings. Stop at 3. Skip nits.**
5. The output format (Step 4).

### Rust Expert looks for:
```
- unwrap() / expect() in non-test code that touches DB results, HTTP responses, or parsed user input
- unsafe blocks with no comment explaining the safety invariant
- SQL queries (sqlx) missing a WHERE clause on tenant/owner/user ID
- Axum handlers that return StatusCode::OK on an error branch
- todo!() / unimplemented!() macros reachable from production paths (not in test modules)
```

Grep commands to start with:
```bash
grep -rn "\.unwrap()\|\.expect(" <segment-root> --include="*.rs" | grep -v "#\[test\]\|mod tests"
grep -rn "unsafe " <segment-root> --include="*.rs"
grep -rn "todo!()\|unimplemented!()" <segment-root> --include="*.rs"
```

### Security (secondary for api-handlers):
```
- JWT claims read without verifying signature first (no .verify() call before .claims())
- User-controlled string concatenated into a file path or shell command
- Sensitive fields (password, token, secret, api_key) appearing in a log! / tracing:: call
- CORS AllowAny or AllowOrigin::mirror_request without a comment explaining the decision
```

Grep:
```bash
grep -rn "log!\|tracing::" <segment-root> --include="*.rs" | grep -i "password\|token\|secret\|api_key"
grep -rn "AllowAny\|mirror_request" <segment-root> --include="*.rs"
```

### Frontend Expert looks for:
```
- useQuery / useMutation call with no error handling (no isError branch, no onError, no ErrorBoundary above)
- TypeScript `as any` or `as unknown as` cast that crosses an API boundary
- Hardcoded user-facing string that should be in i18n (visible text not wrapped in t('...'))
- console.log / console.error left in committed code
- Next.js getServerSideProps / generateStaticParams that returns without null-checking the API response
```

Grep:
```bash
grep -rn "console\.log\|console\.error" <segment-root> --include="*.tsx" --include="*.ts" | grep -v "//.*console"
grep -rn "as any\b" <segment-root> --include="*.ts" --include="*.tsx"
grep -rn "useQuery\|useMutation" <segment-root> --include="*.tsx" | head -30
```

### Completeness (secondary for ppt-web-ui):
```
- Page or component file that returns null unconditionally or has a body that is only `// TODO`
- List/table page with no empty-state UI (no "No items" fallback)
- API SDK type imported but the endpoint that should back it is missing from the OpenAPI spec
```

### Kotlin Expert looks for:
```
- !! (non-null assertion) on values from network deserialization, Room/DB queries, or intent extras
- CoroutineScope launched in GlobalScope or MainScope without cancellation lifecycle management
- expect/actual where the iOS actual is a no-op stub (empty body or throws NotImplementedError)
- suspend function that can throw a network exception with no try/catch at the call site
```

Grep:
```bash
grep -rn "!!" <segment-root> --include="*.kt" | grep -v "//\|test\|Test"
grep -rn "GlobalScope\." <segment-root> --include="*.kt"
grep -rn "TODO()\|NotImplementedError" <segment-root> --include="*.kt" | grep "actual "
```

### Tester (secondary for mobile-rn / mobile-native-kmp):
```
- Public function or class with no corresponding test file anywhere in the repo
- Test file where every test case is skipped (xit / xtest / @Ignore / .skip)
- Integration test that mocks the very thing being tested (e.g. mocking the repository inside a repository test)
```

### Dependency & version hygiene (all experts — apply when reviewing manifest files)

This applies whenever a finding involves a dependency version (`Cargo.toml`, `package.json`, `build.gradle.kts`, `gradle.properties`).

**Key rules:**
- **Check for net-new versions first.** Before flagging a dep as outdated or suggesting a version change, look up the current latest:
  - Rust: `gh api "https://crates.io/api/v1/crates/<name>" --jq '.crate.newest_version'`
  - Node: `gh api "https://registry.npmjs.org/<name>/latest" --jq '.version'`
  - Kotlin/Gradle: check Maven Central or the library's release page via `gh api` or `Read` on known version URLs
- **Respect dependabot history.** This repo uses dependabot/Renovate for routine updates. Before suggesting any version change, check recent dep-update PRs:
  ```bash
  gh pr list --state merged --base dev --limit 50 \
    --json number,title,labels --jq '[.[] | select(.labels[].name == "dependencies")]'
  ```
  If a dep was bumped recently by dependabot, it is already at (or near) the latest — don't re-flag it as stale.
- **Never suggest a downgrade without evidence.** Downgrades are rare and must be justified by a specific, confirmed problem: a known regression in the newer version, a breaking API change that isn't yet fixed upstream, or a security revert where the patch version introduced a worse vulnerability. A feeling that "the old version was more stable" is not sufficient. If a downgrade seems warranted, emit the finding with `candidate_vector: "bug"` and include a concrete reference (issue URL, changelog entry, test failure) in `evidence`.
- **Don't flag dependabot noise.** If the only finding in a manifest file is "version X.Y.Z could be X.Y.(Z+1)", that is dependabot's job, not a code-review finding. Emit findings only for: pinned versions that are several major versions behind with known CVEs, versions explicitly pinned to an old release with no comment explaining why, or `*`/`latest` wildcards in production manifests.

---

## Step 4 — Output format

Each finding becomes one `code-review-finding` signal. Use this exact shape:

```json
{
  "id": "code-review-<segment>-<kebab-slug>",
  "type": "code-review-finding",
  "source": "rotating-expert-review",
  "segment": "<segment-id>",
  "score_delta": 2,
  "confidence": "medium",
  "candidate_vector": "bug",
  "evidence": ["backend/api-server/src/handlers/auth.rs:142 — unwrap() on DB query, no recovery path"],
  "expert": "rust"
}
```

**Score delta rules:**
| Severity | Delta | Examples |
|----------|-------|---------|
| High | +3 | Security flaw, crash path, data loss, auth bypass |
| Medium | +2 | Unhandled error on user-facing path, missing null check on user input |
| Low | +1 | Refactor smell, missing test for a module, i18n gap |

Confidence is always `medium` for static-analysis findings (no runtime confirmation). Only upgrade to `high` if you traced the full call path from entry point to the issue and are certain it is reachable in production.

**Slug rule:** derive the slug from the file basename + issue type, e.g. `auth-rs-unwrap-db`, `faults-page-missing-error-state`. Keep it under 30 chars, kebab-case.

---

## Step 5 — Assemble and return

Collect all signals from all experts (combined max: 3 total, not 3 per expert). If both experts find issues, keep the 3 highest-severity across both.

Return:
```json
{
  "segment_reviewed": "<segment-id>",
  "selection_reason": "churn-aligned | oldest-unreviewed | fallback",
  "experts_used": ["rust", "security"],
  "findings_count": 2,
  "signals": [ ... ]
}
```

The routine Phase 1.5 adds these signals to `signals/<today>.json` and advances `state.review_cursor.<segment>` to today.

---

## Token budget discipline

The goal is targeted insight, not exhaustive coverage. Each expert should:
- Run the grep commands first to get a hit-list
- Read only the files that had grep hits, plus the 2–3 most important entry-point files for the segment
- Stop after 3 findings regardless of how many more could be found
- Skip files > 500 lines unless a grep hit landed there

If a segment has >40 files, narrow to: files touched by the last 3 PRs that modified the segment + the 3 largest files by line count. Full coverage happens over the rotation, not in a single pass.
