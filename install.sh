#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Henry Chea
# Install Rusted Snake for the current user, or system-wide with --system.
#
#   ./install.sh                 # ~/.local
#   ./install.sh --system        # /usr/local (uses sudo when needed)
#   ./install.sh --prefix=/opt   # custom prefix
#   ./install.sh --with-udev     # also install the OpenRazer/uinput rules
#   ./install.sh --uninstall
set -eu

REPO="$(CDPATH= cd -- "$(dirname -- "$(readlink -f -- "$0")")" && pwd)"
DESKTOP_ID="com.henrychea.RustedSnake.desktop"
PREFIX="${PREFIX:-$HOME/.local}"
BUILD=1
UDEV=0
UNINSTALL=0
SUDO=""

usage() {
	cat <<'USAGE'
Install Rusted Snake for Linux.

Usage: ./install.sh [options]

Options:
  --prefix=DIR       Install under DIR (default: ~/.local)
  --system           Shortcut for --prefix=/usr/local
  --no-build         Use the existing target/release/rusted-snake
  --with-udev        Install the raw HID and uinput udev rules (needs sudo)
  --uninstall        Remove the installed binary, desktop entry and icons
  -h, --help         Show this help

The app is installed as:
  DIR/bin/rusted-snake
  DIR/share/applications/com.henrychea.RustedSnake.desktop
  DIR/share/icons/hicolor/{scalable,32x32,...,512x512}/apps/rusted-snake.*

Settings and macro data in ~/.config/rusted-snake are never removed.
USAGE
}

die() {
	printf 'install.sh: %s\n' "$*" >&2
	exit 1
}

while [ "$#" -gt 0 ]; do
	case "$1" in
		--prefix=*) PREFIX="${1#--prefix=}" ;;
		--prefix)
			shift
			[ "$#" -gt 0 ] || die "--prefix needs a directory"
			PREFIX="$1"
			;;
		--system) PREFIX="/usr/local" ;;
		--no-build) BUILD=0 ;;
		--with-udev) UDEV=1 ;;
		--uninstall) UNINSTALL=1 ;;
		-h|--help) usage; exit 0 ;;
		*) die "unknown option: $1 (try --help)" ;;
	esac
	shift
done

[ -n "$PREFIX" ] || die "empty prefix"
case "$PREFIX" in
	/*) ;;
	*) PREFIX="$REPO/$PREFIX" ;;
esac

BINDIR="$PREFIX/bin"
APPDIR="$PREFIX/share/applications"
ICONROOT="$PREFIX/share/icons/hicolor"
BIN="$BINDIR/rusted-snake"
DESKTOP="$APPDIR/$DESKTOP_ID"

if [ "$(id -u)" -eq 0 ]; then
	SUDO=""
elif [ -w "$PREFIX" ] || { [ ! -e "$PREFIX" ] && [ -w "$(dirname "$PREFIX")" ]; }; then
	SUDO=""
else
	SUDO="sudo"
fi

run() {
	if [ -n "$SUDO" ]; then
		"$SUDO" "$@"
	else
		"$@"
	fi
}

refresh_caches() {
	if command -v update-desktop-database >/dev/null 2>&1; then
		run update-desktop-database "$APPDIR" >/dev/null 2>&1 || true
	fi
	if command -v gtk-update-icon-cache >/dev/null 2>&1; then
		run gtk-update-icon-cache -f -t "$ICONROOT" >/dev/null 2>&1 || true
	fi
}

install_udev() {
	[ -d /etc/udev/rules.d ] || die "/etc/udev/rules.d does not exist"
	if [ "$(id -u)" -ne 0 ] && ! command -v sudo >/dev/null 2>&1; then
		die "installing udev rules needs root (run with sudo)"
	fi
	run install -Dm644 "$REPO/linux/70-rusted-snake-hidraw.rules" /etc/udev/rules.d/70-rusted-snake-hidraw.rules
	run install -Dm644 "$REPO/linux/99-rusted-snake-uinput.rules" /etc/udev/rules.d/99-rusted-snake-uinput.rules
	if command -v udevadm >/dev/null 2>&1; then
		run udevadm control --reload-rules
		run udevadm trigger
	fi
	printf 'Installed udev rules. Add yourself to the input group for macros, then sign in again:\n'
	printf '  sudo usermod -aG input "$USER"\n'
}

if [ "$UNINSTALL" -eq 1 ]; then
	run rm -f "$BIN" "$DESKTOP"
	run rm -f "$ICONROOT/scalable/apps/rusted-snake.svg"
	for size in 32 48 64 128 256 512; do
		run rm -f "$ICONROOT/${size}x${size}/apps/rusted-snake.png"
	done
	refresh_caches
	if [ "$UDEV" -eq 1 ]; then
		run rm -f /etc/udev/rules.d/70-rusted-snake-hidraw.rules /etc/udev/rules.d/99-rusted-snake-uinput.rules
		if command -v udevadm >/dev/null 2>&1; then
			run udevadm control --reload-rules
		fi
	fi
	printf 'Removed Rusted Snake from %s. Settings and backups were left alone.\n' "$PREFIX"
	exit 0
fi

[ -f "$REPO/Cargo.toml" ] || die "run this script from the Rusted Snake checkout"
if [ "$BUILD" -eq 1 ]; then
	printf 'Building Rusted Snake (release)...\n'
	(cd "$REPO" && cargo build --release -p rusted-snake)
fi
[ -x "$REPO/target/release/rusted-snake" ] || die "release binary not found; run without --no-build"
[ -f "$REPO/linux/$DESKTOP_ID" ] || die "missing linux/$DESKTOP_ID"

run install -Dm755 "$REPO/target/release/rusted-snake" "$BIN"

TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT INT TERM
# The launcher entry must use an absolute Exec path when installed under a
# custom prefix or ~/.local/bin is not in PATH.
sed -e "s|^Exec=.*|Exec=\"$BIN\"|" \
    -e "s|^TryExec=.*|TryExec=$BIN|" \
    "$REPO/linux/$DESKTOP_ID" >"$TMP"
run install -Dm644 "$TMP" "$DESKTOP"

run install -Dm644 "$REPO/assets/icon/rusted-snake.svg" "$ICONROOT/scalable/apps/rusted-snake.svg"
for size in 32 48 64 128 256 512; do
	run install -Dm644 "$REPO/assets/icon/rusted-snake-$size.png" "$ICONROOT/${size}x${size}/apps/rusted-snake.png"
done
refresh_caches

if [ "$UDEV" -eq 1 ]; then
	install_udev
fi

printf '\nInstalled Rusted Snake:\n  %s\n  %s\n' "$BIN" "$DESKTOP"
case ":$PATH:" in
	*":$BINDIR:"*) ;;
	*) printf '\n%s is not in PATH. The desktop entry uses an absolute path.\n' "$BINDIR" ;;
esac
