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
niruwebapp list
niruwebapp launch "YouTube Music"
niruwebapp edit youtube-music --name "YT Music"
niruwebapp remove youtube-music --delete-profile
```

Each webapp gets an ordinary desktop entry under
`~/.local/share/applications/` and a dedicated browser profile under
`~/.local/share/niruwebapp/profiles/`. Use `--shared-profile` to use the
browser's ordinary profile instead. Removing the launcher keeps the profile
unless `--delete-profile` is explicit.

`--browser PATH` selects a backend; otherwise Helium, Chromium, Chrome or Brave
is detected. No browser is installed automatically. `--icon PATH` copies a
local icon.

## Build

```sh
cargo test --locked
cargo build --release --locked
sh tests/smoke.sh target/release/niruwebapp
```

MIT. Copyright 2026 Nicklas Rudolfsson.
