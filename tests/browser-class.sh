#!/bin/sh
# B1 against a real Chromium-family browser: the window classes niruwebapp
# expects must be the ones the browser sets. Needs Xvfb and network-free
# startup only (the pages need not load).
# Usage: browser-class.sh BIN BROWSER
set -eu
BIN=${1:?usage: browser-class.sh BIN BROWSER}
BROWSER=${2:?usage: browser-class.sh BIN BROWSER}
T=$(mktemp -d)
PIDS=
cleanup() {
  for p in $PIDS; do kill "$p" 2>/dev/null || :; done
  pkill -f "$T" 2>/dev/null || :
  sleep 1
  pkill -9 -f "$T" 2>/dev/null || :
  rm -rf "$T"
}
trap cleanup EXIT
export HOME=$T/home XDG_DATA_HOME=$T/data XDG_CONFIG_HOME=$T/config XDG_CACHE_HOME=$T/cache XDG_RUNTIME_DIR=$T/run
mkdir -p "$HOME" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
unset DBUS_SESSION_BUS_ADDRESS
Xvfb -displayfd 3 -nolisten tcp -screen 0 1280x800x24 3>"$T/display" >/dev/null 2>&1 &
PIDS="$PIDS $!"
i=0; while [ ! -s "$T/display" ] && [ $i -lt 100 ]; do i=$((i + 1)); sleep 0.05; done
DISPLAY=:$(cat "$T/display"); export DISPLAY
# The ordinary browser is already running in the shared (default) profile.
"$BROWSER" --no-first-run --no-default-browser-check about:blank >"$T/browser.log" 2>&1 &
PIDS="$PIDS $!"
sleep 4
"$BIN" create Mail 'https://example.com/mail/' --shared-profile --browser "$BROWSER" >/dev/null
"$BIN" create Idn 'https://bücher.example/' --shared-profile --browser "$BROWSER" >/dev/null
"$BIN" create Iso 'https://example.net/a/b.c?q=1' --browser "$BROWSER" >/dev/null
for app in mail idn iso; do "$BIN" launch "$app"; done
want='mail	shared	example.com__mail	1
idn	shared	xn--bcher-kva.example	1
iso	isolated	niruwebapp-iso	1'
i=0
while :; do
  got=$("$BIN" doctor | awk -F '\t' 'NR > 1 && NF == 6 { print $1 "\t" $2 "\t" $3 "\t" $4 }')
  [ "$got" = "$want" ] && break
  i=$((i + 1))
  if [ $i -ge 60 ]; then
    echo "expected:"; echo "$want"; echo "got:"; echo "$got"
    command -v xprop >/dev/null && for w in $(xprop -root _NET_CLIENT_LIST 2>/dev/null | sed 's/.*# //; s/,//g'); do xprop -id "$w" WM_CLASS; done
    exit 1
  fi
  sleep 0.5
done
"$BIN" doctor
echo "PASS: real browser window classes match ($("$BROWSER" --version 2>/dev/null | head -1))"
