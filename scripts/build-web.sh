#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs |
    sh -s -- -y --profile minimal --no-modify-path
  export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
fi
rustup target add wasm32-unknown-unknown
cargo build --locked --release --lib --target wasm32-unknown-unknown
mkdir -p dist
cp web/index.html web/style.css web/app.js web/board.js web/battle.js web/collection.js web/opponents.webp web/opponents-worried.webp web/opponents-laugh.webp web/start.webp web/icon.png web/_headers dist/
mkdir -p dist/coins
cp web/coins/catalog.tsv web/coins/atlas-{0..9}.webp dist/coins/
cp "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/hundred_math.wasm" dist/
