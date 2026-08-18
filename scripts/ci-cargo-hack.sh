#!/usr/bin/env bash
# cargo-hack 互斥 feature 门禁（Story 1.2 / FR3）
# thread-safe 与空 default（默认同步验收）互斥；禁止隐式 --all-features。
# 重运行时 feature 仍由 ci-extension-matrix.sh 覆盖。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! cargo hack -V >/dev/null 2>&1; then
  echo "==> installing cargo-hack (required for FR3)"
  cargo install cargo-hack --locked
fi

echo "==> cargo hack: feature powerset (depth 1), thread-safe mutually exclusive with default, exclude-all-features"
# 用 check 而非 test：`tests/ui/start_async_requires_feature.rs` 在启用 async 时会从 compile_fail 变成通过。
# 互斥与编译面由 hack 守门；各 feature 的验收仍走 ci-test.sh / ci-extension-matrix.sh。
# --mutually-exclusive-features 仅适用于 --feature-powerset，且至少两个名字。
# default=[] 即默认同步验收；与 thread-safe 互斥。async+thread-safe 等组合仍合法（depth 1 各自单跑）。
# --depth 1 等价于 --each-feature，避免 2^n 拖垮 CI。
cargo hack check -p plugctx \
  --feature-powerset \
  --depth 1 \
  --exclude-features dynamic-native,dynamic-wasm,dynamic-wasm-component \
  --mutually-exclusive-features thread-safe,default \
  --exclude-all-features

echo "OK: ci-cargo-hack.sh finished (FR3)"
