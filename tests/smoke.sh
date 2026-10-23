#!/bin/sh
set -eu
BIN=${1:?usage: smoke.sh BIN}
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
export HOME=$T/home XDG_DATA_HOME=$T/data PATH=$T/bin:$PATH
mkdir -p "$HOME" "$T/bin"
cat > "$T/bin/helium" <<'BROWSER'
#!/bin/sh
printf '%s\n' "$*" >> "$HOME/browser-args"
BROWSER
chmod +x "$T/bin/helium"
"$BIN" --version | grep -qx 'NIRUWEBAPP 0.1.2'
printf '\211PNG\r\n\032\n' > "$T/icon.png"
"$BIN" create 'Example App' 'https://example.com' --icon "$T/icon.png"
[ -f "$T/data/applications/niruwebapp-example-app.desktop" ]
[ -f "$T/data/icons/hicolor/128x128/apps/niruwebapp-example-app.png" ]
grep -q '^Icon=.*niruwebapp-example-app.png$' "$T/data/applications/niruwebapp-example-app.desktop"
"$BIN" list | grep -q '^example-app.*Example App.*https://example.com$'
"$BIN" launch example-app
grep -q -- '--app=https://example.com' "$HOME/browser-args"
grep -q 'launch example-app$' "$T/data/applications/niruwebapp-example-app.desktop"
"$BIN" icon example-app "$T/icon.png"
"$BIN" remove example-app
[ ! -e "$T/data/applications/niruwebapp-example-app.desktop" ]
[ ! -e "$T/data/icons/hicolor/128x128/apps/niruwebapp-example-app.png" ]
echo 'PASS: create/list/launch/remove and browser args'
