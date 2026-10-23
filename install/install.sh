#!/usr/bin/env bash
# Install VoipAppz Builder for the current user on Linux. No sudo.
#
#   install/install.sh [--workspace DIR] [--api-dir DIR] [--client-url URL]
#                      [--no-start-app] [--yes]
#   install/install.sh --uninstall [--purge]
#
# Builds the voipappz-builder binary from this checkout, installs it and the
# launcher in ~/.local/bin, writes settings to
# ~/.config/voipappz-builder/builder.env and adds "VoipAppz Builder" to the
# desktop menu.
set -euo pipefail

REPO=$(cd "$(dirname "$0")/.." && pwd)
PARENT=$(dirname "$REPO")
BIN="$HOME/.local/bin"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/voipappz-builder"
APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICONS="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps"
DESKTOP="$APPS/voipappz-builder.desktop"

WORKSPACE="$PARENT/app-builder"
API_DIR="$PARENT/voipappz-api"
CLIENT_URL="http://localhost:3000"
START_APP=1
ASSUME_YES=0
UNINSTALL=0
PURGE=0

while [ $# -gt 0 ]; do
  case "$1" in
    --workspace) WORKSPACE="$2"; shift 2 ;;
    --api-dir) API_DIR="$2"; shift 2 ;;
    --client-url) CLIENT_URL="$2"; shift 2 ;;
    --no-start-app) START_APP=0; shift ;;
    --yes|-y) ASSUME_YES=1; shift ;;
    --uninstall) UNINSTALL=1; shift ;;
    --purge) PURGE=1; shift ;;
    -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
    *) echo "Unknown option: $1" >&2; exit 2 ;;
  esac
done

if [ "$UNINSTALL" = 1 ]; then
  "$BIN/voipappz-builder-launch" stop 2>/dev/null || true
  rm -f "$BIN/voipappz-builder" "$BIN/voipappz-builder-launch" "$DESKTOP" "$ICONS/voipappz-builder.png"
  [ "$PURGE" = 1 ] && rm -rf "$CONFIG_DIR"
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS" || true
  echo "VoipAppz Builder removed."
  [ "$PURGE" = 1 ] || echo "Settings kept in $CONFIG_DIR (use --purge to remove them)."
  exit 0
fi

ask() {
  local prompt="$1" default="$2" answer
  if [ "$ASSUME_YES" = 1 ] || [ ! -t 0 ]; then echo "$default"; return; fi
  read -r -p "$prompt [$default]: " answer
  echo "${answer:-$default}"
}

echo "VoipAppz Builder installer"
echo
WORKSPACE=$(ask "App checkout the agent works in (a git worktree of ../app)" "$WORKSPACE")
API_DIR=$(ask "voipappz-api checkout for the local API (empty for none)" "$API_DIR")
CLIENT_URL=$(ask "Chat client and previews (the app's dev server)" "$CLIENT_URL")

[ -d "$WORKSPACE" ] || {
  echo "Workspace $WORKSPACE does not exist. Create it with:" >&2
  echo "  git -C $PARENT/app worktree add $WORKSPACE feat/builder-client" >&2
  exit 1
}
WORKSPACE=$(cd "$WORKSPACE" && pwd)
[ -z "$API_DIR" ] || [ -d "$API_DIR" ] || { echo "API checkout $API_DIR does not exist" >&2; exit 1; }
[ -z "$API_DIR" ] || API_DIR=$(cd "$API_DIR" && pwd)

for tool in node npm curl xdg-open setsid; do
  command -v "$tool" >/dev/null 2>&1 || { echo "Missing $tool" >&2; exit 1; }
done
command -v claude >/dev/null 2>&1 || echo "Note: Claude Code (claude) is not on PATH; the agent needs it signed in."

CARGO=$(command -v cargo || echo "$HOME/.cargo/bin/cargo")
[ -x "$CARGO" ] || { echo "Rust (cargo) is needed to build Builder: https://rustup.rs" >&2; exit 1; }
echo
echo "Building voipappz-builder (release) ..."
(cd "$REPO" && "$CARGO" build --release -p builder-cli --quiet)

mkdir -p "$BIN" "$CONFIG_DIR" "$APPS" "$ICONS"
install -m 755 "$REPO/target/release/voipappz-builder" "$BIN/voipappz-builder"
install -m 755 "$REPO/install/voipappz-builder-launch" "$BIN/voipappz-builder-launch"

umask 077
{
  echo "# VoipAppz Builder settings, written by install/install.sh."
  printf 'WORKSPACE=%q\n' "$WORKSPACE"
  printf 'API_DIR=%q\n' "$API_DIR"
  printf 'CLIENT_URL=%q\n' "$CLIENT_URL"
  printf 'PREVIEW_URL=%q\n' "$CLIENT_URL"
  printf 'START_APP=%q\n' "$START_APP"
  echo "PORT=9832"
} > "$CONFIG_DIR/builder.env"
umask 022

ICON_SOURCE="$WORKSPACE/public/images/VA_logo_blue.png"
[ -f "$ICON_SOURCE" ] && install -m 644 "$ICON_SOURCE" "$ICONS/voipappz-builder.png"

cat > "$DESKTOP" <<EOF
[Desktop Entry]
Type=Application
Name=VoipAppz Builder
Comment=Build the VoipAppz app through chat and see it running
Exec=$BIN/voipappz-builder-launch start
Icon=voipappz-builder
Terminal=false
Categories=Development;
Actions=stop;

[Desktop Action stop]
Name=Stop VoipAppz Builder
Exec=$BIN/voipappz-builder-launch stop
EOF
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS" || true

echo
echo "Installed."
echo "  binary    $BIN/voipappz-builder"
echo "  launcher  $BIN/voipappz-builder-launch [start|stop|status|logs]"
echo "  settings  $CONFIG_DIR/builder.env"
echo "  menu      VoipAppz Builder"
echo
echo "Start it from the menu, or run: voipappz-builder-launch"
case ":$PATH:" in *":$BIN:"*) ;; *) echo "Note: add $BIN to your PATH to run it from a terminal." ;; esac
