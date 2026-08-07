# gh-issue-2699-migration-collision

**Vector:** bug
**Score:** 3
**Source:** Issue #2699 | PR #2692 (introduced the collision) | commit 8cf0fb5
**Confidence:** high

## Hypothesis
Two SQLx migrations at prefix `00220` are on `dev` right now (`00220_listing_syndication_view_counters.sql` from the 2026-08-06 dispatcher batch `cb91cac`, and `00220_portal_get_listing_view_count.sql` from #2692 merged 2026-08-07 00:08 `8cf0fb5`). The migration-collision CI gate fails for every branch based on current `dev`, so the whole backend `check`/`test`/`test-shard` set is red across the queue. Renaming the later-merged file to the next free prefix (`00227` — `00221`–`00226` are all used) is the mechanical fix; no code needs to change beyond string references to the migration filename.

## Evidence
- `git ls-tree -r origin/dev -- backend/crates/db/migrations/00220*` returns two blobs: `c74dd53e…` (`listing_syndication_view_counters`) and `f4d70cb6…` (`portal_get_listing_view_count`).
- CI failure on PR #2697's `check` job run `31134619480` (as cited in the issue body): `Migration version collision detected. Renumber the later-merged migration to the next free version prefix`.
- The later-merged of the two is #2692 (2026-08-07 00:08:27Z vs 2026-08-06 for the syndication file); renumbering the earlier-merged file would rewrite migration history that has already landed and been reasoned about, so the newer file moves.
- Free-prefix check on `dev`: `00221_create_layout_tables.sql`, `00222_support_tooling_events_retention.sql`, `00223_add_gdpr_consent_unique_constraint.sql`, `00224_create_oauth_token_events.sql`, `00225_create_layout_change_events.sql`, `00226_voice_assistant_devices_token_hash.sql` — next free is `00227`.

## Files
- `backend/crates/db/migrations/00220_portal_get_listing_view_count.sql`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector, but the root cause is already isolated — kept ticked for regression discipline)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode:** `Mode: cloud-ok` — mechanical file rename plus a `cargo sqlx prepare`; no browser, no device, no interactive tools needed.

Mode: cloud-ok

## Repro steps
1. Check out `origin/dev` and run the migration-collision CI job locally: `cd backend && ls crates/db/migrations/00220*` — observe two files at the same prefix.
2. Run any downstream job (`cargo check --workspace` in `backend/` is enough for the collision gate to fire in CI; the local sqlx offline data will already be consistent, so the CI-side collision detector is what surfaces the bug).
3. Expected after fix: only one `00220_*` file exists, `00227_portal_get_listing_view_count.sql` sits alone at the new prefix, and CI's migration-collision gate passes on any PR based on `dev`.

## Suggested approach
1. `cd backend && git mv crates/db/migrations/00220_portal_get_listing_view_count.sql crates/db/migrations/00227_portal_get_listing_view_count.sql` — no file body edit needed; the version prefix lives entirely in the filename.
2. Grep for any code, test, or doc reference to the old filename and update it: `rg -n '00220_portal_get_listing_view_count' backend/ .research/ docs/` — expect zero non-migration hits (the repo path in code refers to `portal_get_listing_view_count(UUID)` the SQL function, not the migration filename, so the code/tests introduced by #2692 do NOT need to change).
3. Regenerate SQLx offline data so the collision-free ordering is captured in the metadata bundle: `cd backend && cargo sqlx prepare --workspace -- --all-targets --all-features` (or the project's canonical wrapper — check `justfile`).
4. Local verification: `cd backend && cargo check --workspace` and the SQLx migrator sanity check (whichever the collision gate runs — inspect `.github/workflows/backend.yml` for the exact step name).
5. Commit as `fix(backend): renumber portal_get_listing_view_count migration 00220 → 00227 (collision with listing_syndication_view_counters)` — reference `Closes #2699` in the body.
6. Open a PR targeting `dev`, expect the backend `check`/`test`/`test-shard` jobs to go green as soon as they run against the renamed migration.

## Alternatives considered
- **Renumber the syndication file instead of the view-count file** — rejected because that file landed earlier (2026-08-06 vs 2026-08-07) and has already been consumed by downstream analytics rewrites; the issue body explicitly picks the later-merged file as the one to move, matching the repo's established "later collider yields" convention.
- **Squash both into a single 00220 migration** — rejected because the two migrations have unrelated purposes (portal view-count read function vs listing-syndication view-count tables) and merging them would obscure history and complicate rollback of either one independently.

## Root-cause trace
1. Symptom: every PR based on current `dev` fails the backend `check`/`test`/`test-shard` set with `Migration version collision detected. Renumber the later-merged migration to the next free version prefix`.
2. ← Immediate cause at `backend/crates/db/migrations/00220_portal_get_listing_view_count.sql:1` — this file's presence alongside `00220_listing_syndication_view_counters.sql` is what the CI collision detector rejects.
3. ← Upstream cause: PR #2692 authored its migration at prefix `00220` on a branch that pre-dated the syndication migration landing on `dev`; the numbering was picked when only one 00220 file was in scope, and no merge-time rebase re-picked a fresh prefix.
4. Origin: commit `8cf0fb5` (merge of PR #2692, 2026-08-07 00:08:27Z) — the moment the second `00220_*` file landed on `dev`.

## Test plan
- [ ] The CI migration-collision gate on `backend.yml` — this is the check that currently fails; a green run against the renamed migration file is the primary regression signal.
- [ ] `cd backend && ls crates/db/migrations/ | awk -F_ '{print $1}' | sort | uniq -d` — expect empty output (no duplicate prefixes).
- [ ] Sanity: `cd backend && cargo test -p db --test suite_3 portal_view_count_read_via_security_definer_under_force_rls` — the FORCE-RLS regression test introduced by #2692 must still find the renamed migration via SQLx's numeric ordering and pass.
- [ ] Local repro before push: `cd backend && cargo check --workspace` — no `sqlx-macros`/migrator complaints.

## Out of scope
- Any change to the migration's SQL body (the `portal_get_listing_view_count` function definition is correct as-is).
- Any change to code that calls the SQL function `portal_get_listing_view_count(UUID)` — the function name is independent of the migration filename.
- Adding a CI guard that pre-flights migration numbering on PR branches — worthwhile follow-up, but a separate concern that shouldn't ride this fix (opens as its own dx vector once this lands).

## After-merge
- Move this file to `plans/_archive/gh-issue-2699-migration-collision.md`
- Mark the matching `backlog.json` row as `status: "done"`
