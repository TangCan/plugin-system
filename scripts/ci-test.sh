#!/usr/bin/env bash
# CI / 本地回归入口（Story 5.3 / NFR7；Story 5.8 / FR41 扩展矩阵）
# 在 plugin-system/ 根目录执行。默认 features 门优先（NFR5），再跑扩展专项。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> fmt check"
cargo fmt --all -- --check

echo "==> clippy (publishable crates, default features, -D warnings)"
cargo clippy -p plugctx -p plugctx-derive -- -D warnings

echo "==> workspace tests (default features)"
cargo test --workspace

echo "==> plugctx core tests"
cargo test -p plugctx

echo "==> trybuild UI suite (≥3 compile_fail / FR39)"
cargo test -p plugctx --test ui

echo "==> core path benches compile only (FR40; no full run)"
cargo bench -p plugctx --bench core_paths --no-run

echo "==> rustdoc gate"
cargo doc -p plugctx --no-deps

echo "==> examples smoke check"
cargo check -p plugctx --examples
cargo check -p plugctx --examples --features async,stages
cargo check -p plugctx-examples --examples
cargo check -p plugctx-examples --examples --features component
cargo check -p plugctx-examples --examples --features wasm
cargo check -p plugctx-examples --examples --features native
cargo check -p plugctx-examples --examples --features web

echo "==> FR41 extension feature matrix (async/parallel/thread-safe/dynamic)"
bash "$ROOT/scripts/ci-extension-matrix.sh"

echo "==> FR37 tracing acceptance (Story 5.4)"
cargo test -p plugctx --features tracing --test acceptance_story_5_4

echo "==> FR51 publish metadata / dry-run (Story 9.1)"
cargo test -p plugctx --test acceptance_story_9_1

echo "==> FR52 empty default / docs.rs subset (Story 9.2)"
cargo test -p plugctx --test acceptance_story_9_2

echo "==> FR53 publish dry-run CI gate (Story 9.3)"
cargo test -p plugctx --test acceptance_story_9_3

echo "==> post-0.1.1 Trusted Publishing workflow (Story 1.1 / FR1)"
cargo test -p plugctx --test acceptance_story_10_1

echo "==> FR54 0.y version / CHANGELOG alignment (Story 9.4)"
cargo test -p plugctx --test acceptance_story_9_4

echo "OK: ci-test.sh finished"
