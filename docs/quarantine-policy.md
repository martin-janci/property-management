# Test Quarantine-Aging Policy

Every disabled (`#[ignore]`d) Rust test must be **owned** and must have a
**deadline**. A quarantine with no owner and no deadline rots forever: PR #3675
("release 248 quarantined tests") had to un-quarantine ~226 reason-only tests in
one pass precisely because the backlog had grown unbounded with nobody on the
hook to burn it down. This policy makes that impossible to repeat.

## The contract

A quarantined test is any Rust test carrying an `#[ignore = "..."]` attribute.

1. **Reason** — enforced by [`ignore-reason-gate.yml`](../.github/workflows/ignore-reason-gate.yml).
   Every `#[ignore]` must be written as `#[ignore = "<reason>"]`; a bare
   `#[ignore]` fails CI (issue #2032).
2. **Owner + deadline** — enforced by
   [`quarantine-age-gate.yml`](../.github/workflows/quarantine-age-gate.yml).
   Every `#[ignore]`d test must have:
   - an **owner** — an accountable `pm-*` role (`pm-backend`, `pm-qa`,
     `pm-security`, `pm-devops`, …); and
   - a **deadline** (`due=YYYY-MM-DD`) — the date by which the test must be
     fixed, un-ignored, or re-triaged.

   When a deadline passes, **CI fails** until the test is resolved or re-triaged
   with a new deadline. The gate also runs on a daily schedule so a quarantine
   that ages out while its branch is idle is still caught.

## Where owner + deadline live

Two equivalent sources — the gate accepts either (inline wins if both exist):

### Inline (preferred for new, single quarantines)

Put the tokens in the reason string:

```rust
#[ignore = "BIT-440: workspace hostage under the parallel runner (owner=pm-backend; due=2027-02-19)"]
```

The reason must contain an `owner=<role>` token and a `due=YYYY-MM-DD` token.
This keeps the full contract visible at the point of ignore and in the diff.

### Registry (for the grandfathered set / bulk bookkeeping)

[`.github/quarantine/registry.toml`](../.github/quarantine/registry.toml) holds
one row per quarantined test, keyed by `<path-relative-to-repo-root>::<test-fn>`:

```toml
[[quarantine]]
test  = "backend/servers/api-server/tests/suites/analytics_portfolio_success_tests.rs::list_comparisons_returns_rows"
owner = "pm-backend"
due   = "2027-02-19"
ticket = "#3104"
reason = "list_comparisons SELECTs a column absent from the schema -> 500; needs a repo fix"
```

Adding a quarantine without an inline token therefore means adding a registry
row — so a new quarantine is still visible in the diff and can never be silent.

## Adding a quarantine

1. Write `#[ignore = "<reason>"]` with a real reason and a tracking id.
2. Give it an owner + deadline, inline or via a registry row. The deadline must
   be **no more than 90 days out** — a quarantine is a short-lived holding pen,
   not a graveyard.
3. Run the gate locally:
   ```bash
   python3 .github/workflows/scripts/check-quarantine-age.py
   ```

## Releasing (un-quarantining) a test

1. Remove the `#[ignore]` attribute; make the test green.
2. Delete its registry row (if it had one). A stale row — a registry entry with
   no matching `#[ignore]` — is reported as a `WARN` by the gate; keep the
   registry tidy.
3. Regenerate the census skeleton if you released a batch:
   ```bash
   python3 .github/workflows/scripts/check-quarantine-age.py --census
   ```

## When a deadline is about to pass

Prefer **fixing the test and deleting its entry** over bumping `due`. Bump the
deadline only with a fresh reason for why it still cannot land, and keep the new
deadline within 90 days. Repeated bumps on the same test are a signal to escalate
to the owner role rather than to quietly extend.

## Rollout note (2026-11-21)

The 201 tests quarantined at the time this policy landed were grandfathered into
the registry with a shared 90-day burn-down deadline (`2027-02-19`) and an owner
inferred from each test's reason (DB/schema/seed → `pm-backend`, Redis/docker →
`pm-devops`, MFA/rate-limit/TOCTOU → `pm-security`). Owners should re-triage
their rows before that date: fix and release, or set a justified new deadline.
