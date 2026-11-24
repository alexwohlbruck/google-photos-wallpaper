#!/usr/bin/env bash
# Build the macOS app (Rust core included) and launch it, replacing any running copy.
#
#   scripts/run-macos.sh             Debug build, your real library
#   scripts/run-macos.sh --release   Release build
#   scripts/run-macos.sh --scratch   Throwaway library in /tmp, never touches your desktop
#   scripts/run-macos.sh --logs      Stay attached and print the app's output
#
# Flags combine, e.g. --release --scratch.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CONFIG=Debug
SCRATCH=0
LOGS=0
for arg in "$@"; do
  case "$arg" in
    --release) CONFIG=Release ;;
    --scratch) SCRATCH=1 ;;
    --logs) LOGS=1 ;;
    -h|--help) sed -n '2,9p' "$0"; exit 0 ;;
    *) echo "unknown option: $arg" >&2; exit 1 ;;
  esac
done

cd "$ROOT/apps/macos"
command -v xcodegen >/dev/null || { echo "Install xcodegen first: brew install xcodegen" >&2; exit 1; }
# xcodegen needs the generated bindings to exist; on a fresh checkout, build the core once first.
if [ ! -f Generated/pinwheel_core.swift ]; then
  echo "Building Rust core…"
  PINWHEEL_PROFILE=debug "$ROOT/scripts/build-core-macos.sh" >/dev/null
fi
xcodegen generate --quiet

echo "Building ($CONFIG)…"
LOG="$(mktemp -t pinwheel-build)"
if ! xcodebuild -project Pinwheel.xcodeproj -scheme Pinwheel \
    -configuration "$CONFIG" -derivedDataPath build build >"$LOG" 2>&1; then
  grep -E "error:|warning: unused|FAILED" "$LOG" | head -30
  echo "Build failed. Full log: $LOG" >&2
  exit 1
fi
rm -f "$LOG"

APP="$PWD/build/Build/Products/$CONFIG/Pinwheel.app"
BIN="$APP/Contents/MacOS/Pinwheel"

# Quit any running copy (Debug or Release, including builds from before the rename) so the new build owns the menu bar icon.
pkill -f "(Pinwheel|Google Photos Wallpaper).app/Contents/MacOS" 2>/dev/null && sleep 1 || true

ARGS=()
if [ "$SCRATCH" = 1 ]; then
  export PINWHEEL_DATA_DIR="${PINWHEEL_DATA_DIR:-/tmp/pinwheel-scratch}"
  export PINWHEEL_DRY_RUN=1
  echo "Scratch library: $PINWHEEL_DATA_DIR (wallpaper changes are simulated)"
fi
ARGS+=(--show-library)  # open the main window

if [ "$LOGS" = 1 ]; then
  echo "Running. Ctrl-C quits the app."
  exec "$BIN" ${ARGS[@]+"${ARGS[@]}"}
else
  # `open` doesn't pass env vars through, so launch the binary directly and detach.
  nohup "$BIN" ${ARGS[@]+"${ARGS[@]}"} >/dev/null 2>&1 &
  echo "Launched $CONFIG build."
fi
