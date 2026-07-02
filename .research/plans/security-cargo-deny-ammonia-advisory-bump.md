# security-cargo-deny-ammonia-advisory-bump

**Vector:** security
**Score:** 3
**Source:** Issue #2009
**Confidence:** high

## Hypothesis
`cargo-deny` (advisories category) fails on every backend PR that runs the check job because `ammonia v4.1.2` has a RUSTSEC advisory (HTML-sanitisation bypass via tags parsed as raw text — `title`, `textarea`, `xmp`, `iframe`, `noembed`, `noframes`, `plaintext`, `noscript`, `style`, `script`). This is currently a merge blocker for all backend PRs and masks whether individual changesets are clean. The smallest fix is `cd backend && cargo update -p ammonia` to `>=4.1.3` and commit the `Cargo.lock` delta.

## Evidence
- Issue #2009 — `cargo-deny check advisories` fails with `ammonia v4.1.2 └── api-server v0.3.218` on any backend PR.
- Fix path stated in the issue: `cargo update -p ammonia` (or the advisory-permitted `>=4.0.2, <4.1.0` / `>=3.3.2, <4.0.0` ranges).
- Reproduced on PR #2007 (a 5-line doc-only comment change that could not possibly touch dependencies) — proves the failure is environmental, not per-PR.
- `backend/deny.toml` currently has no explicit ammonia override; the advisory should not need a manual `ignore` entry once the bump lands.
- Ammonia's `allow` set in the codebase does not include the affected raw-text tags — the advisory is a fail-closed policy trip, not an actual exploit path for PPT today.

## Files
- `backend/Cargo.lock`
- `backend/deny.toml`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
- Neither C4 nor C5 ticked → `cloud-ok` (can run as a claude.ai routine via the ppt-bridge MCP endpoint).

Mode: cloud-ok

## Repro steps
1. Check out any backend PR that touches a `.rs` file (e.g. PR #2007 as recorded in #2009).
2. Run `cd backend && cargo deny check advisories`.
3. Expected: exit 0, "advisories ok". Actual: `advisories FAILED, bans ok, licenses ok, sources ok` with `ammonia v4.1.2` cited.

## Suggested approach
1. `cd backend && cargo update -p ammonia` — expect `Cargo.lock` to move ammonia to `>=4.1.3` (or the highest allowed range).
2. Re-run `cargo deny check advisories` locally — must be green.
3. `cargo check --workspace` — confirms no compile break from the transitive bump.
4. If `>=4.1.3` introduces a breaking API change, fall back to `>=4.0.2, <4.1.0` or `>=3.3.2, <4.0.0` per the advisory-permitted ranges; add a `[dependencies]` pin in `backend/Cargo.toml` only if the transitive resolver misses the allowed range.
5. Commit the `Cargo.lock` delta (and any minimal `Cargo.toml` pin) with `chore(backend): bump ammonia to >=4.1.3 to clear RUSTSEC advisory (#2009)`.
6. Land as a small standalone PR; CI on the same PR proves the fix.
7. No `deny.toml` `ignore` block should be added — the whole point is to remove the fail-closed trip, not to silence it.

## Alternatives considered
- **Add `[advisories] ignore = [\"RUSTSEC-…\"]` in `deny.toml`** — rejected because it silences the advisory permanently and masks a real class of HTML-sanitisation bypasses if the app ever starts allowing raw-text tags; the bump is smaller and safer.
- **Pin ammonia in `backend/Cargo.toml` at `= "4.1.3"`** — rejected because caret ranges are the workspace convention; only pin if the resolver refuses to pick the advisory-permitted range on its own.

## Root-cause trace
N/A — security-vector dep-update. The advisory is upstream; there is no PPT commit that "introduced" it. The trigger is fail-closed policy on a transitive advisory, not application-code behavior.

## Test plan
- [ ] Run `cargo deny check advisories` inside `backend/` after the bump — expect exit 0.
- [ ] Regression: run the same command on the PR's own workflow (`backend.yml` — advisories job) — must go green where every other current PR is red.
- [ ] Local command: `cd backend && cargo update -p ammonia && cargo deny check advisories && cargo check --workspace`

## Out of scope
- Auditing every other RUSTSEC advisory in the tree — this plan is the ammonia bump, nothing more.
- Reworking `deny.toml`'s policy layout.
- Any application-code change to sanitiser call sites — the bump is transitive, no PPT call sites should need to change.

## After-merge
- Move this file to `plans/_archive/security-cargo-deny-ammonia-advisory-bump.md`
- Mark the matching `backlog.json` row as `status: "done"`
