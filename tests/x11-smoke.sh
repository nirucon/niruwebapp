#!/bin/sh
# B2: single-window launch against a fake X environment.
# Usage: x11-smoke.sh BIN DUMMY   (DUMMY = target/.../examples/x11-dummy)
set -eu
BIN=${1:?usage: x11-smoke.sh BIN DUMMY}
DUMMY=${2:?usage: x11-smoke.sh BIN DUMMY}
if ! command -v Xvfb >/dev/null 2>&1; then
  echo 'SKIP: Xvfb not installed'
  exit 0
fi
T=$(mktemp -d)
PIDS=
cleanup() { for p in $PIDS; do kill "$p" 2>/dev/null || :; done; rm -rf "$T"; }
trap cleanup EXIT
export HOME=$T/home XDG_DATA_HOME=$T/data XDG_RUNTIME_DIR=$T/run PATH=$T/bin:$PATH
mkdir -p "$HOME" "$T/bin" "$XDG_RUNTIME_DIR"
cat > "$T/bin/helium" <<'BROWSER'
#!/bin/sh
printf '%s\n' "$*" >> "$HOME/browser-args"
BROWSER
chmod +x "$T/bin/helium"
Xvfb -displayfd 3 -nolisten tcp -screen 0 640x480x24 3>"$T/display" >/dev/null 2>&1 &
PIDS="$PIDS $!"
i=0; while [ ! -s "$T/display" ] && [ $i -lt 100 ]; do i=$((i + 1)); sleep 0.05; done
DISPLAY=:$(cat "$T/display"); export DISPLAY
# Launches are counted from the --app lines only (detection may run --version).
launches() { grep -c -- '--app=' "$HOME/browser-args" 2>/dev/null || :; }
wait_log() { # FILE PATTERN
  i=0; while ! grep -q "$2" "$1" 2>/dev/null; do i=$((i + 1)); [ $i -lt 100 ] || { echo "timeout waiting for $2 in $1" >&2; exit 1; }; sleep 0.05; done
}
"$BIN" create Iso https://example.net/ >/dev/null
"$BIN" create Mail https://example.com/mail/ --shared-profile >/dev/null

# No window yet: the browser is spawned, a quick second click is swallowed.
"$BIN" launch iso
i=0; while [ "$(launches)" -lt 1 ] && [ $i -lt 100 ]; do i=$((i + 1)); sleep 0.05; done
[ "$(launches)" -eq 1 ]
"$BIN" launch iso 2>"$T/err"
grep -q 'already starting' "$T/err"
sleep 0.2; [ "$(launches)" -eq 1 ]

# Isolated window (root child, no EWMH list): activated instead of spawning.
"$DUMMY" example.net niruwebapp-iso "$T/iso.log" & PIDS="$PIDS $!"
wait_log "$T/iso.log" ready
"$BIN" launch iso
wait_log "$T/iso.log" activated
sleep 0.2; [ "$(launches)" -eq 1 ]

# Shared window joined the running browser: class Helium, instance from URL,
# published through _NET_CLIENT_LIST.
"$DUMMY" example.com__mail Helium "$T/mail.log" --client-list & PIDS="$PIDS $!"
wait_log "$T/mail.log" ready
"$BIN" launch mail
wait_log "$T/mail.log" activated
sleep 0.2; [ "$(launches)" -eq 1 ]
# A window of an unrelated site in the same browser is not taken.
"$DUMMY" example.org Helium "$T/other.log" --client-list & PIDS="$PIDS $!"
wait_log "$T/other.log" ready
"$BIN" launch mail
[ "$(grep -c activated "$T/mail.log")" -eq 2 ]
! grep -q activated "$T/other.log"

# --new-window always spawns.
"$BIN" launch mail --new-window
i=0; while [ "$(launches)" -lt 2 ] && [ $i -lt 100 ]; do i=$((i + 1)); sleep 0.05; done
[ "$(launches)" -eq 2 ]

"$BIN" doctor > "$T/doctor"
# Once _NET_CLIENT_LIST exists it is authoritative, and the unlisted
# isolated dummy no longer counts.
grep -q "^iso	isolated	niruwebapp-iso	0	ok	ok$" "$T/doctor"
grep -q "^mail	shared	example.com__mail	1	ok	ok$" "$T/doctor"
echo 'PASS: single-window launch, activation, --new-window and doctor under Xvfb'
