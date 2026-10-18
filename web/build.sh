#!/bin/sh
# Builds the practice screen into web/: the WebAssembly core goes to web/pkg.
# Needs: rustup target add wasm32-unknown-unknown
#        cargo install wasm-bindgen-cli --version 0.2.129
set -eu
cd "$(dirname "$0")/.."
cargo build --release --target wasm32-unknown-unknown -p musiex-web
wasm-bindgen --target web --no-typescript --out-dir web/pkg \
  target/wasm32-unknown-unknown/release/musiex_web.wasm
# The offline cache is named after the build, so installed copies update.
version=$(cat web/index.html web/style.css web/app.js web/manifest.webmanifest \
  web/icon.svg web/icon-192.png web/icon-512.png web/pkg/musiex_web.js \
  web/pkg/musiex_web_bg.wasm | sha256sum | cut -c1-16)
echo "const VERSION = '$version';" > web/pkg/version.js
echo "Built web/. Serve it with: python3 -m http.server -d web 8000"
