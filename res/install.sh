#!/bin/sh
# Installs (or removes, with --uninstall) pa2 for the current
# user: binary in ~/.local/bin, launcher entry and icon in ~/.local/share.
# Run from the unpacked package folder, or via `just install` from the repo.
set -eu

APP_ID=io.github.gonzaloism.pa2
PREFIX="${PREFIX:-$HOME/.local}"
HERE="$(cd "$(dirname "$0")" && pwd)"

BIN="$PREFIX/bin/pa2"
DESKTOP="$PREFIX/share/applications/$APP_ID.desktop"
ICON="$PREFIX/share/icons/hicolor/scalable/apps/$APP_ID.svg"

refresh() {
    # Let launchers notice the change right away (both tools are optional).
    command -v update-desktop-database >/dev/null 2>&1 &&
        update-desktop-database -q "$PREFIX/share/applications" || true
    command -v gtk-update-icon-cache >/dev/null 2>&1 &&
        gtk-update-icon-cache -q -t "$PREFIX/share/icons/hicolor" || true
}

if [ "${1:-}" = "--uninstall" ]; then
    rm -f "$BIN" "$DESKTOP" "$ICON"
    refresh
    echo "pa2 removed."
    echo "Your settings, favorites and session stay in ~/.config/pa2."
    exit 0
fi

# Files as laid out in the package (bin/, share/); `just install` passes
# SOURCE_* to install straight from the repository instead.
SRC_BIN="${SOURCE_BIN:-$HERE/bin/pa2}"
SRC_DESKTOP="${SOURCE_DESKTOP:-$HERE/share/applications/$APP_ID.desktop}"
SRC_ICON="${SOURCE_ICON:-$HERE/share/icons/hicolor/scalable/apps/$APP_ID.svg}"

install -Dm755 "$SRC_BIN" "$BIN"
install -Dm644 "$SRC_ICON" "$ICON"
install -d "$(dirname "$DESKTOP")"
# Point the launcher at the installed binary, so it works even when
# ~/.local/bin isn't on the launcher's PATH.
sed "s|^Exec=.*|Exec=$BIN|" "$SRC_DESKTOP" > "$DESKTOP"
chmod 644 "$DESKTOP"
refresh

echo "pa2 installed:"
echo "  $BIN"
echo "  $DESKTOP"
echo "  $ICON"
echo "Open it from the app launcher (search \"pa2\")."
