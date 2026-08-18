---
id: "1.3"
key: "1-3-文档废止旧-fr25-native-逻辑卸载"
status: done
epic: 1
story: 3
created: "2026-08-18"
---

# Story 1.3: 文档废止旧 FR25

Status: done

## Story

As a 框架使用者,
I want 文档与 CHANGELOG 写明 native 卸载会 `dlclose`,
so that 我不会按「映射一直保留」来设计热插拔。

## Acceptance Criteria

- [x] README、feature-matrix、requirements §4.3、testing、CHANGELOG 改为物理卸载；WASM FR26 仍为实例 close
- [x] CHANGELOG Unreleased 记 `dynamic-native` breaking；默认同步内核不标 breaking；不 bump `version` 到 0.2.0
- [x] 引言动态加载与物理卸载一致

## Tasks / Subtasks

- [x] 改用户可见文档
- [x] 护栏测试 `user_docs_state_native_physical_unload`
- [x] 更新 AGENTS.md（去掉 FR25 滞后坑）

## Dev Agent Record

### Agent Model Used

Composer (Cursor Auto)

### File List

- `README.md`, `CHANGELOG.md`, `docs/feature-matrix.md`, `docs/testing.md`
- `docs/requirements/1. 引言.md`, `2. 总体架构.md`, `4. 扩展模块设计.md`, `7. 技术选型与依赖.md`
- `AGENTS.md`
- `crates/plugctx/tests/acceptance_story_4_2.rs`
