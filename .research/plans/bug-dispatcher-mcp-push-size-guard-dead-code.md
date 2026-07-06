# bug-dispatcher-mcp-push-size-guard-dead-code

**Vector:** bug
**Score:** 3
**Source:** Issue #2126
**Confidence:** high

## Hypothesis
`.research/dispatcher-prompt.md` invokes `mcp-push-size-guard.sh --staged` at ~L2082, AFTER the `git commit` at ~L2066. Post-commit the index equals HEAD, so the guard's L55 `git diff --cached --name-only --diff-filter=ACM` returns an empty set; the guard prints "no files to check — OK" and exits 0 on every run. It never inspects the real push payload, so the fail-closed net PR #2114 advertised does not exist — any state file >64 KiB (e.g. `assignments.json`, which the Fix#1 reconcile does not shrink) still causes MCP-push silent truncation, the exact #1014 class the guard was meant to backstop. The smallest fix is to move the guard invocation BEFORE `git commit` (right after the final `git add`), and add a self-test that exercises the actual `--staged` wire path.

## Evidence
- `.research/dispatcher-prompt.md` — the Phase 6 sequence at ~L2066 (`git commit`) precedes the guard invocation at ~L2082 (`mcp-push-size-guard.sh --staged`). Post-commit, `git diff --cached` is empty.
- `.research/mcp-push-size-guard.sh:55` — the guard reads `git diff --cached --name-only --diff-filter=ACM`. Empty in put → no files to check → exit 0.
- Issue #2126 (severity: high) — post-merge reviewer confirmed by tracing the exact sequence.
- Fix#1 of #2114 (action-list reconcile + T26 warn→fail) IS correctly wired and closes the primary #1014 vector; only Fix#2 (the size-guard backstop) is dead.
- No test exercises `--staged` mode of the guard; PR #2114 verified only explicit-file mode.

## Files
- `.research/dispatcher-prompt.md`
- `.research/mcp-push-size-guard.sh`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (derived from ticks above — C4 / C5 both unticked):**

Mode: cloud-ok

## Repro steps
1. In a scratch worktree, stage a `.research/` file >64 KiB (e.g. `dd if=/dev/urandom of=.research/big.tmp bs=1 count=70000`).
2. `git add .research/big.tmp && git commit -m stub`.
3. Run `bash .research/mcp-push-size-guard.sh --staged`.
4. Expected (after fix): guard exits non-zero and prints the oversize file name. Actual (today): guard prints "no files to check — OK" and exits 0.

## Suggested approach
1. In `.research/dispatcher-prompt.md` around L2050-L2082, move the `bash .research/mcp-push-size-guard.sh --staged` invocation to run BEFORE `git commit` — insert it right after the final `git add` and before the commit step. Keep the fail-closed exit semantics (`|| exit 3`) so an oversize staged file aborts the dispatcher run.
2. Alternative (equivalent): change the guard to accept `--committed` mode and read `git diff --name-only HEAD~1 HEAD` instead of `git diff --cached`. Pick whichever localises the fix best — likely (1) since it keeps the guard single-purpose.
3. Add `.research/test-mcp-push-size-guard.sh` covering three cases:
   - Oversize STAGED file trips exit 3 (was: silent OK).
   - `PUSH_METHOD=git` skips the check entirely (regression pin for the git fast-path).
   - The exact Phase 6 add→guard→commit sequence catches an oversize staged file at guard time, before commit lands.
4. Wire the new self-test into `dispatcher-self-test.yml` (or the existing `dispatcher-self-test.sh` T26 pathway — `T26` already exists as a warn; promote its check to invoke the new script under `set -e`).
5. Verify: run the new test locally; confirm the Phase 6 sequence in `dispatcher-prompt.md` compiles as intended (no other reference to the old position lingers).

## Alternatives considered
- **Delete the guard entirely and rely on Fix#1's action-list reconcile alone** — rejected because Fix#1 shrinks only `action-list.json`. Other files that grow past 64 KiB (`assignments.json`, `assignments-archive.json`, `coverage.json`, `post-merge-review.json`) would still silently truncate on MCP push; the size guard is the general-case net for all such files, not a `action-list.json`-specific fix.
- **Add a repository-wide GitHub Actions size-check job** — rejected because the truncation happens BEFORE the file lands on GitHub (during the MCP push itself). A CI job runs post-push; it would notice the corruption after the fact but not prevent it. The guard needs to run in the dispatcher's push path, where the fix is trivial.

## Root-cause trace
1. Symptom: MCP-push size guard `exit 0`s on every dispatcher run, even when `assignments.json` is oversize; #1014-class silent truncations remain possible.
2. ← `mcp-push-size-guard.sh:55` reads `git diff --cached --name-only --diff-filter=ACM`. Post-commit that is empty by definition (index = HEAD after commit succeeds).
3. ← `.research/dispatcher-prompt.md` Phase 6 orders `git commit` (~L2066) BEFORE the guard invocation (~L2082) — the reverse of what the guard's `--staged` semantics assume.
4. Origin: PR #2114 (2026-07-06) — the guard was correctly implemented but the invocation position in the prompt was wrong. No self-test caught the mis-ordering because PR #2114 verified only explicit-file mode, not the actual `--staged` wire path.

## Test plan
- [ ] `.research/test-mcp-push-size-guard.sh` — new bash test (no DB, no network):
  - Stage a 70 KiB file, run the guard, assert exit 3.
  - Set `PUSH_METHOD=git`, stage same file, run the guard, assert exit 0.
  - Simulate Phase 6 (`git add && bash mcp-push-size-guard.sh --staged && git commit`), assert the commit is aborted before it lands when the staged file is oversize.
- [ ] Promote `T26` in `.research/dispatcher-self-test.sh:628` from `warn` to `fail` and route it through the new script.
- [ ] Command: `bash .research/test-mcp-push-size-guard.sh && bash .research/dispatcher-self-test.sh` — both must exit 0.
- [ ] Static: `shellcheck .research/mcp-push-size-guard.sh .research/test-mcp-push-size-guard.sh` — no warnings.

## Out of scope
- The `action-list.json` reconcile itself (Fix#1 of #2114) — already correct, do not touch.
- Rewriting the MCP push shape to stream chunks — separate, larger effort tracked under #1680.
- Fixing the low-severity regression in #2126 (T26 warn→fail wedges the dispatcher on transient jq errors) — that is a small follow-up, tracked implicitly by the same issue and can ride this PR's tail if trivial.

## After-merge
- Move this file to `plans/_archive/bug-dispatcher-mcp-push-size-guard-dead-code.md`
- Mark the matching `backlog.json` row (`bug-dispatcher-mcp-push-size-guard-dead-code`) as `status: "done"`
