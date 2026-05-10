# Troubleshooting

Symptoms grouped by root cause. If the symptom isn't here, run the **Preflight smoke test** in `preflight.md` first.

## "UNC paths are not supported. Defaulting to Windows directory."

You invoked `cmd.exe` while `$PWD` is on the WSL filesystem (`/home/...` or anything that resolves to `\\wsl.localhost\...`).

**Fix**: clone or copy the project under `/mnt/c/...` and `cd` there. The wrappers refuse to operate from non-`/mnt/c/` paths for exactly this reason.

## "'node' is not recognized as an internal or external command"

Windows Node.js isn't installed (or just got installed and the WSL session was opened *before* the install — Windows PATH is sampled at WSL session start).

**Fix**:
1. Install Node on Windows (`preflight.md` step 1).
2. Open a fresh WSL terminal (`wsl --shutdown` from PowerShell, then reopen) so the new Windows PATH is inherited.
3. Re-verify: `/mnt/c/Windows/System32/cmd.exe /c "where node"`.

## `cmd.exe: command not found` from WSL

WSL's Windows-PATH interop is disabled. This is fine — the wrappers always use the absolute path `/mnt/c/Windows/System32/cmd.exe`. If you want the bare `cmd.exe` to work too:

```bash
# /etc/wsl.conf — requires `wsl --shutdown` to reload
[interop]
enabled = true
appendWindowsPath = true
```

The skill does NOT require this. Don't change `/etc/wsl.conf` just for Playwright.

## Browser launches but immediately closes / black window

Common when running headed Playwright through **WSLg** (Linux Playwright + Linux browser binary, displayed via WSLg). This skill avoids that path entirely. If you still see it:
- you accidentally ran `npx playwright` from inside WSL (Linux binary)
- check: `which npx` returns `/mnt/c/...` for Windows mode, anything else means Linux mode

**Fix**: use `pw-win` or the explicit `cmd.exe /c "..."` form. Never bare `npx playwright`.

## `npm install` errors with native build failures (sharp, sqlite3, …)

You likely ran `npm install` once on WSL (Linux binaries downloaded), then again on Windows (or vice-versa). The two `node_modules` are not interchangeable.

**Fix**:
```powershell
# In Windows PowerShell, from the project:
rmdir /s /q node_modules
del package-lock.json   # or pnpm-lock.yaml etc.
npm install
```

Then never run `npm install` from WSL on this project again. If the project also has CI in Linux, treat that as a separate environment with its own clean install in CI — your local Windows clone shouldn't share the lockfile workspace anyway when you're using this skill.

## `where playwright-cli` returns nothing

`@playwright/cli` is a separate package from `playwright`. Install globally on Windows:

```powershell
npm install -g @playwright/cli@latest
```

This adds `playwright-cli.cmd` to `%APPDATA%\npm\` which should already be on PATH after a Node MSI install.

## Codegen window opens behind other apps

Win+Tab to find it, or run with `--browser=chromium --target=javascript --output=/path/to/out.js`. Codegen always brings the **Inspector** window forward, not the page; the page can hide behind your terminal.

## Tests pass headless but fail headed (or vice versa)

Real timing difference, not a skill problem. Use `pw-win test --headed --trace=on` and inspect the trace via `pw-win show-trace`. Common causes:
- visible browser is slower → `await page.waitForLoadState('networkidle')` masks/unmasks bugs
- focus / hover differs (headless has no real cursor)
- video recording disabled in headless

## Screenshots saved by Windows Playwright don't appear in WSL

You wrote them to a Windows path like `screenshots/foo.png` while running from `/mnt/c/Users/.../project`. They're there — same directory, same file. From WSL: `ls /mnt/c/Users/.../project/screenshots/`.

If you wrote to `C:\Windows\Temp\…` from the Windows-side script, read it from WSL via `/mnt/c/Windows/Temp/…`.

## Playwright browsers re-download every run

You're running the Linux Playwright (which can't find browsers in `~/.cache/ms-playwright`) thinking it's Windows Playwright. Run `pw-win --version` and check it prints the same version that's in your project's `package.json`. If it errors with "Executable doesn't exist", run `pw-win install`.

## Slow first command (3–5 seconds)

Cold-start of `cmd.exe` + `node.exe` over the WSL→Windows interop. Subsequent calls in the same session are fast because Windows caches the binaries. There is no fix; budget for it.
