---
id: "2.1"
key: "2-1-native-卸载限度写入用户指南"
status: done
story_num: "2.1"
epic: 2
---

# Story 2.1: native 卸载限度写入用户指南

Status: done

Ultimate context engine analysis completed - comprehensive developer guide created.

## Story

As a 应用作者,
I want 中文指南写清物理卸载的平台限度,
so that 我不会在 `dispose` 成功后仍无法覆盖 `.dll`，或误以为一定能 `dlclose`。

## Acceptance Criteria

1. **Given** 现有 `docs/guide.md` 已写 load → use → dispose → load、无 `reload()`、Windows 可能锁文件  
   **When** 本故事补充卸载限度  
   **Then** 至少写明三点：（1）`FreeLibrary`/`dlclose` 成功 ≠ 文件一定可覆盖；（2）macOS 上带非平凡 TLS 析构的 dylib 可能永不卸载；（3）sound 卸载要求无残留引用、出站函数指针、库内线程（FR4）  
   **And** `docs/requirements/4. 扩展模块设计.md` §4.3 与指南一致，不互相矛盾

2. **Given** Architecture AD-1…AD-3 与 NFR3  
   **When** 本故事改文档  
   **Then** 不新增 `reload()` API，不把 `hot-lib-reloader` 写成生产公开 API  
   **And** 不把 WASM 卸载改成 native 式 `dlclose`（NFR4）

## Tasks / Subtasks

- [x] `docs/guide.md` 写清三点卸载限度；保持 load→dispose→load、无 reload()（AC: #1 #2）
- [x] `docs/requirements/4. 扩展模块设计.md` §4.3 对齐，不矛盾（AC: #1）
- [x] WASM 仍为实例 close/free；验收 `acceptance_story_11_1`

## Dev Notes

- 不要改 `plugctx` 运行时、不要加 `reload()`。
- 测试编号 **11.1**（Epic 2）。
- `hot-lib-reloader` 若提及，须标明开发期工具，不是生产公开 API。

## Dev Agent Record

### Agent Model Used

Composer

### Debug Log References

### Completion Notes List

- 指南与 §4.3 写明：dlclose/FreeLibrary 成功 ≠ 可覆盖文件；macOS TLS 可能永不卸载；sound 卸载无残留引用。无 reload()；WASM 仍 close/free。

### File List

- `docs/guide.md`
- `docs/requirements/4. 扩展模块设计.md`
- `docs/testing.md`
- `scripts/ci-test.sh`
- `crates/plugctx/tests/acceptance_story_11_1.rs`
- `_agile-output/implementation-artifacts/2-1-native-卸载限度写入用户指南.md`
- `_agile-output/implementation-artifacts/sprint-status.yaml`
