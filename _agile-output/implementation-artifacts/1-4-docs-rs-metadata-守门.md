---
id: "1.4"
key: "1-4-docs-rs-metadata-守门"
status: done
story_num: "1.4"
epic: 1
---

# Story 1.4: docs.rs metadata 守门

Status: done

Ultimate context engine analysis completed - comprehensive developer guide created.

## Story

As a crate 使用者,
I want docs.rs 按可同时打开的轻量 feature 构建文档,
so that 文档构建不会因 `all-features` 拉进 native/wasm 运行时而失败。

## Acceptance Criteria

1. **Given** `plugctx` 已有 `[package.metadata.docs.rs]` 轻量子集 `async,parallel,thread-safe,tracing,stages`（不含 native/wasm/component）  
   **When** 本故事验收既有 `acceptance_story_9_2`（或等价护栏）  
   **Then** 该子集仍在，且规范化后不得出现 `all-features=true`（FR7）

2. **Given** `plugctx-derive` 当前无 `[package.metadata.docs.rs]`  
   **When** 需要 docs.rs 定制或为与核心 crate 对齐而补表  
   **Then** 使用 `[package.metadata.docs.rs]`；**不得**设 `all-features = true`（FR7）  
   **And** 若 derive 无需额外 features，可只显式禁止 all-features 或保持默认（无 all-features 旗标），并在 `docs/publishing.md` 一句说明两公开 crate 的 docs.rs 约定

3. **Given** NFR1 空 default  
   **When** 本故事改 Cargo.toml metadata  
   **Then** 不得把 `libloading` / `extism` / `wasmtime` 拉进 `plugctx` 默认依赖图

## Tasks / Subtasks

- [x] 保持 `plugctx` docs.rs 轻量子集；`acceptance_story_9_2` 仍绿（AC: #1 #3）
- [x] `plugctx-derive` 增加 `[package.metadata.docs.rs]`，无 `all-features = true`（AC: #2）
- [x] `docs/publishing.md` 一句两 crate 约定；验收 `acceptance_story_10_4`

## Dev Notes

- 不要改 `default = []`，不要给 derive 加假 feature。
- 测试编号 **10.4**。既有 9_2 继续守 plugctx 子集与 cargo doc。

## Dev Agent Record

### Agent Model Used

Composer

### Debug Log References

### Completion Notes List

- derive 补 `[package.metadata.docs.rs]`（仅 targets）；两 crate 均无 `all-features = true`。plugctx 轻量子集未改。9_2 / 10_4 绿。

### File List

- `crates/plugctx-derive/Cargo.toml`
- `docs/publishing.md`
- `docs/testing.md`
- `scripts/ci-test.sh`
- `crates/plugctx/tests/acceptance_story_10_4.rs`
- `_agile-output/implementation-artifacts/1-4-docs-rs-metadata-守门.md`
- `_agile-output/implementation-artifacts/sprint-status.yaml`
