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

## 2. The bash-command corpus (rtk discover, last 30 days)

- 481 sessions, **12,298 bash commands**. RTK already handles most of them, but **only 13
  (0.1%) were routed through RTK** in those recorded sessions.
- `rtk discover` estimates **~1.1M tokens saveable** from commands that RTK *already*
  supports but that ran raw: `grep -n` (1512×, ~336K tok), `git log` (1079×, ~225K),
  `gh pr` (601×, ~196K), `find` (350×, ~102K), `head -c` (390×, ~85K), `curl -s` (383×, ~57K),
  `ls -la` (383×, ~49K).
- Yet the global `~/.claude/settings.json` **does** configure `PreToolUse: Bash → rtk hook claude`,
  and `rtk gain` shows 22M tokens already saved overall. **Conclusion:** the rewrite hook is
  not firing in the agent/cloud heartbeat sessions (likely the runner image lacks the `rtk`
  binary or the hook, or those sessions use a settings scope without it). This is the single
  largest measured opportunity and the fix is environment-level, not code.

## 3. Optimizations ranked by (savings ÷ effort)

| # | Optimization | Est. savings | Effort | Status |
|---|--------------|-------------|--------|--------|
| 1 | **Fix RTK coverage in agent/cloud sessions** (ensure `rtk` + PreToolUse hook in runner) | ~1.1M tok / 30d (~37K/day fleet-wide) | Low–Med (config) | → child issue / escalate |
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

- **RTK coverage in agent/cloud runners** (#1) — investigate why the PreToolUse `rtk hook claude`
  isn't rewriting in heartbeat sessions; ensure the binary + hook ship in the runner. DevOps/Coder.
- **Dispatcher `backlog.json` + prompt slimming** (#5, #6) — query/paginate backlog reads;
  factor on-demand sections out of the 131 KB/76 KB prompts. Careful (behavior risk). Coder.
- **Phase 2 graphify telemetry gate** — instrument/measure symbol-grep frequency; revisit in 2–4 weeks.
