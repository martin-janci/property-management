---
name: ppt-dev-team
description: >
  Orchestrates a multi-expert development team for PPT (Property Management) code review and fixing.
  Use this skill whenever you want a thorough review before merging, releasing, or completing an epic.
  Triggers on: "run the dev team", "10-pass review", "full team review", "expert team review",
  "review with the team", "team review before PR", "run all experts on this".
  The team has 8 roles — Rust, Kotlin, Frontend, Completeness, Owner, Scrum Master, Planner, Tester —
  runs 10 structured review passes, then coordinates all fixes. One Rust compile at a time, always.
when_to_use: You want a thorough, multi-perspective review before merging an epic, cutting a release, or closing a hot-path PR. The 10-pass structure covers correctness, security, architecture, completeness, and tests — heavier than the dispatcher's per-PR reviewer. Triggers on "run the dev team", "10-pass review", "full team review", "team review before PR".
mode: local
---

# PPT Development Team — 10-Pass Review & Fix

Orchestrates 8 expert subagents across 10 structured review passes, then coordinates fixes for every finding.
Works on the current git diff or a specified scope.

## Team Roster

| Role | Responsibilities |
|------|-----------------|
| **Rust Expert** | api-server, reality-server — safety, async correctness, idiomatic patterns, perf |
| **Kotlin Expert** | mobile-native KMP — coroutines, Compose, expect/actual, multiplatform idioms |
| **Frontend Expert** | ppt-web, reality-web, mobile RN — TypeScript, React, TanStack Query, i18n |
| **Completeness Reviewer** | All use-case criteria covered, TODOs resolved, edge cases handled |
| **Product Owner** | Business value, acceptance criteria, user-facing correctness |
| **Scrum Master** | Coordinates the team, tracks findings, produces Master Fix List |
| **Planner** | Architecture, layer boundaries, technical debt, future-proofing |
| **Tester** | Test coverage, test quality, missing scenarios, CI gaps |

## Critical Constraint: One Rust Compile at a Time

Rust compilation is CPU- and memory-intensive. **Never run two Rust compile commands concurrently**, across any agents or passes.

Rules:
- Before compiling: announce "RUST COMPILE STARTING — {api-server|reality-server}"
- After compiling: announce "RUST COMPILE DONE — released"
- `cargo check` for type-check-only passes (fast)
- `cargo test --workspace` only in Pass 7 and final verification
- Two Rust crates compile sequentially: api-server first, then reality-server

All other agent work (static analysis, reading files, non-Rust checks) may run in parallel.

---

## Execution Flow

### Step 0 — Determine Scope

```bash
# Changed files vs main
git diff main...HEAD --name-only

# Or for a specific commit
git show --name-only <sha>
```

Group files into domains:
- **Rust** — `backend/api-server/`, `backend/reality-server/`
- **Kotlin** — `mobile-native/`
- **Frontend** — `frontend/ppt-web/`, `frontend/reality-web/`, `frontend/mobile/`
- **Config/Infra** — migrations, docker, CI
- **Tests** — test files across all domains

Share the scope with all agents before starting.

---

### Phase 1 — 10 Review Passes

Run passes in order. Parallel where safe, serialized for Rust compiles.

---

**Pass 1 — Architecture & Contracts**
Agents: Planner + Product Owner (parallel)
- Planner: layer boundaries, API contracts (OpenAPI), DB schema decisions, coupling
- Product Owner: does the implementation match the epic/story? Are acceptance criteria achievable?
- Output: architectural concerns, contract gaps

---

**Pass 2 — Rust Safety & Correctness** *(serialized compile)*
Agent: Rust Expert
```bash
cd backend/api-server && cargo check 2>&1
cd backend/reality-server && cargo check 2>&1  # after api-server finishes
```
Review: `unwrap()`/`expect()` usage, error propagation with `?`, lifetime issues, unsafe blocks,
SQLx query type safety, Axum handler signatures, JWT/auth middleware correctness.
Output: Rust issues with file:line references, severity (Critical/High/Medium/Low)

---

**Pass 3 — Kotlin/KMP Quality**
Agent: Kotlin Expert (parallel with any non-Rust work)
Review: coroutine scope management, `expect`/`actual` alignment, Compose side effects,
null safety, Flow vs suspend fn choice, Gradle dependency alignment between platforms.
Output: Kotlin issues with file references

---

**Pass 4 — Frontend Quality**
Agent: Frontend Expert (parallel)
Review: TypeScript strict-mode violations, React hook rules, TanStack Query cache keys and
invalidation, i18n string coverage (sk/cs/de/en), Vite bundle concerns, SSR/ISR correctness
in reality-web, SDK type alignment with backend OpenAPI.
Output: frontend issues with file:line references

---

**Pass 5 — Security & Auth**
Agents: Rust Expert + Planner (parallel, no compile needed)
- JWT validation paths, RLS tenant isolation in SQL, input sanitization, CORS config
- S3 presigned URL expiry, secret exposure in logs, SSRF risk in any URL handling
- OAuth flows: token lifetimes, refresh token rotation, scope enforcement
Output: security findings by severity; Critical findings block all further passes until addressed

---

**Pass 6 — Error Handling & Resilience**
Agents: Rust Expert + Kotlin Expert + Frontend Expert (parallel)
- Rust: HTTP status code correctness, error type hierarchy, no silent swallowing of errors
- Kotlin: exception → sealed Result mapping, null recovery, network retry logic
- Frontend: loading/error/empty states on every async operation, user-facing error messages
Output: resilience gaps per domain

---

**Pass 7 — Test Coverage** *(serialized Rust compile)*
Agent: Tester, with compile support from Rust Expert
```bash
cd backend && cargo test --workspace 2>&1   # serialized
cd frontend && pnpm test 2>&1               # after Rust finishes
```
Review: unit test coverage, integration test gaps, mock vs real DB usage, missing happy-path
and error-path tests, snapshot drift in frontend, flaky test patterns.
Output: coverage report, list of missing tests by priority

---

**Pass 8 — Completeness**
Agents: Completeness Reviewer + Product Owner (parallel)
- Compare implementation line-by-line against the story spec / use-case requirements
- Hunt for: TODO/FIXME left in code, commented-out code blocks, stub handlers, placeholder copy
- Product Owner: walk the user-facing flows and confirm they match the design
Output: completeness checklist — each acceptance criterion marked pass/fail

---

**Pass 9 — Performance & Scalability**
Agents: Rust Expert + Frontend Expert + Planner (parallel, no compile)
- Rust: N+1 query patterns, missing DB indexes for new queries, connection pool sizing, Redis TTLs
- Frontend: unnecessary re-renders, missing `useMemo`/`useCallback`, bundle chunk sizing
- Planner: will this hold at 10x load? Any architectural bottlenecks introduced?
Output: perf findings and concrete recommendations

---

**Pass 10 — Final Holistic Review**
All agents review the combined picture from passes 1–9.
Scrum Master consolidates all findings, deduplicates, and produces the **Master Fix List**.

**Master Fix List format:**
```
P1 — Blocking (must fix before merge)
  [RUST-002] backend/api-server/src/handlers/auth.rs:142 — unwrap on DB error
  [SEC-001]  backend/api-server/src/middleware/auth.rs:88 — JWT not validating `aud` claim
  ...

P2 — Should Fix (fix in this branch)
  [FE-003]  frontend/ppt-web/src/pages/Faults.tsx:67 — missing error state
  ...

P3 — Nice to Have (log as TODO, address later)
  [PERF-001] backend/api-server/src/handlers/buildings.rs:201 — N+1 could be batched
  ...
```

---

### Phase 2 — Fix

1. Scrum Master groups P1 and P2 fixes by domain, assigns to the relevant expert agent
2. Fix P1 items first, then P2
3. After each Rust fix:
   ```bash
   cargo check   # serialized — one at a time
   ```
4. After all fixes applied:
   ```bash
   cargo test --workspace   # serialized
   cd frontend && pnpm type-check && pnpm test
   ```
5. Tester verifies that modified/new tests pass and no regressions
6. Completeness Reviewer confirms all P1/P2 items are resolved
7. Product Owner gives final sign-off

P3 items are not fixed — they become `// TODO(team-review): <description>` comments with enough context for a future developer to act on them.

---

### Phase 3 — Summary

Output a structured summary:

```
## Dev Team Review Summary

### Findings
- Pass 1 (Architecture): N findings
- Pass 2 (Rust):         N findings  [M Critical, K High]
- ...
- Total: N findings (P1: X, P2: Y, P3: Z)

### Fixes Applied
- [RUST-002] Fixed: removed unwrap, propagated error properly
  File: backend/api-server/src/handlers/auth.rs:142
- ...

### Unresolved (P3 — logged as TODOs)
- [PERF-001] backend/api-server/src/handlers/buildings.rs:201

### Build Status
- cargo check:          PASS
- cargo test --workspace: PASS (N tests)
- pnpm type-check:      PASS
- pnpm test:            PASS (N tests)
```

---

## Committing After Fix

Follow PPT commit convention:

```
fix(epic-N): dev team review findings — story N.M

P1/P2 fixes:
- [RUST-002] propagate DB error in auth handler (auth.rs:142)
- [SEC-001] validate JWT aud claim (middleware/auth.rs:88)
- [FE-003] add error state to Faults page (Faults.tsx:67)
...
```

---

## Dispatching Agents

Use the `Agent` tool with `subagent_type=claude` for each expert role.
Pass each agent:
1. Their role, focus, and the changed-file scope
2. Their specific pass instructions (copy the relevant pass block above)
3. A reminder of the Rust compile serialization rule

For passes with multiple agents and no compile step: spawn all in a single message (parallel).
For passes with a Rust compile: run the Rust Expert sequentially, others in parallel if they don't need compile output.
