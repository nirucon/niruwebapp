# NIRUWEBAPP

**Part of [NIRUX - A minimal X11 desktop by Nicklas Rudolfsson](https://github.com/nirucon/nirux).**

**Early beta.** A small webapp manager for the NIRU desktop. It uses an already
installed Helium/Chromium in app mode; there is no Electron, WebView runtime,
extra browser or background service.

## Install

NIRUX installs it automatically. Standalone:

```sh
git clone https://github.com/nirucon/niruwebapp
cd niruwebapp
./install.sh
./install.sh --build
```

## Use

```sh
niruwebapp create "YouTube Music" https://music.youtube.com
niruwebapp create https://teams.cloud.microsoft/ --shared-profile   # name: Teams
niruwebapp list
niruwebapp list --json
niruwebapp launch "YouTube Music"
niruwebapp launch teams --new-window
niruwebapp edit youtube-music --name "YT Music"
niruwebapp edit youtube-music --shared-profile
niruwebapp edit youtube-music --isolated-profile
niruwebapp edit youtube-music --fetch-icon
niruwebapp icon youtube-music ~/Pictures/yt.png
niruwebapp doctor
niruwebapp refresh
niruwebapp remove youtube-music --dry-run
niruwebapp remove youtube-music --delete-profile --yes
```

Without a name, `create` names the webapp after its host
(`teams.cloud.microsoft` becomes "Teams"). For Microsoft 365 hosts it
suggests `--shared-profile`; it never switches the mode by itself.

Each webapp gets an ordinary desktop entry under
`~/.local/share/applications/` and a dedicated browser profile under
`~/.local/share/niruwebapp/profiles/`. Use `--shared-profile` to use the
browser's ordinary profile instead. An existing webapp can be switched between
the two modes with `edit --shared-profile` or `edit --isolated-profile`; the old
profile is kept on disk so switching back is non-destructive. Restart the
webapp after switching. Removing the launcher keeps the profile unless
`--delete-profile` is explicit.

A shared profile is useful for Microsoft 365 and other sites where sign-in
should follow the ordinary browser session. The trade-off is that cookies and
sessions are shared with normal browsing.

### Microsoft 365 sign-in

Teams uses a third-party authentication frame. If the browser blocks
third-party cookies, Teams can show “We need you to sign in again” and its
Sign in button may do nothing. Keep the global setting on “Block third-party
cookies” and add these sites to “Sites allowed to use third-party cookies”:

- `[*.]cloud.microsoft`
- `[*.]microsoft.com`
- `[*.]microsoftonline.com`
- `[*.]office.com`
- `[*.]live.com`
- `[*.]sharepoint.com`
- `[*.]onedrive.com`

The exceptions live in the browser profile, so they apply to both normal
browsing and every webapp that uses the shared profile.

### Windows and window classes

Launching a webapp that already has a window activates that window
(`_NET_ACTIVE_WINDOW`) instead of opening a second one; a second click while
the first window is still starting is ignored. `--new-window` always opens a
new window.

Desktop entries declare the window class the webapp's windows really get, so
launchers, taskbars and window rules can match them:

| Profile | `StartupWMClass` | Example WM_CLASS |
|---|---|---|
| isolated | `niruwebapp-<id>` | `claude.ai`, `niruwebapp-claude` |
| shared | host and path, `/` as `__` | `outlook.cloud.microsoft__mail`, `Helium` |

A shared-profile webapp usually joins the browser that is already running, and
Chromium then ignores `--class`; the instance name derived from the URL is the
only stable identity. International host names are used as punycode.

`niruwebapp doctor` lists every webapp, its expected class, how many matching
windows are open, and whether its desktop entry and browser are current. It
only reads. `niruwebapp list --json` gives the same data (id, name, url,
profile, icon, expected class, running) for launcher views.

### Icons

Every webapp gets its own icon: a letter on a rounded square in the NIRUX
accent colour of the selected theme. When two webapps start with the same
letter, they get two letters (Ch, Cl, Co). `niruwebapp refresh` redraws the
letter icons, for example after a theme change.

`--fetch-icon` (with `create` or `edit`) is an explicit opt-in: it asks the
site over HTTPS for `/apple-touch-icon.png`, then `/favicon.ico`, with a 5 s
timeout and a 512 KB limit, and only accepts a valid PNG (also inside an
ICO). NIRUWEBAPP never contacts a site otherwise. `--icon PATH` or
`niruwebapp icon ID PATH` sets your own icon and overrides both.

### Removing

`remove` prints what it will delete: the desktop entry, the icon, and the
isolated profile with its size only with `--delete-profile`. It asks for
confirmation on a terminal; scripts must pass `--yes`. `--dry-run` only
prints the list. A shared profile is never touched.

### Upgrading from 0.1

The first command after the upgrade backs up `apps.tsv` and every desktop
entry to `~/.local/share/niruwebapp/backup-0.1/` (once; never overwritten),
gives webapps without an icon a letter icon, keeps your own icons, and
rewrites the entries with the new class, comment and escaping. Profiles and
sign-ins are not changed.

### Browser

`--browser PATH` selects a backend; otherwise Helium, Chromium, Chrome or Brave
is detected in `~/.local/bin` and on `PATH` (or `NIRUWEBAPP_BROWSER`). No
browser is installed automatically. If a webapp's browser path disappears,
for example after a Helium update, the next launch detects the browser again
and updates the record.

## Build

```sh
cargo test --locked
cargo build --release --locked --bins --examples
sh tests/smoke.sh target/release/niruwebapp
sh tests/x11-smoke.sh target/release/niruwebapp target/release/examples/x11-dummy
sh tests/browser-class.sh target/release/niruwebapp "$(command -v helium)"
```

The X11 tests need Xvfb; `browser-class.sh` runs a real Chromium-family
browser in a temporary HOME.

MIT. Copyright 2026 Nicklas Rudolfsson.
