# NIRUWEBAPP

Web apps as their own desktop windows, using the Chromium-family browser you already have.

**Status: alpha** · Part of the [NIRUX](https://github.com/nirucon/nirux) suite · [MIT License](LICENSE)

## About

NIRUWEBAPP is a small web-app manager for the NIRUX desktop. It turns a URL into
a desktop entry that opens the site in an installed Helium/Chromium-family
browser in app mode, with its own window class, icon and (by default) its own
browser profile. There is no Electron, WebView runtime, extra browser or
background service.

## Features

- Create, list, launch, edit and remove web apps from the command line.
- An isolated browser profile per web app, or the browser's ordinary profile
  with `--shared-profile` (useful for Microsoft 365); switching is
  non-destructive.
- Launching a web app that already has a window activates that window instead
  of opening a second one.
- Desktop entries declare the real window class, so launchers, taskbars and
  NIRUWM window rules can match them.
- Letter icons in the active NIRUX accent colour, or your own icon, or (opt-in)
  the site's own icon.
- `doctor` and `list --json` report expected classes, open windows and stale
  entries for NIRU Launcher views.
- Careful removal: prints what it will delete, asks on a terminal, keeps
  profiles unless told otherwise.

## Requirements

- Linux with X11 (Debian, Arch and Void glibc, x86_64).
- One of Helium, Chromium, Google Chrome or Brave, in `~/.local/bin` or on
  `PATH`. No browser is installed automatically by NIRUWEBAPP (NIRUX can
  install Helium).

## Installation

### With NIRUX

NIRUWEBAPP is a core component: `nirux install` and `nirux update` install and
update it from the pinned manifest. The standalone installer refuses to touch a
NIRUX-managed copy.

### Standalone

```sh
git clone https://github.com/nirucon/niruwebapp
cd niruwebapp
./install.sh            # download the pinned release, verify SHA256SUMS, install to ~/.local/bin
./install.sh --build    # or build from this checkout with cargo
```

`NIRUWEBAPP_PREFIX` (or `NIRU_PREFIX`) changes the prefix (default `~/.local`).

## Usage

```sh
niruwebapp create "YouTube Music" https://music.youtube.com
niruwebapp create https://teams.cloud.microsoft/ --shared-profile   # name: Teams
niruwebapp list
niruwebapp list --json
niruwebapp launch "YouTube Music"
niruwebapp launch teams --new-window
niruwebapp edit youtube-music --name "YT Music"
niruwebapp edit youtube-music --url https://music.youtube.com/library
niruwebapp edit youtube-music --shared-profile
niruwebapp edit youtube-music --isolated-profile
niruwebapp edit youtube-music --fetch-icon
niruwebapp icon youtube-music ~/Pictures/yt.png
niruwebapp doctor
niruwebapp refresh
niruwebapp remove youtube-music --dry-run
niruwebapp remove youtube-music --delete-profile --yes
```

| Command | Options |
|---|---|
| `create [NAME] URL` | `--browser PATH`, `--shared-profile`, `--icon PATH` or `--fetch-icon` |
| `list` | `--json` |
| `launch ID\|NAME` | `--new-window` |
| `edit ID\|NAME` | `--name NAME`, `--url URL`, `--icon PATH` or `--fetch-icon`, `--shared-profile` or `--isolated-profile` |
| `icon ID\|NAME PATH` | Set your own icon |
| `remove ID\|NAME` | `--delete-profile`, `--dry-run` or `--yes` |
| `refresh` | Redraw letter icons and rewrite desktop entries (for example after a theme change) |
| `doctor` | Read-only report of every web app |

Without a name, `create` names the web app after its host
(`teams.cloud.microsoft` becomes "Teams"). For Microsoft 365 hosts it suggests
`--shared-profile`; it never switches the mode by itself.

### Profiles

Each web app gets an ordinary desktop entry and, by default, a dedicated
browser profile. `--shared-profile` uses the browser's ordinary profile instead,
so sign-in follows the normal browser session; the trade-off is that cookies and
sessions are shared with normal browsing. `edit --shared-profile` and
`edit --isolated-profile` switch an existing web app; the old profile is kept on
disk so switching back is non-destructive. Restart the web app after switching.

### Windows and window classes

Launching a web app that already has a window activates it
(`_NET_ACTIVE_WINDOW`); a second click while the first window is still starting
is ignored. `--new-window` always opens a new window.

| Profile | `StartupWMClass` | Example WM_CLASS |
|---|---|---|
| isolated | `niruwebapp-<id>` | `claude.ai`, `niruwebapp-claude` |
| shared | host and path, `/` as `__` | `outlook.cloud.microsoft__mail`, `Helium` |

A shared-profile web app usually joins the browser that is already running, and
Chromium then ignores `--class`; the instance name derived from the URL is the
only stable identity. International host names are used as punycode.

`niruwebapp doctor` lists every web app, its expected class, how many matching
windows are open, and whether its desktop entry and browser are current. It only
reads. `niruwebapp list --json` gives the same data (id, name, url, profile,
icon, expected class, running).

### Icons

Every web app gets a letter on a rounded square in the NIRUX accent colour of
the selected theme; two web apps with the same first letter get two letters (Ch,
Cl, Co). `--fetch-icon` is an explicit opt-in: it asks the site over HTTPS for
`/apple-touch-icon.png`, then `/favicon.ico`, with a 5 s timeout and a 512 KB
limit, and only accepts a valid PNG (also inside an ICO). NIRUWEBAPP never
contacts a site otherwise. `--icon PATH` or `niruwebapp icon ID PATH` sets your
own icon and overrides both.

## Configuration

There is no configuration file. The browser is chosen with `--browser PATH` or
`NIRUWEBAPP_BROWSER`; otherwise Helium, Chromium, Chrome or Brave is detected in
`~/.local/bin` and on `PATH`. If a web app's browser path disappears (for
example after a Helium update), the next launch detects the browser again and
updates the record.

## Files and paths

| Path | Purpose |
|---|---|
| `~/.local/share/niruwebapp/apps.tsv` | The list of web apps |
| `~/.local/share/niruwebapp/profiles/<id>/` | Isolated browser profiles |
| `~/.local/share/applications/niruwebapp-<id>.desktop` | Desktop entries |
| `~/.local/share/icons/hicolor/128x128/apps/niruwebapp-<id>.png` | Icons |
| `~/.local/share/niruwebapp/backup-0.1/` | One-time backup made when upgrading from 0.1 |

`$XDG_DATA_HOME` replaces `~/.local/share` when set.

## Troubleshooting

### Microsoft 365 sign-in

Teams uses a third-party authentication frame. If the browser blocks
third-party cookies, Teams can show "We need you to sign in again" and its Sign
in button may do nothing. Keep the global setting on "Block third-party cookies"
and add these sites to "Sites allowed to use third-party cookies":

- `[*.]cloud.microsoft`
- `[*.]microsoft.com`
- `[*.]microsoftonline.com`
- `[*.]office.com`
- `[*.]live.com`
- `[*.]sharepoint.com`
- `[*.]onedrive.com`

The exceptions live in the browser profile, so they apply to normal browsing and
every web app that uses the shared profile.

### Other problems

- **A launcher or window rule does not match:** run `niruwebapp doctor` and use
  the expected class it reports; `niruwebapp refresh` rewrites stale entries.
- **Upgrading from 0.1:** the first command after the upgrade backs up
  `apps.tsv` and every desktop entry to `~/.local/share/niruwebapp/backup-0.1/`
  (once; never overwritten), gives web apps without an icon a letter icon,
  keeps your own icons and rewrites the entries. Profiles and sign-ins are not
  changed.

## Uninstall

Remove each web app first if you want its entry gone:

```sh
niruwebapp remove ID --dry-run          # see what would be deleted
niruwebapp remove ID --delete-profile   # also delete an isolated profile
```

`remove` deletes the desktop entry and the icon, and the isolated profile (with
its size shown) only with `--delete-profile`. It asks for confirmation on a
terminal; scripts must pass `--yes`. A shared profile is never touched.

Then remove the program: with NIRUX via `nirux uninstall`, standalone with
`./install.sh --uninstall`.

## Building from source

```sh
cargo test --locked
cargo build --release --locked --bins --examples
sh tests/smoke.sh target/release/niruwebapp
sh tests/x11-smoke.sh target/release/niruwebapp target/release/examples/x11-dummy
sh tests/browser-class.sh target/release/niruwebapp "$(command -v helium)"
```

The X11 tests need Xvfb; `browser-class.sh` runs a real Chromium-family browser
in a temporary HOME.

## Status, disclaimer and how it's made

**Status: alpha.** I use it for my everyday web apps, but window classes depend
on how each Chromium-family browser behaves, and that changes under it.

NIRUX and every niru* project are made by me, Nicklas Rudolfsson
([@nirucon](https://github.com/nirucon)), for me. They are tailored to my own
needs, preferences and workflows, on my own machines. If they fit yours too,
that's a nice bonus.

- **Free and open source.** Anyone is free to use, study, change and share
  it under the [MIT License](LICENSE).
- **No guarantees.** There is no warranty of any kind. Use it at your own
  risk, read what you install and keep backups of anything that matters.
- **How it's made.** Some of these projects I built from scratch by hand. In
  more recent releases I have used Claude, Anthropic's AI, to do much of the
  heavy lifting under my direction: I decide what to build and how it should
  behave, and I test it on my own machines. It saves me a lot of time, and
  the result is quite OK.
- **Alpha, beta, dev, and maybe forever.** Most things here are alpha, beta
  or dev, and may stay that way. Maybe one day there will be a real stable
  release, but given how fast development moves, and how I am as a person,
  maybe never :)

| Label | Meaning |
|---|---|
| dev | Under active construction; may not build, install or work yet. |
| alpha | Works on my machines, but incomplete or rough; expect breaking changes. |
| beta | In daily use on my machines and mostly complete; rough edges remain. |

## License

MIT License, see [LICENSE](LICENSE). Copyright (c) 2026 Nicklas Rudolfsson.
