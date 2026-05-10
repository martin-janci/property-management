---
name: playwright-windows
description: Drive a real Windows Chrome/Edge/Firefox window from a WSL agent via Windows-side Node.js + Playwright. Use when the task needs a visible browser window, real clicking/typing, codegen, or headed debugging — not headless Linux. Triggers on phrases like "open the browser", "click through this page", "playwright codegen", "headed test", "visible browser", "real browser".
---

# playwright-windows skill

Run **Windows Playwright** from a WSL-based agent so the browser opens as a real Windows GUI app. Avoids WSLg flakiness and gives you a clickable, screenshot-able, debuggable window.

## When to use

Use this skill when:
- you need a **visible** browser window
- you need real clicking / typing / `codegen`
- you need to debug a flake with `--headed --pause`
- you need to test Windows-specific browser behavior

Do **NOT** use this skill for:
- headless CI runs (use Linux Playwright in WSL — faster, no interop overhead)
- token-cheap accessibility-tree introspection (use Playwright MCP server instead)

## Architecture

```
WSL2 (agent)              Windows (browser)
┌─────────────────┐       ┌──────────────────────────┐
│ Claude Code     │       │ Node.js + npm            │
│  cmd.exe /c ─── │──────▶│ npx playwright …         │
│  (absolute path)│       │   chromium.exe (visible) │
└─────────────────┘       └──────────────────────────┘
        │                         ▲
        ▼                         │
  /mnt/c/Users/<U>/projects/<P>  ─┘  (shared filesystem)
```

**Hard rule**: project must live on `/mnt/c/...` (Windows filesystem). UNC paths like `\\wsl.localhost\Ubuntu\...` are **rejected by cmd.exe** with "UNC paths are not supported" and silently default to `C:\Windows`.

## Preflight (agent: do this once per repo)

Before invoking any browser command, verify the environment. If anything fails, stop and direct the user to `references/preflight.md`.

```bash
# 1. cmd.exe interop available?
ls /mnt/c/Windows/System32/cmd.exe || { echo "Not WSL with C: mount — abort"; exit 1; }

# 2. Windows Node installed?
/mnt/c/Windows/System32/cmd.exe /c "node -v && npm -v" \
  || { echo "Install Node.js on Windows side first — see references/preflight.md"; exit 1; }

# 3. Project on /mnt/c/...?
case "$PWD" in
  /mnt/c/*) ;;
  *) echo "Project at $PWD — must be cloned under /mnt/c/Users/<you>/... for Windows Playwright"; exit 1 ;;
esac
```

If the project is currently on the WSL filesystem (`/home/...` or `\\wsl.localhost\...`), **do not silently clone elsewhere**. Tell the user the constraint and ask whether to:
- (a) clone the repo to `/mnt/c/Users/<them>/projects/<repo>` and continue there, or
- (b) fall back to Linux/headless Playwright inside WSL.

## How the agent should call Playwright

Two equivalent forms — pick one and stay consistent within a session.

### Form A: direct `cmd.exe` invocation (works without any setup)

```bash
WIN_DIR='C:\Users\Martin\projects\property-management'

/mnt/c/Windows/System32/cmd.exe /c \
  "cd /d ${WIN_DIR} && npx playwright codegen https://example.com"
```

Quote rules:
- single quotes around the bash variable so `\` is literal
- `cd /d` is required because `cmd.exe` won't change drives without it
- spaces in path → wrap the Windows path in `\"...\"` inside the cmd string

### Form B: wrapper scripts (after running `scripts/install.sh` once)

```bash
pw-win codegen https://example.com
pw-win test --headed
pw-win show-report
playwright-cli-win install-browser
```

Wrappers live in `scripts/`. They read `WIN_PROJECT_DIR` from env (or auto-derive from `$PWD` if it's under `/mnt/c/`) and forward args to Windows `npx playwright` / `playwright-cli`.

## Common commands

| Goal | Command |
|------|---------|
| Record a flow into test code | `pw-win codegen <url>` |
| Run all tests with visible browser | `pw-win test --headed` |
| Run a single test, paused on failure | `pw-win test path/to.spec.ts --headed --debug` |
| Open Playwright UI mode | `pw-win test --ui` |
| View last HTML report | `pw-win show-report` |
| View a trace.zip | `pw-win show-trace trace.zip` |
| Install browsers (Windows-side) | `pw-win install` |
| Agent-CLI help | `playwright-cli-win --help` |

## One-off scripts (no Playwright Test config)

For quick screenshot-and-verify flows on dev URLs, write a `.mjs` script under `/tmp/` on Windows:

```bash
# 1. Stage the script on Windows-side tmp
cat > "/mnt/c/Windows/Temp/check.mjs" <<'EOF'
import { chromium } from 'playwright';
const b = await chromium.launch({ headless: false });  // visible window
const p = await b.newPage();
await p.goto(process.argv[2]);
await p.screenshot({ path: 'C:\\Windows\\Temp\\shot.png', fullPage: true });
console.log('title:', await p.title());
await b.close();
EOF

# 2. Run it through Windows Node, against the project's node_modules
/mnt/c/Windows/System32/cmd.exe /c \
  "cd /d C:\\Users\\Martin\\projects\\property-management && node C:\\Windows\\Temp\\check.mjs https://wt-feature-design-integration.dev.rlt.sk/sk/sell"

# 3. Read the screenshot back from WSL
# (the agent uses Read tool on /mnt/c/Windows/Temp/shot.png — Read supports PNG)
```

## Hard rules — do not violate

1. **Never mix `node_modules`.** `npm install` must run on the same side (Windows) where you'll run Playwright. Linux-side install + Windows-side run = native binary mismatch and confusing errors.
2. **Never run `npx playwright` directly from WSL** when a Windows GUI is requested. WSLg + headed Playwright produces flaky black/empty windows on many setups; this skill exists specifically to avoid that path.
3. **Never assume `cmd.exe` is on PATH.** Probes show many WSL configurations have Windows-PATH-interop disabled. Always use `/mnt/c/Windows/System32/cmd.exe` absolutely.
4. **Never operate from a UNC path.** If `pwd` returns `/home/...` or anything not under `/mnt/c/...`, refuse and report.
5. **Never silently fall back** to Linux Playwright when Windows Playwright fails — the user asked for a real browser; fallback would defeat the purpose.

## Token / context efficiency

If the session will involve many sequential browser steps (>5 navigations, >10 clicks), strongly prefer the **Playwright MCP server** instead of this skill — accessibility-tree snapshots are dramatically cheaper than screenshot+OCR. This skill is best for:
- a single recording session via `codegen`
- a handful of headed verification steps
- visual debugging of a flake

## Files

| Path | Purpose |
|------|---------|
| `scripts/pw-win` | `npx playwright` wrapper |
| `scripts/playwright-cli-win` | `@playwright/cli` wrapper (agent CLI) |
| `scripts/install.sh` | symlinks both wrappers into `~/bin/` |
| `references/preflight.md` | one-time Node-on-Windows install checklist |
| `references/troubleshooting.md` | common failure modes and fixes |

## Decision policy (what to pick)

1. Visible browser / clicking / codegen → **this skill**.
2. Many small browser ops in one session → **Playwright MCP server** (different setup, not this skill).
3. CI / headless / pure assertion → **Linux Playwright in WSL** (`npx playwright test` directly, no wrapper).
4. Project lives only on WSL fs and user can't move it → **Linux Playwright** (and accept WSLg headed limitations) or **MCP**.
