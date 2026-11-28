# Stale PR Triage — 2026-11-28

Owner role: `pm-devops` · Priority: medium

This note triages the open, stale pull requests flagged by the research
dispatcher — seven Dependabot PRs (#3443, #3444, #3446, #3447, #3448, #3449,
#3634) and two long-lived human feature PRs (#2555, #2559). For each it records
a recommendation — **merge**, **rebase**, or **close** — with the reasoning
(staleness, CI state, conflict likelihood, blast radius).

It is a **planning artifact only**. No PR is merged, closed, or force-pushed by
this note, and `.research/**` backlog state is left untouched. The recommendations
are for the human maintainer (`@martin-janci`, the requested reviewer / author on
every row) to action. CI conclusions below are read from each PR's head-commit
check runs as of 2026-11-28; "age" is measured from each PR's last update.

## How to read the classification

- **Merge** — CI is green on the head commit and the change is low-risk (patch /
  minor version, or a revert-of-breaking-change). The only caveat is base drift:
  update the branch first (`Update branch` button / `@dependabot rebase`), let CI
  re-run green, then merge. No code adaptation needed.
- **Rebase** — the change is wanted and substantive, but it cannot land as-is:
  either CI is red on a *recoverable* basis (stale base, needs re-run) or the
  branch is far enough behind `dev` that a conflict/behavioral re-verification is
  required before review can conclude. The work is **not** thrown away.
- **Close** — the change is a **major** dependency bump whose upstream carries
  breaking API / MSRV changes, and CI confirms it does not compile or test clean
  against the current tree. Dependabot's own rebase cannot fix a compile break —
  these need a *deliberate, code-adapting migration task*, not a drive-by merge.
  Closing with a tracked follow-up is cleaner than leaving a permanently-red PR
  open. (Close here means: maintainer closes the PR and opens the follow-up; this
  note does not close anything.)

## Summary

| PR | Title (dep / feature) | Stack | Head CI | Age | Recommendation |
|----|-----------------------|-------|---------|-----|----------------|
| #3446 | expo-secure-store 57.0.3 → 58.0.2 | frontend (RN/Expo) | ✅ green | ~16d | **Rebase → merge** (as Expo-58 group) |
| #3448 | expo-device 57.0.2 → 58.0.5 | frontend (RN/Expo) | ✅ green | ~16d | **Rebase → merge** (as Expo-58 group) |
| #3449 | expo-status-bar 57.0.1 → 58.0.4 | frontend (RN/Expo) | ✅ green | ~16d | **Rebase → merge** (as Expo-58 group) |
| #3634 | kotlinx-serialization-json 1.12.0 → 1.13.0 | mobile-native (KMP) | ✅ green | ~2d | **Merge** (update branch first) |
| #3443 | utoipa-swagger-ui 9.0.2 → 10.0.2 | backend (Rust) | ❌ `check`/`test-build`/`test` fail | ~16d | **Close** + tracked major-bump task |
| #3444 | jsonwebtoken 10.4.0 → 11.1.0 | backend (Rust, auth) | ❌ `check`/`test` fail | ~16d | **Close** + tracked auth-migration task |
| #3447 | vitest 4.1.11 → 5.1.0 | frontend (dev dep) | ❌ `frontend`/web builds fail | ~14d | **Close** + tracked test-runner task |
| #2555 | feat(acc): invoice sent/cancelled lifecycle (UC-ACC-05.17) | backend (accounting) | ⚠️ green at creation, now stale | ~4mo | **Rebase** → re-review |
| #2559 | feat(acc): PAY by square QR endpoint (UC-ACC-05.8) | backend (accounting) | ⚠️ green at creation, now stale | ~4mo | **Rebase** → re-review |

---

## Dependabot PRs

### #3446 / #3448 / #3449 — Expo modules 57 → 58 — **Rebase → merge (as a group)**

Three sibling Dependabot PRs bump individual Expo modules from the 57 line to the
58 line:

- #3446 `expo-secure-store` 57.0.3 → 58.0.2
- #3448 `expo-device` 57.0.2 → 58.0.5
- #3449 `expo-status-bar` 57.0.1 → 58.0.4

**CI:** all three are **green** on their head commits — `frontend`, `Build
Property Management Web`, `Build Reality Portal Web`, `Build Property Management
Mobile`, `Test Property Management Web`, and the E2E result gates all pass. The
diffs are small (pnpm lockfile + manifest).

**Reasoning:** these are green and low-risk at the JS/lockfile layer. The one
caveat is **SDK-line alignment**: jumping three native modules from 57 to 58
should be done as one coherent step (and ideally alongside the rest of the Expo
58 module set and the Expo SDK/`expo` package itself), not as three independent
merges landing at different times — mismatched native module majors are a known
source of runtime-only breakage that the JS CI here does not exercise. Green CI
covers typecheck/build/web-E2E but **not** a native device run.

**Action:** update each branch onto current `dev` (`@dependabot rebase`), confirm
CI stays green, and **merge the three together** in one sitting. Before merging,
the maintainer should confirm the app's `expo` core and remaining `expo-*`
modules are (or are being moved) to the 58 line so the SDK set stays internally
consistent; if the project is deliberately pinned to the 57/SDK line, close all
three instead and let Dependabot re-raise on the next SDK bump.

### #3634 — kotlinx-serialization-json 1.12.0 → 1.13.0 — **Merge**

**CI:** **green** — `build-android`, `build-ios`, `check`, `frontend` and the web
E2E gates all pass on the head commit. It is also the **freshest** PR in this
batch (opened 2026-11-19, last updated 2026-11-26).

**Reasoning:** low-risk minor bump, and notably a **de-risking** one — upstream
1.13.0 *reverts* the exception-rework breaking change that 1.12.0 introduced
(`Json` no longer intercepts/wraps serializer exceptions in a `JsonException`) and
adds a `JsonElement.toString` perf improvement. One-line change to
`gradle/libs.versions.toml` (the kotlin Dependabot group). Nothing in the
mobile-native Ktor/serialization path depends on the 1.12 interception behavior.

**Action:** click **Update branch** to pull current `dev` (base is behind), let CI
re-run, and squash-merge. This is the strongest merge candidate in the set.

### #3443 — utoipa-swagger-ui 9.0.2 → 10.0.2 — **Close** (+ tracked task)

**CI:** **red** — `check` (clippy), `test-build`, and the final `test` gate all
**fail** on the head commit. `cargo-deny` passes; the failure is compile/type.

**Reasoning:** this is a **major** bump (9 → 10) across the `utoipa` family. The
upstream 10.0.0 release notes include "**Upgrade MSRV to 1.88**", a `base64`
22 → 23 bump, and the swagger-ui asset-embedding rework. The repo pins
`rust-version = 1.75` (`backend/Cargo.toml`) and uses `utoipa` 5 /
`utoipa-swagger-ui` 9 as the documented stack (CLAUDE.md "Tech Stack → backend").
A Dependabot rebase cannot fix a type/MSRV break — this needs a deliberate
`utoipa` 5→? + swagger-ui 9→10 migration with an MSRV decision.

**Action:** **close** the PR and open a tracked backend chore ("evaluate utoipa /
utoipa-swagger-ui 10 upgrade; requires MSRV bump to 1.88 and base64 23"). Keeping
a permanently-red major-bump PR open adds noise to the queue without moving the
upgrade forward.

### #3444 — jsonwebtoken 10.4.0 → 11.1.0 — **Close** (+ tracked auth task)

**CI:** **red** — the final `test` gate and `check` (clippy) **fail** on the head
commit. (The parallel `test-shard (1–4)` and `test-build` jobs went green, which
tells us the break is concentrated in the crates that touch the JWT API, not the
whole workspace — consistent with a localized breaking-API change.)

**Reasoning:** **major** bump (10 → 11) and **auth-critical**. The 11.0.0
changelog is a wall of BREAKING changes directly in the surface this repo uses:
`EncodingKey.inner` → `as_bytes`, `DecodingKey.as_bytes`/`try_get_hmac_secret`
removed, `Validation.insecure_disable_signature_validation` removed, several enums
made `#[non_exhaustive]`, `Jwk.thumbprint` now returns `Result`. The documented
stack is `jsonwebtoken` 9.2 (CLAUDE.md) — the tree is already ahead at 10.4, and
11 is another breaking step. This must be migrated deliberately, with the security
reviewer in the loop, not merged as a dependency drive-by.

**Action:** **close** and open a tracked **security/auth** task to migrate the JWT
encode/decode call sites to the jsonwebtoken 11 API and re-green clippy+tests.
Route it through `pm-security` review given the auth blast radius.

### #3447 — vitest 4.1.11 → 5.1.0 — **Close** (+ tracked task)

**CI:** **red** — `frontend`, `Build Property Management Web`, `Build Reality
Portal Web`, `frontend-images`, and the `ppt-web` E2E gate **fail**; the
reality-web docker build failed and the ppt-web docker build was cancelled as a
result.

**Reasoning:** **major** test-runner bump (4 → 5). CLAUDE.md pins **Vitest 4** as
the frontend test stack; a v5 jump changes config/reporter/coverage surfaces and
is clearly not green against the current workspace. This is a toolchain migration
(touching 12 files per the PR), not a lockfile patch.

**Action:** **close** and open a tracked frontend chore to adopt Vitest 5
deliberately (config migration + fix the failing builds/E2E) when the team wants
to move the test toolchain. Until then the pin to 4 stands.

---

## Human feature PRs

Both are authored by the maintainer, were **green when opened (2026-07-28)**, and
have sat untouched since 2026-07-30 — roughly **four months**. Their base commits
(`48dd433…` for #2555, `6da4f78…` for #2559) are far behind the current `dev`
tip, so GitHub's mergeability is stale/uncomputed and a conflict-free replay is
very unlikely — especially for #2555, which adds a DB **migration** and extends a
status **enum** (migration numbering and enum/CHECK constraints are the classic
collision points after months of drift). Neither should be **closed**: they are
substantive, test-backed slices of the deliberately-sequenced accounting
"close the MVP loop" Phase A work, and their value has not expired.

### #2555 — invoice sent/cancelled lifecycle (UC-ACC-05.17) — **Rebase**

390 additions across 7 files: wires `POST /invoices/{id}/send` and
`POST /invoices/{id}/cancel`, adds the `cancel_invoice_rls` guard, blocks
payment-status updates and credit notes against cancelled documents, and extends
`InvoiceStatus` with `Sent` + `Cancelled` to match the 00202 migration's DB CHECK.
Ships 3 new `#[sqlx::test]` lifecycle tests (not quarantined). CI was fully green
at creation (test / security-tests / rls-smoke / sqlx-migrations all passed).

**Reasoning:** wanted feature, was green, test-backed — but four months stale with
a migration + enum change that almost certainly now conflicts with `dev`. The
recorded green CI is from July and no longer authoritative.

**Action:** **rebase** onto current `dev` (renumber the migration if 00202 was
taken, reconcile the `InvoiceStatus` enum against any intervening changes),
re-run CI, then put through normal review. Maintainer to confirm the UC-ACC-05
lifecycle is still the intended design before investing the rebase.

### #2559 — PAY by square QR endpoint (UC-ACC-05.8) — **Rebase**

459 additions across 8 files: `GET /invoices/{id}/qr` returning `{ pay_by_square,
spayd }`, a new `accounting_core::bysquare` PAY-by-square v1.2.0 encoder/decoder
(CRC32 + LZMA1 + Base32Hex), new deps `crc32fast` + `lzma-rs`. 7 unit tests
including an upstream reference-vector round-trip. CI green at creation.

**Reasoning:** same shape as #2555 — self-contained, well-tested feature, explicit
companion to #2555 in the Phase A sequence, but four months stale. Lower conflict
risk than #2555 (no migration; new module + new deps), but the new `Cargo.toml`
dependencies and `dev`'s `Cargo.lock` will need reconciliation, and it should land
*after/with* #2555 for a coherent lifecycle+QR story.

**Action:** **rebase** onto current `dev`, resolve `Cargo.lock`, re-run CI, then
review. Sequence behind #2555.

---

## Recommended action order for the maintainer

1. **#3634** — update branch, merge (green, freshest, de-risking revert).
2. **#3446 + #3448 + #3449** — rebase, confirm Expo-58 SDK alignment, merge as one group.
3. **#2555 then #2559** — rebase the accounting Phase-A pair, re-run CI, review/merge in that order.
4. **#3443, #3444, #3447** — close, and open one tracked migration task each (utoipa 10 / MSRV, jsonwebtoken 11 / auth, vitest 5 / frontend toolchain).

> Scope note: every mutating step above (merge / close / rebase-push / branch
> update) is left to the maintainer. This triage touches only
> `docs/process/triage/` and takes no GitHub action on the referenced PRs.
