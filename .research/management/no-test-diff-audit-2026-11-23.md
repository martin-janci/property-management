# No-test-diff PR audit — 2026-11-23

_Owner: pm-qa. Task: `pm-qa-no-test-diff-sweep-rate-2026-11-23` (priority low)._

**Question.** This window's merged-PR metric shows a 50% no-test-diff rate.
Classify each flagged PR as **docs-only**, **inline-`#[cfg(test)]`**, or
**real test-gap**, and decide whether the rate needs a corrective gate or a
legitimate explanation.

**Verdict.** The rate is almost entirely a **detector defect, not a QA
defect**. Of the four flagged PRs, **zero are real test-gaps**: two ship tests
the current detector cannot see (an inline Rust `#[cfg(test)]` module and a
shell test harness under `.claude/skills/`), one is docs-only, and one threads
an already-unit-tested helper through call sites. The corrective action is to
**fix the detector's classification rules**, not to add a blocking per-crate
test-ratio gate on top of a miscounting signal.

## Classification

| PR | Files | Classification | Ships tests? | Legit test-less? |
|----|-------|----------------|--------------|------------------|
| #3714 | 30 (ppt-web) | mechanical refactor — tested-helper delegation | existing (helper) | yes |
| #3718 | 3 (ppt-pr-merge skill) | tooling + **shell test harness** (detector-blind) | yes (`.sh`) | yes |
| #3724 | 2 (api-server) | **inline `#[cfg(test)]`** (detector-blind) | yes (inline) | yes |
| #3728 | 1 (screen doc) | **docs-only** | n/a | yes |

### PR #3714 — i18n locale threading (30 ppt-web files)

Pure find-replace: swaps `.toLocaleDateString()` / `.toLocaleString()` /
`new Intl.NumberFormat(undefined, …)` for the same calls parameterised with
`getActiveLocale()` from `src/lib/locale.ts`, plus a `getUserLocale()`
re-export shim in `features/financial/utils/formatting.ts`.

The behaviour-bearing unit is `getActiveLocale()`, and it **is already
unit-tested** — `frontend/apps/ppt-web/src/lib/locale.test.ts` covers the
`i18n.language` → locale resolution and an en-vs-sk divergent-formatting case
(`1234567.89` → comma vs space/comma grouping). The 30 edits are typed
delegations to that tested helper; TypeScript + the helper's own suite cover
them. Adding a snapshot test per call site would be low-value churn.

**Not a real gap.** Classify: legitimately test-less (mechanical delegation of
a covered helper).

### PR #3718 — locale-JSON key-union merge resolver (ppt-pr-merge skill)

Three files: `SKILL.md` (docs), **new** `scripts/merge-locale-json.py` (a
249-line structural three-way JSON merger), and **new**
`scripts/test-merge-locale-json.sh` — a 10-fixture harness covering key-union,
one-sided change/delete, same-key-divergent refusal (exit 2), object-vs-scalar
refusal, `--sort-keys`, non-ASCII preservation, and an end-to-end git-index
merge.

This PR **ships a thorough test suite**. It is flagged only because the
detector does not treat `test-*.sh` under `.claude/skills/**` as a test file.

**Not a real gap.** Classify: detector-blind (shell test harness outside the
recognised test-path globs).

### PR #3724 — voice-webhook + APNs characterization (api-server)

Two files: `routes/voice_webhooks/parsers.rs` adds four characterization tests
to its existing `mod tests`; `services/push/apns.rs` extracts the stale-token
decision into a pure `apns_reason_indicates_stale()` and adds a five-test
`#[cfg(test)]` suite pinning exactly which APNs reasons evict a device token.

This PR is **almost entirely tests**. It is flagged only because the tests live
in inline `#[cfg(test)] mod tests` blocks inside the modified `.rs` files —
no separate `*_test.rs` / `tests/` path is touched, which is the only thing a
path-pattern detector looks for.

**Not a real gap.** Classify: detector-blind (inline `#[cfg(test)]`).

### PR #3728 — Create Lease screen-map reconciliation

One file: `docs/screens/ppt/lease-create.md` — flips `buildStatus` to
`shipped`, rewrites the States section, adds an Agent Log entry. Pure
screen-map documentation reconciling with already-merged PR #3700.

**Not a real gap.** Classify: docs-only (correctly test-less).

## Why the rate reads 50%

The metric counts a merged PR as "no-test-diff" when its diff touches no path
matching the recognised test globs (`*.test.ts[x]`, `*_test.rs`, `tests/**`,
etc.). That definition miscounts three legitimate categories:

1. **Inline `#[cfg(test)]` modules** — the Rust-idiomatic location for unit
   tests. A PR editing `foo.rs` and adding tests to its `mod tests` touches no
   separate test path and is flagged. (#3724)
2. **Non-`*.test.*` test harnesses** — `test-*.sh` / `test_*.py` fixtures,
   notably under `.claude/skills/**` and `scripts/`. (#3718)
3. **Legitimately test-less diffs** — docs / screen-maps (#3728) and mechanical
   delegations to an already-tested helper (#3714).

None of the four flagged PRs is a diff that changed production behaviour
without any corresponding test. The 50% is an artefact of (1)+(2) being
mislabelled and (3) being expected.

## Recommendation — fix the detector before any ratio gate

Publish this taxonomy (pm-qa role item 2 / 26) as the prerequisite for the
proposed per-crate test-ratio gate. **Do not** ship the ratio gate against the
current signal — it would block PRs like #3724/#3718 that already test well.

Detector rules to correct first:

- **Count inline Rust tests.** Treat a `.rs` hunk that adds or changes lines
  inside a `#[cfg(test)]` / `mod tests` region as a test diff. Cheap
  approximation: in a modified `.rs`, count added lines under a `#[cfg(test)]`
  attribute or a `#[test]`/`#[tokio::test]` line as test additions.
- **Recognise non-`*.test.*` harnesses.** Add `test-*.sh`, `test_*.py`,
  `*_test.sh`, and `.claude/skills/**/scripts/test-*` to the test-path set.
- **Exempt, don't flag, the test-less-by-design classes:** docs-only diffs
  (`docs/**`, `**/*.md`), screen-map-only diffs (`docs/screens/**`), and
  lockfile/generated-only diffs. These should drop out of the denominator
  rather than count as "no-test-diff".
- **Only then** compute a per-crate test-ratio gate on the cleaned signal, and
  surface the remaining true no-test-behaviour-change PRs for review rather
  than hard-blocking.

Expected effect on this window: all four audited PRs reclassify as
legitimately covered or exempt, and the headline no-test-diff rate drops to
reflect genuine behaviour-change-without-test PRs only.
