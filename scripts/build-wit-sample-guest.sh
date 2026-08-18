#!/usr/bin/env bash
# 重建 FR50 wasip2 WIT 样例客人，并同步检入制品到 plugctx/testdata。
# 要求：rustup target add wasm32-wasip2；可选 wasm-tools（用于打印 WIT）。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GUEST="$ROOT/guests/wit-sample"
OUT="$ROOT/crates/plugctx/testdata/wit_sample_add.wasm"

cd "$GUEST"
cargo build --target wasm32-wasip2 --release
cp -f target/wasm32-wasip2/release/wit_sample_guest.wasm "$OUT"
echo "Wrote $OUT ($(wc -c < "$OUT") bytes)"

if command -v wasm-tools >/dev/null 2>&1; then
  echo "==> embedded WIT:"
  wasm-tools component wit "$OUT"
fi

echo "OK: rebuild wit-sample-guest → testdata/wit_sample_add.wasm"
