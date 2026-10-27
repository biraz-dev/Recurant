#!/usr/bin/env sh
# Build the browser version into target/web/.
#
# Needs: rustup target add wasm32-unknown-unknown
#        cargo install wasm-bindgen-cli --version <same as wasm-bindgen in Cargo.lock>
#
# No wasm-opt pass: older binaryen releases (e.g. Ubuntu's 108) break
# wasm-bindgen's reference table ("WebAssembly.Table.grow(): failed to grow").
set -eu
cd "$(dirname "$0")/.."

profile="${PROFILE:-web-release}"
out=target/web

cargo build -p zrpg --target wasm32-unknown-unknown --profile "$profile"

rm -rf "$out"
mkdir -p "$out"
wasm-bindgen --target web --no-typescript --remove-name-section \
  --out-dir "$out" --out-name zrpg \
  "target/wasm32-unknown-unknown/$profile/zrpg.wasm"

cp web/index.html "$out/"
echo "Built $out ($(du -sh "$out" | cut -f1)). Serve it with:"
echo "  python3 -m http.server --directory $out 8000"
