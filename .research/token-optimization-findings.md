# Token-Cost Optimization — Findings & Recommendations (PAP-161)

Parent: PAP-160 (board ask: "Improve consumption of tokens… for instance cached graphify").
Author: CTO. Date: 2026-06-11. Method: measured the actual bytes loaded into agent context
and the bash-command corpus over the last 30 days (`rtk gain` / `rtk discover`, `wc`, repo inspection).

Token estimates use ~4 bytes/token (markdown/English). Treat as order-of-magnitude, not exact.

---

## 1. What loads on a cold heartbeat (measured)

Repo- or memory-controlled context auto-loaded **every** agent heartbeat:

| Source | Size | ~Tokens | Loaded | Controllable? |
|--------|------|--------:|--------|---------------|
| Project `CLAUDE.md` | 17.7 KB (410 ln) | ~4,440 | every heartbeat | ✅ repo |
| `MEMORY.md` (auto-memory index) | 9.5 KB (35 ln) | ~2,365 | every heartbeat | ⚠️ `~/.claude`, not repo |
| Global `~/.claude/CLAUDE.md` + `RTK.md` | ~3.3 KB | ~830 | every heartbeat | ⚠️ user-global |
| `docs/CLAUDE.md` | 13.6 KB (467 ln) | ~3,400 | **on demand** (indexed) | ✅ repo |
| Skill/tool manifests | large | several K | every heartbeat | ❌ harness |

Loaded only on **research-dispatcher / routine** runs (frequent, but not every agent):

| Source | Size | ~Tokens |
|--------|------|--------:|
| `.research/dispatcher-prompt.md` | 131 KB (2413 ln) | ~32,800 |
| `.research/backlog.json` | 143 KB | ~35,700 |
| `.research/routine-prompt.md` | 76.5 KB | ~19,100 |
| `.research/implementer-prompt.md` | 19.7 KB | ~4,900 |

## 2. The bash-command corpus — CORRECTED (PAP-162 investigation, 2026-06-11)

> The original §2 read the `rtk discover` "0.1% routed through RTK / ~1.1M saveable" number
> as a real coverage gap and concluded *"the rewrite hook is not firing in agent/cloud
> heartbeat sessions."* **PAP-162 disproved that conclusion with end-to-end evidence.**
> Corrected findings below; the original figures are retained for context but are a
> measurement artifact, not a coverage gap.

**What `rtk discover` reported:** 481 sessions, 12,298 bash commands, **only 13 (0.1%) "routed
through RTK"**, **~1.1M tokens "saveable"** from `grep -n`/`git log`/`gh pr`/`find`/`head -c`/
`curl -s`/`ls -la` that "ran raw."

**What is actually true (measured in a live agent heartbeat session):**

1. **The hook IS installed and IS firing.** `rtk` is on PATH (`/home/dev/.local/bin/rtk`,
   installed 2026-06-05) and `~/.claude/settings.json` has `PreToolUse: Bash → rtk hook claude`.
   The hook correctly rewrites bare *and* compound/piped commands
   (`cd … && git status … | head` → `cd … && rtk git status … | head`).
2. **The rewrite is honored end-to-end and rtk executes.** Running `git log` / `grep` through
   the Bash tool in an agent session increments `rtk gain`'s exec counter in real time, and
   `rtk gain -F` logs 1,184 fallbacks at a **99.6% recovery rate** — i.e. the hook invokes rtk
   even on commands rtk can't optimize (grep on `/tmp` files, `ps aux`, `rsync`, `wget`).
3. **`rtk discover`'s 0.1% is a measurement artifact, not coverage.** Claude Code records the
   model's **pre-hook** command in the transcript JSONL. `rtk discover` parses transcripts, so
   it can *never* observe a `PreToolUse` rewrite. Proof: `git log --oneline -5` ran through rtk
   (gain counter incremented) yet is recorded in the transcript as raw `git log --oneline -5`,
   no `rtk` prefix. Therefore discover reports ~0% **regardless of true coverage** — and the
   number cannot "jump above 0.1%" by any runner fix.
4. **The ~1.1M estimate is inflated** on two counts: (a) the 30-day window mostly predates
   rtk's 2026-06-05 install (rtk's telemetry only has 2026-06-09→11 data), and (b) discover's
   "MISSED SAVINGS" double-counts commands that *already* routed through rtk (recorded raw) plus
   fallback commands rtk genuinely can't compress.

**True coverage signal = `rtk gain` (rtk's own exec log, not transcripts):** 22M tokens saved,
78.4% efficiency, 99.6% recovery, ~5.3K execs over 3 active days. **The hook works as designed;
there is no runner-config bug and no ~1.1M to recover by "fixing the hook."**

**Genuine (small) residual:** commands rtk does not yet wrap — `rbuild` (cargo/pnpm offload,
~349×/6d), `ssh` (74×), heredoc `cat`, `until grep` loops. These are *upstream rtk feature
requests*, not an environment fix. One unverified unknown: if any agents run on **separate**
cloud-runner infra (different host than this `/home/dev` box), rtk presence there is unconfirmed
— but every agent heartbeat observable from this host has rtk working.

## 3. Optimizations ranked by (savings ÷ effort)

| # | Optimization | Est. savings | Effort | Status |
|---|--------------|-------------|--------|--------|
| 1 | ~~**Fix RTK coverage in agent/cloud sessions**~~ — **RESOLVED, NO FIX NEEDED** (PAP-162): hook fires + rtk executes; the "0.1% / ~1.1M" is a `rtk discover` measurement artifact (transcripts store pre-hook commands). See §2. | ~0 recoverable (already saving 22M via `rtk gain`) | n/a | ✅ closed — investigated PAP-162 |
| 2 | **Trim `CLAUDE.md`** — move release/hotfix/versioning detail to `docs/git-workflow.md` | ~1,073 tok × every heartbeat | Low | ✅ landed this PR |
| 3 | **Cached repo map** `docs/repo-map.md` (graphify Phase 1) — read instead of grep fan-out | ~2–10K tok per avoided exploration; recurring | Low | ✅ landed this PR |
| 4 | **Search/read discipline rule** in `CLAUDE.md` (Explore fan-out, read slices, repo-map first) | indirect, compounding | Low | ✅ landed this PR |
| 5 | **Slim `backlog.json` reads** in dispatcher (query/paginate, don't load 143 KB whole) | ~up to 35K tok per dispatcher run | Med | → child issue |
| 6 | **Trim dispatcher/routine prompts** (131 KB + 76 KB) — factor shared boilerplate, on-demand sections | ~10–20K tok per run | Med–High (behavior risk) | → child issue |
| 7 | **Tighten `MEMORY.md`** index lines (verbose one-liners → terse hooks) | ~1K tok/heartbeat | Low | guidance (lives in `~/.claude`, not repo) |
| 8 | **Prompt-cache hygiene** — avoid sub-300s `ScheduleWakeup`/poll waits that bust the 5-min cache; keep dynamic content out of early context | avoids full-context cache-miss re-reads | Low | codified in #4 + memory |

## 4. "Graphify" / cached repo-graph recommendation — **PHASED (do Phase 1 now)**

The board asked specifically about a "cached graphify". Recommendation:

- **Phase 1 — DO NOW (landed this PR): a coarse committed `docs/repo-map.md`.** The dominant
  agent cost is *locating* a feature (where does `faults` live?), not needing a full symbol
  graph. This repo has a strong vertical-slice naming convention (`routes/<noun>.rs` ↔
  `repositories/<noun>.rs` ↔ migrations ↔ typespec). A one-page map of crates/apps/routes +
  that convention lets an agent resolve location by reading one file instead of grepping
  84 route + 101 repo files. Low effort, low staleness (coarse), no build infra.
- **Phase 2 — EVALUATE, don't build yet: a generated symbol index** (universal-ctags or
  tree-sitter, regenerated in CI, committed or artifact). Worth it **only if** post-Phase-1
  telemetry shows agents still grep heavily for *symbols* (not files). Costs: staleness,
  a CI regen step, and a multi-hundred-KB index that itself costs tokens to read — so it must
  be queried, never loaded whole. Decision gate: revisit after 2–4 weeks of Phase 1.
- **Phase 3 — NOT WORTH IT: a live code-graph DB / service** (e.g. a running indexer agents
  query over RPC). Cost-to-operate (a service to run, secure, and keep fresh) exceeds the
  gain for a team this size. Reject unless the org scales many-fold. **Escalate to Atlas only
  if Phase 2 telemetry justifies Phase 3.**

Evidence for Phase-1-first: `rtk discover` shows `grep -n` (1512×) and `find` (350×) dominate —
both are *file/location* searches the repo-map directly displaces, whereas symbol-level grep is
a smaller slice.

## 5. What landed in this PR (acceptance criteria)

- ✅ **Opt #2** — `CLAUDE.md` trimmed 410→285 lines (~1,073 tok/heartbeat saved); detail moved
  to `docs/git-workflow.md`.
- ✅ **Opt #3** — `docs/repo-map.md` added (graphify Phase 1, measurable grep-avoidance).
- ✅ **Opt #4** — search/read-discipline rule added to `CLAUDE.md`.
- ✅ Graphify question answered with a phased recommendation + evidence (this doc, §4).
- ✅ Findings doc committed (this file) and linked on PAP-161.

## 6. Delegated follow-ups (child issues)

- ~~**RTK coverage in agent/cloud runners** (#1)~~ — **DONE (PAP-162):** the hook *is* firing and
  rtk *is* executing in heartbeat sessions; the "not rewriting" symptom was a `rtk discover`
  measurement artifact (it reads transcripts, which store the pre-hook command). No runner fix
  needed. Residual = upstream rtk feature requests for `rbuild`/`ssh`/heredocs. See §2.
- **Dispatcher `backlog.json` + prompt slimming** (#5, #6) — query/paginate backlog reads;
  factor on-demand sections out of the 131 KB/76 KB prompts. Careful (behavior risk). Coder.
- **Phase 2 graphify telemetry gate** — instrument/measure symbol-grep frequency; revisit in 2–4 weeks.
