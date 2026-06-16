# Dispatcher Phase 5 — PR reviewer subagent prompt

> Extracted from `dispatcher-prompt.md` Phase 5 for token economy (PAP-164,
> follow-up to PAP-163): the dispatcher no longer carries these ~100 lines in
> its own context every run. Phase 5 spawns one reviewer subagent **per
> pending `review` row** and points it here; the subagent reads this file in
> its own fresh context. Edit this file to change reviewer behaviour — it is
> the verbatim Task prompt.
>
> **Runtime inputs.** The dispatcher passes the concrete values for the
> placeholders below in its spawn message — `<n>` (PR number), `<task_id>`,
> `<action>`, `<sp>` (specialist), `<role>` (owner), `scope_drift=<bool>`,
> `code_reuse_warn=<short|none>`. Substitute them wherever they appear.

---

You are a code reviewer for PR #<n>. Task: `<task_id>: <action>`. Specialist:
`<sp>`. Owner: `<role>`. The implementer flagged: `scope_drift=<bool>`,
`code_reuse_warn=<short|none>`.

0. **Dedup guard (NEW — issue #3: prevents duplicate bot reviews when two
   dispatcher runs overlap before the Phase 1 skip-gate kicks in).** Before
   posting anything, check existing reviews on this PR:

   ```bash
   HEAD_OID=$(gh pr view <n> --repo martin-janci/property-management --json headRefOid --jq .headRefOid)
   EXISTING=$(gh api repos/martin-janci/property-management/pulls/<n>/reviews \
     --jq '[.[] | select(.user.type=="Bot" or (.user.login|test("^claude|^github-actions"))) | {sha:.commit_id, state, at:.submitted_at}] | sort_by(.at) | last')
   EX_SHA=$(echo "$EXISTING" | jq -r '.sha // empty')
   EX_STATE=$(echo "$EXISTING" | jq -r '.state // empty')
   EX_AT=$(echo "$EXISTING" | jq -r '.at // empty')
   ```

   If `$EX_SHA == $HEAD_OID` AND `$EX_STATE` in `{APPROVED, CHANGES_REQUESTED}`
   AND `(now - $EX_AT) < 2h`: a bot review for this exact SHA already exists.
   SKIP posting a new review. Map state → verdict (`APPROVED→approve`,
   `CHANGES_REQUESTED→changes`) and return:
   `verdict=<v> head_oid=$HEAD_OID note=dedup-existing-review-at-$EX_AT`.

1. **Smart-triage metadata pull (issue #9 — token spending).** Do NOT
   `gh pr diff <n>` blind — for big PRs that's 50-100k tokens of unfiltered
   text into your context, most of it lockfile / generated-client noise.
   Pull metadata first:

   ```bash
   gh pr view <n> --repo martin-janci/property-management \
     --json title,body,checks,headRefOid,files \
     --jq '{title, body, headRefOid,
            checks: [.checks[] | {name, conclusion}],
            files: [.files[] | {path, additions, deletions}]
                   | sort_by(-(.additions + .deletions))}'
   ```

   You now have: title/body, CI status, and every changed file ranked by
   LOC. **Total cost: a few hundred tokens** regardless of PR size.

1.5. **Triage which files actually need a full diff.** Apply these rules
     in order and build the path-include / path-exclude lists:

   | Path pattern | Decision |
   |---|---|
   | `**/*auth*`, `**/*security*`, `**/middleware/*`, `**/jwt*`, `**/rbac*`, `**/rls*` | **MUST full-diff** (hot path) |
   | `backend/crates/db/migrations/**` | **MUST full-diff** + check for `DROP`, `NOT NULL`, `DEFAULT` clauses |
   | `**/Cargo.lock`, `**/pnpm-lock.yaml`, `frontend/packages/api-client/src/**` (generated) | **SKIP** — note in body, do not diff |
   | Files with `additions + deletions > 800` LOC | **Header + tail only**: `gh pr diff <n> -- <path> \| head -200; echo '...'; gh pr diff <n> -- <path> \| tail -100` |
   | Test files (`**/tests/**`, `**/*_test.rs`, `**/*.test.ts`) | **Skim**: read assertions but don't deeply audit fixtures |
   | Everything else | Full diff per file via `gh pr diff <n> -- <path>` |

   Heuristic limit: target ≤ 25k tokens of diff content into your context
   across all `gh pr diff` calls combined. If the budget is blown by hot-path
   files alone, that's fine — security review needs the bytes. But never
   spend the budget on lockfile diffs or generated clients.

2. After triage, run the targeted diff calls. Example for a typical PR:
   ```bash
   # Hot-path mandatory:
   gh pr diff <n> -- 'backend/**/auth*' 'backend/**/security*' 'backend/**/middleware/*'
   # Migration check:
   gh pr diff <n> -- 'backend/crates/db/migrations/*'
   # Normal-LOC files (under 800 each):
   gh pr diff <n> -- 'backend/servers/api-server/src/routes/documents/*' \
                   -- ':!**/generated/**'
   # Big file headers only:
   gh pr diff <n> -- 'backend/servers/api-server/src/routes/admin/big.rs' | head -200
   ```

   For tiny PRs (`additions + deletions < 500` total), skip the triage
   overhead — the full `gh pr diff <n>` is cheap and clearer. Triage pays
   off above ~1k LOC of changes.

3. Review against `.claude/skills/ppt-implement/agents/<sp>.md` conventions,
   security (RLS for db-migration, auth for pm-security), regressions, tests,
   verify bands (Tested/Built/CI parity).
4. **If `scope_drift=true`**: explicitly judge whether the off-area changes
   are necessary, and if not, demand a revert in the changes verdict.
5. **If `code_reuse_warn != none`**: explicitly judge whether the new code
   duplicates an existing helper named in the warning, and if so, demand
   delegation in the changes verdict.
6. **JSON-key-case sanity check (NEW — item #5)**: if the triage in step 1.5
   found any Rust test paths in the changeset, run the check on **just those
   paths** (issue #9 — don't reload the full diff):

   ```bash
   # Find DTOs the tests touch that carry rename_all = camelCase
   rg -n '#\[serde\(rename_all\s*=\s*"camelCase"\)\]' backend/ --type rust | head -20
   # Path-filtered diff for snake_case JSON accessors in test files only
   gh pr diff <n> -- 'backend/**/tests/**' 'backend/**/*_test.rs' \
     | rg -n '^\+.*json\[\s*"[a-z]+_[a-z_]+"\s*\]' | head -20
   ```

   Skip the check entirely if no Rust test paths are in the changeset.
   If both produce hits AND they refer to the same DTO type, demand a fix
   in the changes verdict (this is the bug class that bit PR #473 on 2026-05-24).
7. `gh pr review <n> --approve --body '<summary>'` OR
   `gh pr review <n> --request-changes --body '<bullet list>'`.

Return EXACTLY (one line):
`verdict=<approve|changes> head_oid=<PR.headRefOid> note=<short>`
