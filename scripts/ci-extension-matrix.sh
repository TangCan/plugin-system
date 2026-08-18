#!/usr/bin/env bash
# 扩展模块专项测试矩阵（Story 5.8 / FR41 / 设计 §8.7）
# 在 plugin-system/ 根目录执行。各 feature 显式启用；不污染默认同步门（NFR5）。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> FR41 extension matrix: async (3.1)"
cargo test -p plugctx --features async --test acceptance_story_3_1

echo "==> FR41 extension matrix: parallel (3.2)"
cargo test -p plugctx --features parallel --test acceptance_story_3_2

echo "==> FR41 extension matrix: thread-safe (4.1)"
cargo test -p plugctx --features thread-safe --test acceptance_story_4_1

echo "==> FR41 extension matrix: build native plugins for dynamic-native"
cargo build -p hello_plugin -p echo_plugin

echo "==> FR41 extension matrix: dynamic-native (4.2)"
cargo test -p plugctx --features dynamic-native --test acceptance_story_4_2

echo "==> FR41 extension matrix: dynamic-wasm (4.3)"
cargo test -p plugctx --features dynamic-wasm --test acceptance_story_4_3

echo "==> FR41 extension matrix: dynamic-native+dynamic-wasm (4.4 / 4.5)"
cargo test -p plugctx --features "dynamic-native,dynamic-wasm" --test acceptance_story_4_4
cargo test -p plugctx --features "dynamic-native,dynamic-wasm" --test acceptance_story_4_5

echo "==> FR47 extension matrix: dynamic-wasm-component (8.1)"
cargo test -p plugctx --features dynamic-wasm-component --test acceptance_story_8_1

echo "==> FR48 extension matrix: PluginBackend dual path (8.2)"
cargo test -p plugctx --features "dynamic-wasm,dynamic-wasm-component" --test acceptance_story_8_2

echo "==> FR49 extension matrix: one Store one instance Drop (8.3)"
cargo test -p plugctx --features dynamic-wasm-component --test acceptance_story_8_3

echo "==> FR50 extension matrix: WIT world + wasip2 sample guest (8.4)"
cargo test -p plugctx --features dynamic-wasm-component --test acceptance_story_8_4

echo "OK: ci-extension-matrix.sh finished (FR41)"
