#!/usr/bin/env bash
# Build the Rust core as a universal static library and generate Swift bindings
# into apps/macos/Generated. Xcode runs this as a pre-build step.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/apps/macos/Generated"
PROFILE="${PINWHEEL_PROFILE:-release}"
# Xcode's build environment has a minimal PATH.
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"
# Xcode exports SDK and toolchain variables for its own target; give cargo a clean macOS SDK.
unset MACOSX_DEPLOYMENT_TARGET LD CC CXX 2>/dev/null || true
export SDKROOT="$(xcrun --sdk macosx --show-sdk-path)"

cd "$ROOT"
FLAG=$([ "$PROFILE" = "release" ] && echo "--release" || echo "")

TARGETS=(aarch64-apple-darwin)
if rustup target list --installed 2>/dev/null | grep -q x86_64-apple-darwin; then
  TARGETS+=(x86_64-apple-darwin)
fi

LIBS=()
for t in "${TARGETS[@]}"; do
  MACOSX_DEPLOYMENT_TARGET=15.0 cargo build -p pinwheel-core $FLAG --target "$t"
  LIBS+=("target/$t/$PROFILE/libpinwheel_core.a")
done

mkdir -p "$OUT"
lipo -create "${LIBS[@]}" -output "$OUT/libpinwheel_core.a"

cargo run -q -p uniffi-bindgen -- generate \
  --library "target/${TARGETS[0]}/$PROFILE/libpinwheel_core.a" \
  --language swift --out-dir "$OUT" --no-format
# Xcode finds the C module through a file named module.modulemap.
mv -f "$OUT/pinwheel_coreFFI.modulemap" "$OUT/module.modulemap"
echo "core built for: ${TARGETS[*]}"
