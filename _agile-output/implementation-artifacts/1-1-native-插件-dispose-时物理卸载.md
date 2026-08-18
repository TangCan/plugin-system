---
id: "1.1"
key: "1-1-native-插件-dispose-时物理卸载"
status: done
epic: 1
story: 1
created: "2026-08-18"
---

# Story 1.1: Native 插件 dispose 时物理卸载

Status: done

<!-- Ultimate context engine analysis completed - comprehensive developer guide created -->

## Story

As a 插件宿主开发者,
I want native 插件在精确卸载后释放动态库映射,
so that 旧代码不能再被调用，并为换新 `.so` 腾出进程。

## Acceptance Criteria

1. **Given** 已用 `dynamic-native` 加载并安装 `hello_plugin`，且 `start` 成功  
   **When** 对该插件 `PluginHandle::dispose`  
   **Then** 对应 `libloading::Library` 被 Drop（平台 `dlclose` / `FreeLibrary`）  
   **And** 撤销 Context 注册与 Effect 仍先于 Drop 发生（逻辑卸载顺序不变）

2. **Given** dispose 已成功  
   **When** 使用卸载前拿到的 `NativeInvoker`（或等价句柄）再 `call`  
   **Then** 返回可诊断 `Error`，进程不以成功路径完成该次调用  
   **And** 不以 panic 作为控制流（NFR3）

3. **Given** 仅启用默认 features  
   **When** 运行 `cargo test -p plugctx`  
   **Then** 不链接 `libloading`（NFR1）

4. **Given** 旧验收曾断言「保留 Library 映射 / 不以 dlclose 为前提」  
   **When** 本故事完成  
   **Then** 那些断言已删除或改写为物理卸载契约（FR1/FR2）

## Tasks / Subtasks

- [x] 去掉 `ManuallyDrop<Library>`，dispose/Drop 路径 Drop `Library`（AC: #1）
  - [x] `NativeState` 在实例 `destroy` 之后 `take`/`Drop` `Library`（即使仍有 `NativeInvoker` 的 `Arc` 克隆）
  - [x] 逻辑卸载顺序：撤销 provide/on/effect → `vtable.destroy` → Drop `Library`
  - [x] `Context::dispose` 级联卸掉该插件时走同一路径
- [x] 失效 Invoker 不可调用（AC: #2）
  - [x] 先置失效标志，再 destroy/dlclose，禁止再跳进旧 vtable
  - [x] `call` 返回可 `match` 的 `Error`（优先现有 `NativeCall` / `PluginAlreadyDisposed`，无必要时不新增变体）
- [x] 改写旧 FR25 验收（AC: #4）
  - [x] 改 `tests/acceptance_story_4_2.rs`：`dispose_is_logical_unload_not_dlclose` → 物理卸载契约
  - [x] 删除或改写 `NativePlugin::library_mapping_retained` 与单元测试 `library_mapping_retained_is_true`
  - [x] 新增：dispose 前 clone `NativeInvoker`，dispose 后再 `call` 失败
- [x] 默认 features 不链接 `libloading`（AC: #3）
  - [x] 不改 `default = []`；本故事代码仅在 `dynamic-native` 下编译

## Dev Notes

### 当前实现（必须改）

[`crates/plugctx/src/dynamic_native.rs`](crates/plugctx/src/dynamic_native.rs)：

- `NativeState._library: ManuallyDrop<Library>`，`Drop` 只 `destroy_instance()`，**不** `Library::drop`。
- `NativePlugin::library_mapping_retained()` **恒为 `true`**。
- `build()` 里 `provide(NativeInvoker)` + Effect 只调 `destroy_instance`。
- `NativeInvoker` 持 `Arc<NativeState>`。若只在 `NativePlugin` Drop 时释放 `Library`，clone 出的 Invoker 会把 `Library` 按住 → **不满足 AC#1**。必须在 `destroy_instance`（Effect / Drop）里从 state **取出并丢弃** `Library`。

建议形状（实现可微调，语义不可变）：

```rust
library: Mutex<Option<Library>>, // std::sync::Mutex，勿为这点去拉 parking_lot
```

`destroy_instance`: CAS `destroyed` → `vtable.destroy` → `library.lock().take()`。`call` 开头若 `destroyed` 则返回 `Error`，**不要**再调 vtable。

### 架构约束（必须遵守）

[Source: `_agile-output/planning-artifacts/architecture.md`]

- **AD-1**：插件 scope 卸载完成后必须 Drop `Library`，禁止 `ManuallyDrop` 泄漏。WASM 路径不动。
- **AD-2**：物理卸载后 `NativeInvoker::call` 必须 `Error`，禁止 use-after-unmap 当成功。
- **AD-3**：**不要**新增 `reload()`。热插拔是 load → use → dispose → load（那是 Story 1.2）。
- **AD-4**：本故事只改 `dynamic-native` 卸载语义；默认 features 测试不得依赖 `libloading`。
- 热插拔错误走已有 `Native*` / `AlreadyDisposed`，或新增**一个**可匹配变体；不要恢复 TypeId 载荷大重构。[Source: `docs/api-freeze.md`]
- `plugctx` `default = []`；`libloading` 仅 `dep:` / `dynamic-native`。[Source: `crates/plugctx/Cargo.toml`]

### 不要做（范围）

- 不改 README / feature-matrix / requirements 用户文档（Story 1.3）。
- 不实现换 `.so` 再加载验收（Story 1.2），但本故事的 Drop 必须让 1.2 成为可能。
- 不改 WASM `dynamic-wasm` / FR26。
- 不把 `extism`/`wasmtime` 拉进默认图。
- 不要用 `cargo test --all-features` 当唯一门禁。

### 测试

- 回归入口：`./scripts/ci-test.sh`；本故事专项：
  - `cargo test -p plugctx --features dynamic-native --test acceptance_story_4_2`
  - `cargo test -p plugctx`（默认，确认无 libloading）
- 现有 `example_lib()` 辅助：先 `cargo build -p hello_plugin -p echo_plugin`。
- 平台库名：Linux `libhello_plugin.so`，macOS `libhello_plugin.dylib`，Windows `hello_plugin.dll`。
- 可观测性：dispose 后 `ctx.get::<NativeInvoker>()` 为 `None`；clone 出的 invoker `call` 失败。不必扫 `/proc/self/maps`。

### Project Structure Notes

- 只改 `crates/plugctx` 的 `dynamic-native` 模块与其验收测试。
- `plugin-host` / hello / echo 脚手架本故事不改。
- WIT guest `guests/wit-sample/` 不是 workspace member，勿碰。

### References

- [Source: `_agile-output/planning-artifacts/epics.md` — Story 1.1]
- [Source: `_agile-output/planning-artifacts/architecture.md` — AD-1, AD-2, AD-3]
- [Source: `_agile-output/planning-artifacts/prd.md` — FR-1 native 物理卸载]
- [Source: `docs/api-freeze.md` — Error 单元变体；`get` 返回 Option]
- [Source: `AGENTS.md` — 以 architecture 为准，勿再断言 FR25]
- [Source: `crates/plugctx/tests/acceptance_story_4_2.rs` — 现有逻辑卸载断言]
- [Source: `crates/plugctx/src/dynamic_native.rs` — ManuallyDrop 现状]

## Dev Agent Record

### Agent Model Used

Composer (Cursor Auto)

### Debug Log References

- `cargo test -p plugctx --features dynamic-native --test acceptance_story_4_2`
- `cargo test -p plugctx`

### Completion Notes List

- `NativeState.library` 改为 `Mutex<Option<Library>>`；`destroy_instance` 在持锁下 CAS、`vtable.destroy`、再 `take` Drop。
- `call` 持同一把锁，避免与 `dlclose` 并发。
- 未新增 `reload()`；未改默认 features；未改用户文档（1.3）。
- 验收改写 `acceptance_story_4_2.rs`；删除 `library_mapping_retained`。

### File List

- `crates/plugctx/src/dynamic_native.rs`
- `crates/plugctx/Cargo.toml`
- `crates/plugctx/tests/acceptance_story_4_2.rs`
- `_agile-output/implementation-artifacts/1-1-native-插件-dispose-时物理卸载.md`
- `_agile-output/test-artifacts/atdd-checklist-1-1-native-插件-dispose-时物理卸载.md`
