#!/usr/bin/env bash
# FR53：crates.io publish dry-run 门禁。失败必须以非零退出阻断流水线。
# 在 plugin-system/ 根目录执行（或由本脚本 cd 到根）。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> cargo publish --workspace --dry-run (FR53)"
# --allow-dirty：CI/本地工作树常有未提交文件；实际上架前在干净树上去掉该旗标。
cargo publish --workspace --dry-run --allow-dirty

echo "OK: ci-publish-dry-run.sh finished"
