# Agent assignment: NIRUWEBAPP 0.2.0, a one-shot window identity, icon and UX update

## 0. Quick start

1. Read this file, `README.md`, `src/main.rs` and `tests/smoke.sh`. The audit baseline is
   `9d91ccf` (0.1.4, pinned by NIRUX 0.12.199). Revalidate each item against current HEAD
   and skip items that are already fixed, noting that you did.
2. Implement **B1–B5, then U1–U4 in order**. Commit per item. Keep fmt, clippy
   `-D warnings`, `cargo test --locked` and `tests/smoke.sh` green.
3. Pass the gates in §4, release in §5 and report in §6.

This is an **execution** assignment. Do not stop after a plan or a branch. Make sensible
decisions, document them, and continue without routine questions.

## 1. Invariants

- Use an already installed Chromium-family browser in app mode: no Electron, no WebView
  and no background service. Never install a browser.
- Keep the profile modes: isolated (default) and shared. Switching is non-destructive,
  and removal keeps the profile unless `--delete-profile` is given.
- Existing webapps in `~/.local/state/niruwebapp` and their desktop entries keep
  working after the upgrade. Rewrite the entries only through the same code path, and
  leave the user's data untouched.
- Keep `rust-version = 1.85`.
- **The owner has 7 webapps** (ChatGPT, Claude, Copilot, Gemini, Grok, Outlook, Teams),
  several of them with a shared profile for Microsoft 365. Never close or restart the
  owner's browser or webapp windows during testing. Use a temporary HOME and a test
  browser profile.

## 2. Findings (verified 2026-11-04 on niruxwork, live windows inspected read-only)

### B1: Shared-profile webapps get the wrong window class (high)
`launch()` passes `--class=niruwebapp-<id>`, and the desktop entry declares
`StartupWMClass=niruwebapp-<id>`. Chromium honors `--class` only when it starts a new
browser process. When the shared profile's browser is already running, the window joins
that process. Live on niruxwork:

| Webapp | Actual WM_CLASS |
|---|---|
| claude (isolated) | `claude.ai`, `niruwebapp-claude` ✓ |
| chatgpt, gemini, teams, outlook (shared) | `chatgpt.com`/`gemini.google.com`/`teams.cloud.microsoft`/`outlook.cloud.microsoft__mail`, **`Helium`** ✗ |

As a result, launcher window switching, desktop-entry matching, WM rules and taskbar or
icon association do not work for shared webapps.
**Fix:**
- For shared-profile apps, write `StartupWMClass` as the instance name Chromium actually
  uses for `--app` windows: the host plus path, with `/` mapped to `__`, as observed
  above. Compute it with a tested function and verify it against a real Helium and
  Chromium.
- For isolated apps keep `niruwebapp-<id>`.
- Add `niruwebapp doctor`, which lists every webapp, its expected class, and whether a
  matching window is open now (read-only `_NET_CLIENT_LIST`/`WM_CLASS` through x11rb or
  `xprop`).
- When switching profile mode, rewrite the entry.

### B2: A second launch opens a duplicate window
`niruwebapp launch <id>` always spawns the browser. Clicking Teams twice gives two Teams
windows. **Fix:** before spawning, look for an existing window with the expected class
(B1). If one exists, activate it (`_NET_ACTIVE_WINDOW` client message) and exit. Add
`--new-window` to force a new one.

### B3: Every webapp gets the same generic icon
All 7 entries use `Icon=web-browser`, so the launcher and taskbar show identical icons.
**Fix:**
- By default, generate a local themed letter icon (first letter, NIRUX accent color, PNG
  at 128 px with `ab_glyph`/`png`, or a simple SVG) at create time and for existing apps
  on upgrade.
- Add `--fetch-icon`, an **explicit opt-in**: one HTTPS request for the site's
  `apple-touch-icon` or `favicon` with a 5 s timeout, a 512 KB limit and image validation.
  Never fetch automatically.
- `set-icon PATH` still overrides both.

### B4: Desktop entry escaping
`desktop()` writes `Name` with only `\n`/`\r` replaced. The quoting in `desktop_exec()`
handles spaces but not the characters the Desktop Entry spec reserves inside quotes
(`"`, `` ` ``, `$`, `\`), and `%` must be doubled in `Exec`. **Fix:** implement the
spec's escaping for `Exec` arguments and the string escaping for `Name`/`Comment`. Test
it with names and paths that contain `%`, `$`, quotes and non-ASCII characters.

### B5: Comment and category
`Comment=Webapp` is the same for every entry. Use the host name instead ("Webapp ·
teams.cloud.microsoft"), so launchers can tell apps apart and search by domain.
Keep `Categories`.

## 3. UX

### U1: Management in the launcher
Check whether NIRULAUNCHER lists webapps through their desktop entries only. Add to
NIRUWEBAPP a JSON `list --json` (id, name, url, profile mode, icon, expected class,
running), so that a launcher view can manage them later. Do not change nirulauncher in
this assignment. Mention the follow-up in the report.

### U2: `create` with good defaults
If the name is omitted, derive it from the host (`teams.cloud.microsoft` → "Teams").
Recognize Microsoft 365 hosts and suggest `--shared-profile` in the output, with the
README's cookie note in one line. Never switch the mode silently.

### U3: Safer remove
`remove` without `--yes` prints what will be removed (desktop entry, icon, and the
profile only with `--delete-profile`, including its size) and asks for confirmation when
it runs on a TTY. Non-interactive runs require `--yes`.

### U4: Browser selection that tells the user something
When no supported browser exists, list the names that were searched and how to point at
one with `--browser`. When the stored browser path has disappeared, for example after a
Helium update changed the path, fall back to detection and update the record, with a
notice.

Out of scope: an embedded WebView, extensions, sync and a GUI window.

## 4. Gates

Automated:
- fmt, clippy, tests, `tests/smoke.sh`, and an MSRV job on 1.85 (if it is missing).
- New tests: B1 class derivation (a table of URLs including paths, ports and IDN), B4
  escaping, the B3 letter icon output (size and valid PNG), the U2 name derivation, and
  the U3 dry-run listing.
- A smoke test with a fake browser on PATH that records argv, and a fake X environment
  for B2 (Xvfb plus a dummy window with the expected class).

Native, on **niruxwork** (the owner allows local testing):
- Use a temporary HOME and a test webapp (`https://example.com`) with isolated and shared
  profiles against a **separate** browser user-data-dir. Do not use the owner's running
  Helium profile for the shared test.
- Verify the B1 classes with `xprop`, B2 activation, B3 icons in nirulauncher, and
  `doctor`.
- Run the upgrade path against a **copy** of the owner's `~/.local/state/niruwebapp` in
  the temporary HOME, and confirm that the 7 entries are regenerated correctly. Leave the
  real ones untouched until the owner installs the release through NIRUX.

## 5. Release

Bump to **0.2.0** and update the README: classes, single-window behavior, icons, doctor
and remove. Tag `v0.2.0` through the normal workflow and verify the assets. Then pin it in
`nirucon/nirux` `manifest`:
- If `nirux/tools/suite-pin` exists, use it.
- Otherwise bump the manifest the usual way. Change only the niruwebapp line, and rebase
  if other pins land at the same time.

After `nirux update`, run `niruwebapp doctor` on niruxwork and include the output, with
URLs shortened to hosts.

## 6. Final report

For each item B1–B5 and U1–U4: the change, the tests and the evidence (the class table
before and after, doctor output, icon screenshots). Include release URLs, installed
versions, the nirulauncher follow-up and remaining limits. Mark anything unfinished as
**PARTIAL** and give the exact remaining steps.
