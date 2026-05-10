# Preflight — one-time setup on Windows

The agent runs in WSL but Playwright runs on Windows. Do this once per Windows machine before invoking the skill.

## 1. Install Node.js on Windows

Pick **one** of the following — do not mix.

### Option A: official MSI (simplest)

1. Open https://nodejs.org/en/download in a Windows browser.
2. Grab the LTS Windows Installer (`.msi`).
3. Run the installer with default options. It puts `node` and `npm` on the system PATH.

### Option B: winget

```powershell
winget install OpenJS.NodeJS.LTS
```

### Option C: nvm-windows (if you want multiple Node versions)

https://github.com/coreybutler/nvm-windows/releases — install `nvm-setup.exe`, then:

```powershell
nvm install lts
nvm use lts
```

> Do **not** install Node via Chocolatey if you already have nvm-windows — they fight over PATH.

## 2. Verify from WSL

```bash
/mnt/c/Windows/System32/cmd.exe /c "node -v && npm -v && where node"
```

You should see versions and a path like `C:\Program Files\nodejs\node.exe`. If `node` is "not recognized", reopen the WSL shell — Windows PATH is sampled at WSL session start.

## 3. Move (or clone) the project to /mnt/c/

`cmd.exe` cannot work from `\\wsl.localhost\...` (UNC paths). The project must be on a real drive letter.

```powershell
# In Windows PowerShell:
mkdir C:\Users\<You>\projects
cd C:\Users\<You>\projects
git clone <repo-url>
```

Then in WSL `cd /mnt/c/Users/<You>/projects/<repo>` for any session that uses this skill.

## 4. Install project deps + Playwright browsers — on Windows

```powershell
cd C:\Users\<You>\projects\<repo>
npm install              # or pnpm install / yarn — match the project
npx playwright install   # downloads chromium, firefox, webkit to %USERPROFILE%\AppData\Local\ms-playwright
```

If the project uses **pnpm** and you don't have it on Windows yet:

```powershell
npm install -g pnpm
```

## 5. (Optional) Install agent CLI

Only needed if you want `playwright-cli` (different from `npx playwright`):

```powershell
npm install -g @playwright/cli@latest
playwright-cli --help
playwright-cli install-browser
```

## 6. Install the WSL wrappers

Back in WSL:

```bash
~/projects/.../property-management/.claude/skills/playwright-windows/scripts/install.sh
```

This symlinks `pw-win` and `playwright-cli-win` into `~/bin/`. Add `~/bin` to PATH if not already.

## 7. Smoke test

```bash
cd /mnt/c/Users/<You>/projects/<repo>
pw-win --version
pw-win codegen https://example.com   # a real Chromium window should appear
```

If a window pops up: you're done. Close it; no test files needed for this check.

## What "looks wrong" but is actually fine

- First `pw-win` run is slow (~3–5 s) because cmd.exe + Node.js cold start.
- `npx playwright install` writes to `%USERPROFILE%\AppData\Local\ms-playwright`, not your project dir. That's correct.
- WSL's `~/.cache/ms-playwright` stays empty — those would be Linux browsers, which we deliberately aren't using.
