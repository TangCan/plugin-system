---
stepsCompleted:
  - step-01-preflight-and-context
  - step-02-generation-mode
  - step-03-test-strategy
  - step-04-generate-tests
  - step-04c-aggregate
  - step-05-validate-and-complete
lastStep: step-05-validate-and-complete
lastSaved: '2026-08-18'
storyId: '1.1'
storyKey: '1-1-native-插件-dispose-时物理卸载'
storyFile: '_agile-output/implementation-artifacts/1-1-native-插件-dispose-时物理卸载.md'
atddChecklistPath: '_agile-output/test-artifacts/atdd-checklist-1-1-native-插件-dispose-时物理卸载.md'
generatedTestFiles:
  - crates/plugctx/tests/acceptance_story_4_2.rs
detected_stack: backend
generation_mode: ai
pact_mcp_reachable: false
pact_fallback_source: none
---

# ATDD 1.1 Native dispose 物理卸载

## Step 1 — Preflight

- 栈：Cargo 后端（无 Playwright）。Playwright/Pact 工具不适用，不写契约测试。
- 框架：`cargo test` + 既有 `acceptance_story_4_2.rs`（`required-features = ["dynamic-native"]`）。
- 故事已 ready-for-dev，AC 清晰。

## Step 2 — Mode

AI generation（纯后端，无 UI 录制）。

## Step 3 — Strategy

| AC | 场景 | 级别 | 优先级 |
| --- | --- | --- | --- |
| 1 | dispose 后服务撤销且 Library 不再被 ManuallyDrop 按住 | 集成 `acceptance_story_4_2` | P0 |
| 2 | dispose 前 clone 的 NativeInvoker 再 call 返回 Error | 集成 | P0 |
| 3 | 默认 features 测试不启用 dynamic-native | 由现有 `cargo test -p plugctx` 覆盖，不新写 | P1 |
| 4 | 删除 library_mapping_retained 恒 true 断言 | 改写 4_2 + 单元测试 | P0 |

无浏览器 E2E。红灯：当前 `ManuallyDrop` 下 AC1/AC2/AC4 新断言应失败。

## Step 4 — 生成位置

复用 `crates/plugctx/tests/acceptance_story_4_2.rs`（禁止另起 Playwright skip 脚手架）。
