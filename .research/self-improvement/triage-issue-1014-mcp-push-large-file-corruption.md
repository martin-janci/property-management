# Triage — issue #1014: `action-list.json` corruption when MCP push falls back from a blocked `git push`

- **Task:** `triage-dispatcher-mcp-push-large-file-issue-1014`
- **Owner role:** pm-tech-lead
- **Priority:** low (investigation)
- **Issue:** [#1014](https://github.com/martin-janci/property-management/issues/1014) — *"[dispatcher] action-list.json corrupted on dev — restore from commit 704ea44"* (closed/completed 2026-06-06; recovery of the one corrupted snapshot was done. This triage covers whether the **systemic** fix is actually in force.)

## TL;DR

The systemic fix for #1014 was **authored but never wired in**. Two purpose-built scripts
exist — `action-list-reconcile.sh` (shrink the file) and `mcp-push-size-guard.sh`
(fail-closed backstop) — but **neither is invoked by the dispatcher's Phase 6 flow**.
As a result the corruption vector is still live today:

- `.research/management/action-list.json` is **94,851 B** right now — **above** the
  guard's 65,536 B (64 KiB) inline-push ceiling, and ~1.4× the size that corrupted on
  2026-06-03.
- **102 of 118** items in it are terminal (`done`=76, `dropped`=26) — exactly the
  accretion the issue predicted. Only 16 are live (`open`=8, `in-progress`=8).
- If a Phase 6 `git push` is HTTP-403'd today and falls back to `mcp__github__push_files`,
  the 92 KB literal truncates on emission and lands on `dev` as a 1–2 item stub — the
  identical #1014 corruption, **unguarded**.

A third, secondary fidelity defect is also visible in the tree: the two #1014 scripts
were committed via the MCP push path and **lost their `100755` exec bit** (landed
`100644`), the mode-loss half of the same bug class.

## The bug class (three related failure modes)

The dispatcher's Phase 6 "push contract" assumes `mcp__github__push_files` can land any
`.research/management/**` file. It cannot, for two independent fidelity reasons, and the
one mitigation that would keep files inside the safe envelope is not running:

### 1. Large-file truncation (the primary #1014 corruption) — UNGUARDED

- **Where:** `.research/dispatcher-prompt.md:2049-2058` (Phase 6 push). `PUSH_METHOD`
  defaults to `mcp`; the MCP branch (`:2055-2058`) lands the delta via
  `mcp__github__push_files` **with no size check first**.
- **Mechanism:** `push_files` requires the full file **content inline** in the tool call.
  A ~75–92 KB literal cannot be emitted reliably and is silently truncated, so the file is
  written to `dev` as a tiny stub. Unlike a blocked `git push` (recoverable — the next run
  retries on a corrected base), a truncated MCP push is **committed corruption** and is not
  self-healing.
- **Why it's live:** the file has grown back to 94,851 B (see evidence below).

### 2. Exec-bit / mode loss — PRESENT IN TREE

- **Where:** `git ls-files -s .research/*.sh` shows `.research/action-list-reconcile.sh`
  and `.research/mcp-push-size-guard.sh` recorded as **`100644`**, while every sibling
  reconciler (`archive-reconcile.sh`, `gc1-reconcile.sh`, `backlog-refill.sh`,
  `goal-check.sh`, `issue-ingest.sh`, …) is **`100755`**.
- **Mechanism:** `mcp__github__push_files` does not preserve the Unix executable bit and
  cannot restore mode `100755`; the two scripts most recently added through that path
  landed non-executable.
- **Impact today:** low but real. The dispatcher invokes reconcilers as `bash .research/x.sh`,
  which ignores the mode, so it still runs. But both scripts document `./.research/<script>.sh`
  invocation, and any consumer (a human, CI, or a future `find -perm` sweep) that relies on
  the exec bit is broken. This is the mode-fidelity signature of the same push path.

### 3. Partial-write / no-atomicity across a multi-file push — LATENT

- `push_files` writes N files as one API commit; the guard and the reconciler both aim to
  keep each file small enough that emission does not truncate mid-file. There is no
  post-push content verification of the *non-management* files, but Phase 6 does verify the
  `.research/management/` tree (`dispatcher-prompt.md:2093-2100`). This angle is adequately
  covered once (1) and (2) are addressed; noted for completeness.

## Concrete evidence (this worktree, base `origin/dev` @ `4e0603e`)

```
# action-list.json is oversize and terminal-heavy
$ wc -c .research/management/action-list.json     -> 94851
$ jq '.items|length'                              -> 118
$ jq '.items|group_by(.status)|map({(.[0].status):length})|add'
  { "done":76, "dropped":26, "in-progress":8, "open":8 }   # 102/118 terminal

# The size guard already flags it (but nothing calls the guard):
$ PUSH_METHOD=mcp bash .research/mcp-push-size-guard.sh .research/management/action-list.json
  OVERSIZE .research/management/action-list.json = 94851B > 65536B ceiling   (exit 3)

# The reconciler would fix it (dry-run — writes nothing):
$ bash .research/action-list-reconcile.sh
  action-list-reconcile: 102 terminal item(s) in active action-list.json   # -> would leave ~16 live

# Neither script is referenced by the Phase 6 flow:
$ grep -rn 'mcp-push-size-guard' .research/*.md .research/dispatcher-self-test.sh   -> (no hits)
$ grep -n  'action-list-reconcile' .research/dispatcher-prompt.md
  885: ...the same corruption vector `action-list-reconcile.sh` guards   # a passing mention, NOT an invocation

# Exec-bit loss from the MCP push path:
$ git ls-files -s .research/action-list-reconcile.sh .research/mcp-push-size-guard.sh
  100644 ... action-list-reconcile.sh      # should be 100755
  100644 ... mcp-push-size-guard.sh        # should be 100755
```

### The wiring gap, precisely

- `mcp-push-size-guard.sh` is **dead code**: it appears nowhere outside its own file. The
  Phase 6 MCP branch (`dispatcher-prompt.md:2055`) pushes without ever calling it.
- `action-list-reconcile.sh --apply` is **never executed** by the dispatcher. The only
  references are:
  - `dispatcher-prompt.md:885` — a prose aside inside the Tier-1b backlog-refill note.
  - `dispatcher-self-test.sh:622-628` (test **T26**) — an **advisory `warn`**, not a
    `fail`, that merely prints *"run: bash .research/action-list-reconcile.sh --apply"*.
- Contrast `archive-reconcile.sh`, whose assignments-side equivalent **is** run every
  Phase 6 — which is why `assignments.json` stays at 5,549 B while `action-list.json`
  bloated to 94,851 B.

Net: the belt (reconcile to stay small) and the suspenders (fail-closed size guard) both
exist in the repo but neither is fastened, so the file grew back past the ceiling and the
push path has no guard to stop a corrupting emission.

## Smallest recommended fixes (named at file:line)

Ranked; all are for a **human reviewer** — this triage PR only lands #3 (the safe part),
because #1 and #2 modify Phase-6 control flow, which is out of scope for an auto-triage.

1. **Fasten the belt — run the reconciler every Phase 6.** Add, in the Phase 6 pre-commit
   block just before the `git add` at `.research/dispatcher-prompt.md:1997`, an
   idempotent sweep:
   ```bash
   bash .research/action-list-reconcile.sh --apply   # move terminal action-list rows to archive (#1014)
   ```
   and stage `action-list.json` + `action-list-archive.json` when it moved anything. This
   is the root-cause fix: it keeps the live file at ~16 rows (well under 64 KiB), the same
   discipline `archive-reconcile.sh` already applies to `assignments.json`. Idempotent and
   count-guarded, so it fails closed.

2. **Fasten the suspenders — call the size guard before the MCP push.** In the
   `PUSH_METHOD=mcp` branch at `.research/dispatcher-prompt.md:2055-2058`, before emitting
   the `push_files` call:
   ```bash
   PUSH_METHOD=mcp bash .research/mcp-push-size-guard.sh --staged \
     || { echo "PHASE 6 ABORT: oversize file(s) — not MCP-pushing (issue #1014); next run retries on a shrunk base." >&2; exit 0; }
   ```
   Fail-closed: a blocked push is recoverable; a truncated one is not. (Optional hardening:
   promote self-test **T26** from `warn` to `fail` at `dispatcher-self-test.sh:628` so an
   oversize file trips the existing pre-commit self-test gate at `dispatcher-prompt.md:2027`.
   Do this only together with fix #1, or the gate will block every run until the file is
   shrunk once.)

3. **Restore the exec bit on the two #1014 scripts** — **applied in this PR.**
   `git update-index --chmod=+x .research/action-list-reconcile.sh .research/mcp-push-size-guard.sh`
   brings them back to `100755`, matching their siblings. Landed via plain `git push`
   (which preserves modes) precisely because the MCP path is what stripped them. Zero
   runtime risk — making a documented-as-`./`-invoked script executable cannot break the
   `bash <script>` callers.

4. **(Operator, environment)** Provision a `GITHUB_TOKEN` so a `curl --data @file` to the
   Contents API (payload read from disk, no inline limit) is available as the large-file
   push path — the durable escape hatch called out in the issue's "Systemic fix" §1. Out
   of code scope; tracked here for the operator.

## What this PR changes

- **Adds** this analysis file.
- **Restores** `100755` on `.research/action-list-reconcile.sh` and
  `.research/mcp-push-size-guard.sh` (fix #3 above).

Deliberately **not** changed (out of scope for auto-triage; left for human review):
Phase-6 control flow in `dispatcher-prompt.md`, the self-test gate, and every
`.research/management/*.json` data file (including `action-list.json`, which is still
oversize — fix #1 is the intended remediation and should run as part of a normal
dispatcher cycle, not a manual data edit here).
