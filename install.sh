#!/bin/sh
set -eu
VERSION="0.1.1"
REPO="${NIRUWEBAPP_REPO:-nirucon/niruwebapp}"
PREFIX="${NIRUWEBAPP_PREFIX:-${NIRU_PREFIX:-$HOME/.local}}"
BIN="$PREFIX/bin"
need(){ command -v "$1" >/dev/null 2>&1; }
say(){ printf '%s\n' "$*"; }
die(){ say "niruwebapp install: $*" >&2; exit 1; }
fetch(){ need curl && curl -fsSL "$1" -o "$2" || wget -qO "$2" "$1" || die "download failed"; }
managed_guard(){ [ -L "$BIN/niruwebapp" ] || return 0; case "$(readlink "$BIN/niruwebapp")" in */share/nirux/current/bin/niruwebapp) die 'NIRUWEBAPP is managed by NIRUX; use nirux update.' ;; esac; }
managed_guard
case "${1:-}" in
  --build) need cargo || die 'cargo required'; (cd "$(dirname "$0")" && cargo build --release --locked); install -d "$BIN"; install -m755 "$(dirname "$0")/target/release/niruwebapp" "$BIN/niruwebapp" ;;
  --uninstall) rm -f "$BIN/niruwebapp" ;;
  "") d=$(mktemp -d); trap 'rm -rf "$d"' EXIT; fetch "https://github.com/$REPO/releases/download/v$VERSION/niruwebapp" "$d/niruwebapp"; fetch "https://github.com/$REPO/releases/download/v$VERSION/SHA256SUMS" "$d/SHA256SUMS"; sum=$(sha256sum "$d/niruwebapp" | cut -d' ' -f1); grep -q "^$sum  niruwebapp$" "$d/SHA256SUMS" || die 'checksum mismatch'; install -d "$BIN"; install -m755 "$d/niruwebapp" "$BIN/niruwebapp"; say "Installed NIRUWEBAPP $VERSION" ;;
  *) die "unknown option $1" ;;
esac
