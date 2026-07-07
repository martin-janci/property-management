# security-rustsec-2026-0204-crossbeam-epoch

**Vector:** security
**Score:** 3
**Source:** Issue #2141
**Confidence:** high

## Hypothesis
`crossbeam-epoch 0.9.18` in `backend/Cargo.lock` is pinned to a version with RUSTSEC-2026-0204 — an invalid pointer dereference in the `fmt::Pointer` impl for `Atomic` and `Shared`. Every backend PR now shows a red `cargo-deny` advisory on `dev`, masking any new advisory that lands after this one. Because the crate is a transitive dependency of `metrics-util 0.15.1 -> metrics-exporter-prometheus 0.12.2` (pulled by `api-server`, `reality-server`, `accounting-server`), the fix is a lockfile-only bump: `cargo update -p crossbeam-epoch --precise 0.9.20`. No source, no API surface, no manifest edit.

## Evidence
- Issue #2141 filed 2026-07-07 by the dispatcher — `cargo-deny advisories FAILED on dev: RUSTSEC-2026-0204 (crossbeam-epoch 0.9.18) blocks every backend PR`.
- Advisory text (from the linked `cargo-deny` output): `Invalid pointer dereference in fmt::Pointer impl for Atomic and Shared; Upgrade to >=0.9.20`.
- Dep chain: `crossbeam-epoch v0.9.18` → `metrics-util v0.15.1` → `metrics-exporter-prometheus v0.12.2` (used by three servers).
- Reproducer: `cd backend && cargo deny check advisories` currently returns `advisories FAILED, bans ok, licenses ok, sources ok`. Same error visible in the `cargo-deny` job on PR #2131's check runs (test-only PR whose only failing check is `cargo-deny`).
- `backend/Cargo.lock:1896` — `name = "crossbeam-epoch"` (the pinned entry).

## Files
- `backend/Cargo.lock:1896`

## Dependencies
<!-- no dependencies -->

## Required capabilities
- [ ] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. `cd backend && cargo deny check advisories`.
2. Expected: `advisories ok`. Actual: `advisories FAILED` with `RUSTSEC-2026-0204` naming `crossbeam-epoch 0.9.18` at `backend/Cargo.lock:122` (advisory offset — the crate is defined at `backend/Cargo.lock:1896`).

## Suggested approach
1. `cd backend && cargo update -p crossbeam-epoch --precise 0.9.20` (or the newest 0.9.x clearing RUSTSEC-2026-0204). Lockfile-only edit — no `Cargo.toml` change.
2. `cargo check --workspace` to confirm the compile stays green.
3. `cargo deny check advisories` — expect `advisories ok`.
4. `cargo deny check` (all sections) — confirm `bans`, `licenses`, `sources` still green.
5. Commit `backend/Cargo.lock` alone with `chore(deps): bump crossbeam-epoch to clear RUSTSEC-2026-0204 (#2141)`.
6. Push, open PR against `dev`, verify the `cargo-deny` job goes green on CI.

## Alternatives considered
- **Bump the upstream `metrics-util` / `metrics-exporter-prometheus`** — rejected because those crates haven't cut a version that pins the newer `crossbeam-epoch`, and a top-level bump risks API breakage on our metrics wiring. Transitive lockfile pin is smaller-blast-radius.
- **Add a `[advisories.ignore]` entry in `backend/deny.toml`** — rejected because it silences the advisory without fixing the vulnerability, and would also silence any future crossbeam-epoch advisory.

## Root-cause trace
N/A — security doesn't need backward tracing; the advisory names the exact crate and version. The immediate cause is `backend/Cargo.lock:1896` pinning `0.9.18`; the origin is whichever previous `cargo update` step landed 0.9.18 (predates the current window and not worth tracing for a one-line lockfile bump).

## Test plan
- [ ] `cd backend && cargo deny check advisories` — must exit 0 (`advisories ok`).
- [ ] `cd backend && cargo check --workspace` — must exit 0 (no compile break from the transitive bump).
- [ ] Confirm on CI: the `cargo-deny` job on `backend.yml` for the PR is green.

## Out of scope
- Any `Cargo.toml` edits (this is a lockfile-only bump).
- Wiring `cargo-deny` as a *required* status check on `dev` (that is a separate action item — surfaced by #2141's "not currently a required status check" note; belongs in the CI-gates workstream, not this security fix).
- Auditing any other advisories.

## After-merge
- Move this file to `plans/_archive/security-rustsec-2026-0204-crossbeam-epoch.md`.
- Mark the `security-rustsec-2026-0204-crossbeam-epoch` row in `backlog.json` as `status: "done"`.
