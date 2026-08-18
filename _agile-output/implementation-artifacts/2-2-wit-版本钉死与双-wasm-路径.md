---
id: "2.2"
key: "2-2-wit-版本钉死与双-wasm-路径"
status: done
story_num: "2.2"
epic: 2
---

# Story 2.2: WIT 版本钉死与双 WASM 路径

Status: done

Ultimate context engine analysis completed - comprehensive developer guide created.

## Story

As a 使用 Component Model 适配器的宿主作者,
I want 文档钉死**当前工具链实际使用的 WIT / wasmtime / wit-bindgen 版本**,
so that 我不会把样例 guest 提前钉到已发布的 `wasi@0.3.0` 而导致实例化失败。

## Acceptance Criteria

1. **Given** 现有 `docs/component-model-versions.md` 钉 wasmtime 47.x、wit-bindgen 0.60.x、`wasm32-wasip2`  
   **When** 本故事对照当前 workspace / 客人工具链复核  
   **Then** 文档写明**实际** WIT/WASI pin（与 `guests/wit-sample/` 及检入 `.wasm` 一致）；在 wasmtime / wit-bindgen 刷新到已发布 `wasi@0.3.0` 之前，**禁止**把样例 guest 改钉该发布标签（FR5）  
   **And** `guests/wit-sample/` README（或等价）含同一 pin 与「不要提前改钉」的说明

2. **Given** Extism 字节 ABI 与 Wasmtime Component Model 不能互相加载  
   **When** 文档描述两条动态 WASM 路径  
   **Then** 保持 `dynamic-wasm` 与 `dynamic-wasm-component` 分 feature、分制品；禁止暗示一份 `.wasm` 两吃（FR5）  
   **And** WASM 卸载仍为实例 `close`/`free` 或 Component 路径 Drop Store，不写成 native `dlclose`（NFR4）

3. **Given** NFR1  
   **When** 本故事完成  
   **Then** 不把 `extism` / `wasmtime` 拉进 `plugctx` 默认依赖图；不把 Fidius 式签名包做成补丁

## Tasks / Subtasks

- [x] `docs/component-model-versions.md` 写明实际 WIT pin；禁止提前 `wasi@0.3.0`（AC: #1）
- [x] `guests/wit-sample/README.md` 同一 pin + 不要提前改钉（AC: #1）
- [x] 双路径分 feature 分制品；WASM 卸载非 dlclose；不改 default 依赖图（AC: #2 #3）
- [x] 验收 `acceptance_story_11_2`

## Dev Notes

- 客人 WIT：`package plugctx:sample@0.1.0`（`wit/world.wit`），**无** `wasi@0.3.0` import。目标 `wasm32-wasip2`，`wit-bindgen = "0.60"`。
- 工作区 `wasmtime = "47"`。不要升级依赖或重建 wasm，除非 pin 与检入制品不一致（当前一致则只改文档）。
- 测试编号 **11.2**。

## Dev Agent Record

### Agent Model Used

Composer

### Debug Log References

### Completion Notes List

- 实际 pin：`plugctx:sample@0.1.0` + wit-bindgen 0.60 + wasmtime 47 + wasm32-wasip2。禁止提前 `wasi@0.3.0`。Extism / CM 仍分制品。未改 default 依赖、未做 Fidius。

### File List

- `docs/component-model-versions.md`
- `guests/wit-sample/README.md`
- `docs/testing.md`
- `scripts/ci-test.sh`
- `crates/plugctx/tests/acceptance_story_11_2.rs`
- `_agile-output/implementation-artifacts/2-2-wit-版本钉死与双-wasm-路径.md`
- `_agile-output/implementation-artifacts/sprint-status.yaml`
