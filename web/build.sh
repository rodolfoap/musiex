#!/bin/sh
# Builds the practice screen into web/: the WebAssembly core goes to web/pkg.
# Needs: rustup target add wasm32-unknown-unknown
#        cargo install wasm-bindgen-cli --version 0.2.129
set -eu
cd "$(dirname "$0")/.."
cargo build --release --target wasm32-unknown-unknown -p musiex-web
wasm-bindgen --target web --no-typescript --out-dir web/pkg \
  target/wasm32-unknown-unknown/release/musiex_web.wasm
echo "Built web/. Serve it with: python3 -m http.server -d web 8000"
