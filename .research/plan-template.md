# <slug>

**Vector:** <bug|refactor|perf|test-gap|dx|security|dep-update>  (no `triage` — those stay in backlog)
**Score:** <N>
**Source:** PR #<num> | Issue #<num> | commit <sha> | hotspot in <path>
**Confidence:** <low | medium | high>

## Hypothesis
<2–4 sentences. What's the problem, why it matters, smallest change that resolves it.>

## Evidence
<Max 5 bullets. Each names a concrete artifact.>
- <PR url, commit sha, or file:line>
- <…>

## Files
<Concrete paths the plan touches. ≥1 path required (G7 verifies it exists).
Paths are relative to the repo root and must currently exist on `main`.>
- `<path/to/file>:<line?>`
- `<path/to/file>`

## Required capabilities
<Tick the ones the implementation agent needs. See implementer-prompt.md
for what each provides. Be honest — over-asking wastes setup time,
under-asking blocks the agent mid-flight.>
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [ ] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
- If C4 or C5 is ticked → `local` (implementer must run on the user's Mac)
- Otherwise → `cloud-ok` (can run as a claude.ai routine via `ppt-bridge` MCP at `https://p.rlt.sk/mcp`)

State the mode explicitly in this section under the checklist, e.g.
`Mode: cloud-ok` or `Mode: local-only (reason: C4 — flow needs Chrome DOM inspection)`.

## Repro steps
<Smallest deterministic sequence that reproduces the problem the plan
solves. The implementation agent uses this to author the failing-on-main
test (IG3). One numbered list, each step concrete enough to paste.>
1. <…>
2. <expected vs actual at the end>

## Suggested approach
<Max 7 numbered steps. Reference files by path with line numbers when known.>
1. <…>
2. <…>

## Alternatives considered
<Exactly 2 bullets. Each names another approach you weighed and the concrete reason you rejected it.>
- **<alt name>** — rejected because <…>
- **<alt name>** — rejected because <…>

## Root-cause trace
<Required for vectors `revert`, `risky-churn`, `bug` with confidence ≥ medium.
Otherwise: write `N/A — <vector> doesn't need backward tracing.`
Trace data flow from the failure symptom backward through layers: which
boundary leaked, which assumption broke, which contract was implicit. Name
the file:line for each step.>

1. Symptom: <observed behavior / failing test / stack-trace tip>
2. ← <immediate cause at <file:line>>
3. ← <upstream cause at <file:line>>
4. Origin: <commit sha or PR # that introduced the latent issue>

## Test plan
- [ ] <unit/integration test that would have caught this — file path or test name>
- [ ] <regression scenario>
- [ ] <exact command to run locally: `cargo test -p foo` / `pnpm -F bar test` / etc.>

## Out of scope
<Explicit non-goals so the implementation agent doesn't bloat the PR.>

## After-merge
- Move this file to `plans/_archive/<slug>.md`
- Mark the matching `backlog.json` row as `status: "done"`
