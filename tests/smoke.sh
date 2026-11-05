#!/bin/sh
set -eu
BIN=${1:?usage: smoke.sh BIN}
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
export HOME=$T/home XDG_DATA_HOME=$T/data XDG_CONFIG_HOME=$T/config XDG_RUNTIME_DIR=$T/run PATH=$T/bin:$PATH
unset DISPLAY
mkdir -p "$HOME" "$T/bin"
cat > "$T/bin/helium" <<'BROWSER'
#!/bin/sh
printf '%s\n' "$*" >> "$HOME/browser-args"
BROWSER
chmod +x "$T/bin/helium"
wait_for_launch() {
  want=${1:-1}
  i=0
  while [ "$i" -lt 100 ]; do
    now=$(wc -l < "$HOME/browser-args" 2>/dev/null || echo 0)
    [ "$now" -ge "$want" ] && return 0
    i=$((i + 1)); sleep 0.05
  done
  echo "browser launch did not produce argument line $want" >&2
  return 1
}
"$BIN" --version | grep -qx 'NIRUWEBAPP 0.1.4'
printf '\211PNG\r\n\032\n' > "$T/icon.png"
"$BIN" create 'Example App' 'https://example.com' --icon "$T/icon.png"
[ -f "$T/data/applications/niruwebapp-example-app.desktop" ]
[ -f "$T/data/icons/hicolor/128x128/apps/niruwebapp-example-app.png" ]
grep -q '^Icon=.*niruwebapp-example-app.png$' "$T/data/applications/niruwebapp-example-app.desktop"
"$BIN" list | grep -q '^example-app.*Example App.*https://example.com$'
"$BIN" list --json > "$T/list.json"
grep -q '"id": "example-app", "name": "Example App", "url": "https://example.com", "profile": "isolated"' "$T/list.json"
grep -q '"expected_class": "niruwebapp-example-app", "running": null' "$T/list.json"
if command -v python3 >/dev/null 2>&1; then python3 -c 'import json,sys; assert json.load(open(sys.argv[1]))[0]["icon_source"] == "user"' "$T/list.json"; fi
"$BIN" launch example-app
wait_for_launch 1
grep -q -- '--app=https://example.com' "$HOME/browser-args"
grep -q -- '--user-data-dir=.*/profiles/example-app' "$HOME/browser-args"
grep -q 'launch example-app$' "$T/data/applications/niruwebapp-example-app.desktop"
grep -qx 'Comment=Webapp · example.com' "$T/data/applications/niruwebapp-example-app.desktop"
grep -qx 'Categories=Network;WebBrowser;' "$T/data/applications/niruwebapp-example-app.desktop"

"$BIN" edit example-app --shared-profile
awk -F '\t' '$1 == "example-app" && $5 == "shared" { found = 1 } END { exit !found }' "$T/data/niruwebapp/apps.tsv"
"$BIN" launch example-app
wait_for_launch 2
tail -n 1 "$HOME/browser-args" | grep -q -- '--app=https://example.com'
if tail -n 1 "$HOME/browser-args" | grep -q -- '--user-data-dir='; then
  echo 'shared profile used a user-data-dir' >&2
  exit 1
fi

"$BIN" edit example-app --isolated-profile
awk -F '\t' '$1 == "example-app" && $5 == "profile" { found = 1 } END { exit !found }' "$T/data/niruwebapp/apps.tsv"
"$BIN" launch example-app
wait_for_launch 3
tail -n 1 "$HOME/browser-args" | grep -q -- '--user-data-dir=.*/profiles/example-app'

if "$BIN" edit example-app --shared-profile --isolated-profile 2>/dev/null; then
  echo 'conflicting profile flags were accepted' >&2
  exit 1
fi

# U2: the name comes from the host; Microsoft 365 gets a profile hint.
"$BIN" create https://teams.cloud.microsoft/ > "$T/out"
grep -q '^created Teams (teams)$' "$T/out"
grep -q '^hint: Microsoft 365 .*--shared-profile' "$T/out"
awk -F '\t' '$1 == "teams" && $5 == "profile" { found = 1 } END { exit !found }' "$T/data/niruwebapp/apps.tsv"
"$BIN" create https://outlook.cloud.microsoft/mail/ --shared-profile > "$T/out"
if grep -q '^hint' "$T/out"; then echo 'unexpected hint' >&2; exit 1; fi
"$BIN" remove teams --yes >/dev/null
"$BIN" remove outlook --yes >/dev/null

# B4: Desktop Entry escaping of names and of an Exec path with reserved
# characters (space, %, $).
"$BIN" create 'Smör 100% "$x" \ `y`' https://example.org/ >/dev/null
grep -qxF 'Name=Smör 100% "$x" \\ `y`' "$T/data/applications/niruwebapp-sm-r-100-x-y.desktop"
mkdir -p "$T/odd dir 100%\$"
cp "$BIN" "$T/odd dir 100%\$/niruwebapp"
"$T/odd dir 100%\$/niruwebapp" edit sm-r-100-x-y >/dev/null
grep -qxF "Exec=\"$T/odd dir 100%%\\\\\$/niruwebapp\" launch sm-r-100-x-y" "$T/data/applications/niruwebapp-sm-r-100-x-y.desktop"
"$BIN" remove sm-r-100-x-y --yes >/dev/null
"$BIN" icon example-app "$T/icon.png"
# U3: a dry run lists what goes and what stays; without --yes a
# non-interactive remove refuses and changes nothing.
head -c 3000 /dev/zero > "$T/data/niruwebapp/profiles/example-app/Cookies"
"$BIN" remove example-app --dry-run > "$T/plan"
grep -q "^  delete desktop entry .*/niruwebapp-example-app.desktop$" "$T/plan"
grep -q "^  delete icon .*/niruwebapp-example-app.png$" "$T/plan"
grep -q "^  keep profile .*/profiles/example-app (2.9 KB; --delete-profile removes it)$" "$T/plan"
"$BIN" remove example-app --delete-profile --dry-run | grep -q "^  delete profile .*/profiles/example-app (2.9 KB)$"
if "$BIN" remove example-app < /dev/null 2>"$T/err"; then echo 'remove without --yes succeeded' >&2; exit 1; fi
grep -q 'without --yes' "$T/err"
[ -f "$T/data/applications/niruwebapp-example-app.desktop" ]
"$BIN" remove example-app --yes
[ -d "$T/data/niruwebapp/profiles/example-app" ]
[ ! -e "$T/data/applications/niruwebapp-example-app.desktop" ]
[ ! -e "$T/data/icons/hicolor/128x128/apps/niruwebapp-example-app.png" ]
echo 'PASS: create/list/launch/edit/remove, profile modes and browser args'
