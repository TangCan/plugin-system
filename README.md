# plugin-system

Rust 插件系统工作区：既有 **C ABI `cdylib` 脚手架**（`plugin-api` / `plugin-host`），以及演进中的进程内插件框架核心库 **`plugctx`**。二者**并存演进**——`plugctx` 不立即替换 ABI 脚手架；动态加载能力按 feature 渐进接入同一 `Context` 生命周期。

> crates.io：**0.1.0 已上架** — [`plugctx`](https://crates.io/crates/plugctx) / [`plugctx-derive`](https://crates.io/crates/plugctx-derive)。曾用名 `pluggable` 已被无关方占用，勿与 crates.io 上其他 `pluggable` 混淆。

```bash
cargo add plugctx
cargo add plugctx-derive   # 可选过程宏
```

## 布局

```
plugin-system/
├── crates/plugctx         # 新核心：同步插件框架（Context/DI/事件等，按故事迭代）
│   └── examples/            # 包内示例：combo / async-start / stages-lifecycle
├── crates/plugctx-derive  # 可选过程宏：`#[derive(Plugin)]`（FR27；核心不依赖）
├── examples/                # 工作区演示包 plugctx-examples（publish=false；derive/wasm/component）
├── crates/plugin-api        # ABI 常量、PluginVTable、缓冲分配
├── crates/plugin-host       # libloading 加载器 + CLI
└── crates/plugins/
    ├── hello                # greet 示例
    └── echo                 # echo 示例
```

`plugctx-derive` 为**可选**独立 crate：用 `#[derive(Plugin)]` + `#[plugin(depends(...))]` 生成 `dependencies()`，并将 `build` 委托到 `on_build`。仅依赖 `plugctx`、不引入 derive 时，手写 `impl Plugin` 仍完全可用。

## 发布切片（FR42 / FR54）

- **CHANGELOG**（0.1.0 核心 / 0.2.0 扩展**能力清单**；≠ 强制 `version = "0.2.0"`）：[`CHANGELOG.md`](CHANGELOG.md)
- **Feature 矩阵**（对齐设计 §2.4 / §7.3，含刻意偏离）：[`docs/feature-matrix.md`](docs/feature-matrix.md)
- **0.y / 锁步 / crates.io 改名**：[`docs/publishing.md`](docs/publishing.md)

## crates.io 发布边界（FR51–FR53）

- 已上架：`plugctx`、`plugctx-derive` **0.1.0**（license / description / documentation；repository 见 [`docs/publishing.md`](docs/publishing.md)）
- 不可发布（`publish = false`）：`plugin-api`、`plugin-host`、示例插件、WASM/WIT fixtures
- CI dry-run 门禁：`./scripts/ci-publish-dry-run.sh`（接入 `ci-test.sh`；失败阻断）
- 后续发版（锁步 bump、先 `plugctx` 后 `plugctx-derive`、token / trusted publishing / 速率限制）：见 [`docs/publishing.md`](docs/publishing.md)

详情与必填字段清单：[`docs/publishing.md`](docs/publishing.md)。

## Feature 矩阵

完整对照与偏离说明见 [`docs/feature-matrix.md`](docs/feature-matrix.md)。摘要：

| Feature | 能力 | 默认依赖图 | 说明 |
| --- | --- | --- | --- |
| （默认） | 进程内同步 `Plugin` / DI / 事件 / Effect | 无额外依赖 | `start` / `dispose` 核心路径 |
| `async` | `AsyncPlugin` + `start_async` | `async-trait` / `futures` | 不绑定具体运行时 |
| `parallel` | `emit_parallel` 宿主侧 fan-out | 隐含 `async` | 不假定 guest 多线程 |
| `thread-safe` | `Send+Sync` Context 存储 | `parking_lot` | 持 `ServiceRef` 时勿重入写路径 |
| `dynamic-native` | C ABI + `libloading` | `libloading`（包内 `c_abi`） | dispose 后 **Drop `Library`（`dlclose`）**；热插拔 = load → dispose → load；脚手架 `plugin-api` 同源且 `publish = false` |
| `dynamic-wasm` | WASM 适配器（Extism） | 可选 `extism` | 实例显式 `close`/`free` |
| `dynamic-wasm-component` | `wasmtime::component` 嵌入 | 可选 `wasmtime` **47.x** | 与 Extism 经 `PluginBackend` 分路径 / **分制品**（FR48）；见 [`docs/component-model-versions.md`](docs/component-model-versions.md) |
| `tracing` | build / emit / dispose 诊断 span | 仅 `tracing` 门面 | **非默认**；不引入 `tracing-subscriber`（应用侧自选后端） |
| `stages` | `InitEvent` / `PostStartEvent` / `PreDisposeEvent` | 无额外依赖 | **非默认**；核心仍保证 Ready/Dispose（FR32 / §4.7） |

启用诊断（示例）：

```bash
cargo test -p plugctx --features tracing --test acceptance_story_5_4
```

启用扩展生命周期阶段：

```bash
cargo test -p plugctx --features stages --test acceptance_story_6_1
```

应用侧自行安装订阅端，例如：

```rust
// 依赖：tracing-subscriber（由应用引入，非 plugctx 默认依赖）
tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();
```

### 动态扩展细节

| Feature | 路径 | 卸载语义 | ABI 协商 |
| --- | --- | --- | --- |
| （默认） | 进程内 `Plugin` | `PluginHandle::dispose` / Context dispose | 无跨边界 ABI |
| `dynamic-native` | C ABI + `libloading` | 先撤销注册与 Effect，再 **Drop `Library`（`dlclose`）** | `PLUGIN_ABI_VERSION`（vtable） |
| `dynamic-wasm` | WASM 适配器（Extism） | **实例显式 `close`/`free`**（FR26） | `WASM_ABI_VERSION`（custom section / `abi_override`） |
| `dynamic-wasm-component` | `wasmtime::component` | 销毁=Drop Store（FR49，`store_drop_count`）；dispose Effect 触发 | 最小组件导出；WIT wasip2 样例（FR50）；版本矩阵 NFR12 |

动态适配器（`NativePlugin` / `WasmPlugin` / `ComponentPlugin`）实现同一 `Plugin` trait，经 `Context::plugin` / `start` / `PluginHandle::dispose` 与进程内插件对齐（可混合安装、依赖排序与事件共存）。

统一扩展入口：`DynamicLoader`（与 Interceptor / AsyncPlugin 同级）+ `DylibLoader` / `WasmLoader` / `ComponentLoader`；WASM 双后端另有 `PluginBackend`（`Extism` | `Component`）。`load(DynamicSource)` → `Box<dyn Plugin>`，可直接 `ctx.plugin(...)`。失败返回可诊断错误，不半初始化 Context。便捷函数 `load_native_plugin` / `load_wasm_plugin` / `load_wasm_component` 仍可用。

**分制品（FR48）**：Extism PDK `.wasm` 与 Component Model 组件**二进制不兼容**。同一宿主可同时启用两 feature 并在同一 `Context` 安装两种插件，但客人须分别编译（或另建适配层）；**禁止**暗示一份 `.wasm` 两吃。

### ABI 与工具链锁定（NFR6）

- **native**：宿主与插件须同工具链、同 `plugin-api`；布局/语义不兼容时递增 `PLUGIN_ABI_VERSION`。加载前协商，不匹配返回 `Error::AbiMismatch`，**不**执行 `create`/`init`。
- **wasm**：宿主 `WASM_ABI_VERSION` 与制品声明（custom section `plugctx.abi` / `abi_override`；缺省视为与宿主相同）协商；不匹配返回 `Error::WasmAbiMismatch`，不实例化。
- **不以 `abi_stable` 为基线**；禁止跨 DSO 传递不稳定 `dyn Trait`。

## 生命周期与常见错误

典型流程：`Context::new` → `plugin(...)`（可多次）→ `start` → 业务（`get` / `emit` / …）→ `dispose`。

| API | 语义 |
| --- | --- |
| `start` | 按依赖序构建插件；成功后触发 `ReadyEvent`；失败不进入 Started。启用 `stages` 时：`InitEvent` → 构建 → `ReadyEvent` → `PostStartEvent` |
| `dispose` | 触发 `DisposeEvent` → effect **逆序** cleanup → 级联子上下文；**幂等**。启用 `stages` 时：`PreDisposeEvent` → `DisposeEvent` → … |

未启用 `stages` 时，扩展阶段事件类型不可用；Ready/Dispose 行为不变。

| 错误 | 典型场景 |
| --- | --- |
| `AlreadyStarted` | 重复 `start` |
| `AlreadyDisposed` | 销毁后再 `start` / `plugin` / `isolate`，或 Context 已毁后 `PluginHandle::dispose` |
| `MissingDependency` | 插件 `dependencies` 所需服务未被 `provide` |
| `CircularDependency` | 依赖成环，**或**乐观构建 ≥2 个插件同时无法进展（各自缺不同依赖也属此变体） |
| `BuildFailed` | 插件 `build` 返回失败 |
| `PluginAlreadyDisposed` | 上下文仍存活时插件句柄再次 `dispose`（与 `AlreadyDisposed` 区分：后者是 Context 级） |
| `ServiceNotFound` | 预留（`get`/`get_trait` 仍返回 `Option`） |

核心 API / Error 与设计 §6 的冻结对照（含偏差说明）：[`docs/api-freeze.md`](docs/api-freeze.md)。  
销毁窗口内 `provide`/`on`/`effect` 何时允许：[`docs/dispose-registration-window.md`](docs/dispose-registration-window.md)。  
`ContextData` 拆分评估（推迟）：[`docs/adr-contextdata-split.md`](docs/adr-contextdata-split.md)。  
更完整的 API 说明：`cargo doc -p plugctx --no-deps --open`。

## 测试金字塔与回归门禁

关键路径（生命周期 / DI 错误 / 事件重入 / effect 逆序 / isolate 级联）已由 `acceptance_story_1_2`…`1_7` 覆盖；层次与映射见 [`docs/testing.md`](docs/testing.md)。属性测试（proptest 随机安装/卸载/emit 序列，FR38）见 `acceptance_story_5_5`。核心路径基准（get/emit/start，FR40，**已交付**）见 `cargo bench -p plugctx --bench core_paths`。扩展模块专项（async/parallel/thread-safe/dynamic，FR41，**已交付**）见 `./scripts/ci-extension-matrix.sh`。

```bash
cd plugin-system
# 一键回归：先 rustfmt，再 ci-test.sh（fmt check + clippy + 默认门 + trybuild + bench 编译 + rustdoc + FR41）
just test
./scripts/ci-test.sh              # 不先 fmt 写回，仅检查 + 测试
./scripts/ci-extension-matrix.sh   # 仅 FR41 扩展矩阵

cargo test -p plugctx
cargo test -p plugctx --test acceptance_story_5_5   # proptest 属性测试（Story 5.5）
cargo test -p plugctx --test ui          # trybuild ≥3 compile_fail（Story 5.6 / FR39，已交付）
cargo test -p plugctx --test acceptance_story_5_6   # trybuild 套件护栏
cargo test -p plugctx --test acceptance_story_5_7   # 核心路径 bench 护栏（Story 5.7 / FR40）
cargo test -p plugctx --test acceptance_story_5_8   # 扩展矩阵护栏（Story 5.8 / FR41）
cargo test -p plugctx --test acceptance_story_6_2   # Error/API 冻结（Story 6.2 / FR35）
cargo test -p plugctx --test acceptance_story_6_3   # 0.1/0.2 发布切片（Story 6.3 / FR42）
# 基准（本地满量；默认 CI 仅 --no-run）
cargo bench -p plugctx --bench core_paths
cargo bench -p plugctx --bench core_paths -- --save-baseline main   # 记录基线
cargo doc -p plugctx --no-deps
```

## 可运行示例

索引与命令见下表。设计说明：[`examples/README.md`](examples/README.md)；研究卷宗：`_agile-output/.../technical-plugctx-examples-directory-2026-08-17/`。

### 包内（`crates/plugctx/examples/`）

| 示例 | 演示 | 命令 |
|------|------|------|
| `combo` | ≥2 插件 / DI / 事件 / Effect / start·dispose（FR29） | `cargo run -p plugctx --example combo` |
| `async-start` | `AsyncPlugin` + `start_async` | `cargo run -p plugctx --example async-start --features async` |
| `stages-lifecycle` | Init→Ready→PostStart / PreDispose→Dispose | `cargo run -p plugctx --example stages-lifecycle --features stages` |

```bash
cd plugin-system
cargo check -p plugctx --examples
cargo check -p plugctx --examples --features async,stages
```

### 工作区包（`examples/` → `plugctx-examples`，`publish = false`）

| 示例 | 演示 | 命令 |
|------|------|------|
| `derive-plugin` | `#[derive(Plugin)]` | `cargo run -p plugctx-examples --example derive-plugin` |
| `component-add` | wasip2 WIT 组件 `add` | `cargo run -p plugctx-examples --example component-add --features component` |
| `wasm-echo` | Extism echo.wasm | `cargo run -p plugctx-examples --example wasm-echo --features wasm` |
| `cli-hotplug` | native 加载 / 调用 / 物理卸载 | `cargo build -p hello_plugin && cargo run -p plugctx-examples --example cli-hotplug --features native` |
| `web-service` | tiny_http + plugctx | `cargo run -p plugctx-examples --example web-service --features web` |
| `game-loop` | 无引擎 tick + 卸载后停手 | `cargo run -p plugctx-examples --example game-loop` |

```bash
cargo check -p plugctx-examples --examples
cargo check -p plugctx-examples --examples --features component
cargo check -p plugctx-examples --examples --features wasm
```

> `plugin-host` + `hello`/`echo` 是 **C ABI cdylib** 脚手架演示，与上表互补。

## 构建与演示

```bash
cd plugin-system
cargo build --workspace
cargo test --workspace
cargo test -p plugctx

# 组合场景示例（FR29）与其它演示见上文「可运行示例」
cargo run -p plugctx --example combo
cargo run -p plugctx-examples --example derive-plugin

# 可选过程宏（独立 crate；核心不依赖）
cargo test -p plugctx-derive

# 原生动态加载（feature dynamic-native；dispose 后物理卸载 / dlclose）
cargo build -p hello_plugin -p echo_plugin
cargo test -p plugctx --features dynamic-native --test acceptance_story_4_2

# WASM 动态路径（feature dynamic-wasm；Extism + 实例显式 close / FR26）
cargo test -p plugctx --features dynamic-wasm --test acceptance_story_4_3

# WASM 实例池有界 checkout（Story 7.1 / FR43）
cargo test -p plugctx --features dynamic-wasm --test acceptance_story_7_1

# WASM 实例池归还/reset/destroy（Story 7.2 / FR44–FR45）
cargo test -p plugctx --features dynamic-wasm --test acceptance_story_7_2

# 池概念文档门禁（Story 7.3 / FR46；无需 dynamic-wasm）
cargo test -p plugctx --test acceptance_story_7_3

# Component Model 宿主嵌入骨架（Story 8.1 / FR47；wasmtime 不进 default）
cargo test -p plugctx --features dynamic-wasm-component --test acceptance_story_8_1

# PluginBackend 双路径共存 / 分制品（Story 8.2 / FR48）
cargo test -p plugctx --features "dynamic-wasm,dynamic-wasm-component" --test acceptance_story_8_2
# 一 Store 一实例销毁探针（Story 8.3 / FR49）
cargo test -p plugctx --features dynamic-wasm-component --test acceptance_story_8_3
# 最小 WIT world + wasip2 样例客人（Story 8.4 / FR50）
cargo test -p plugctx --features dynamic-wasm-component --test acceptance_story_8_4
# 重建客人（需 rustup target wasm32-wasip2；CI 用检入 .wasm）
# ./scripts/build-wit-sample-guest.sh

# 混合接入 + ABI 协商（native + wasm）
cargo test -p plugctx --features "dynamic-native,dynamic-wasm" --test acceptance_story_4_4

# DynamicLoader 统一入口
cargo test -p plugctx --features "dynamic-native,dynamic-wasm" --test acceptance_story_4_5

# tracing 诊断（Story 5.4 / FR37；仅门面，无强制 subscriber）
cargo test -p plugctx --features tracing --test acceptance_story_5_4

# 列出 target/debug 下的插件 .so
cargo run -p plugin-host -- target/debug list

# 调用
cargo run -p plugin-host -- target/debug call hello greet rust
cargo run -p plugin-host -- target/debug call echo echo ping
```

`plugctx` 启用 `dynamic-native` 后可用 `load_native_plugin` 将 C ABI 插件安装进同一 `Context`。卸载先撤销注册与 Effect，再 Drop `libloading::Library`（`dlclose` / `FreeLibrary`）。热插拔：load → use → dispose → load（不提供 `reload()`）。Windows 上若映射期间文件被锁，须先 dispose 或换路径再写制品。

启用 `dynamic-wasm` 后可用 `load_wasm_plugin` 加载 **Extism** WASM 实例（制品须为合法 `\0asm`，验收用 `bundled_echo_wasm` / `testdata/echo.wasm`）。dispose / `close` 显式释放 Extism 插件实例（FR26）。亦提供 `WasmInstancePool`：可配置 `max_instances`、带超时 `checkout`（FR43）；Guard **Drop 归还**（`reset` + 工厂重建，防跨借出串态，FR44），[`WasmCheckoutGuard::destroy`] **销毁不归还**（FR45）。`extism` 仅经本 feature 进入依赖图，不进默认构建。

启用 `dynamic-wasm-component` 后可用 `load_wasm_component` / `ComponentLoader` / `PluginBackend::Component` 经 **`wasmtime::component`** 加载组件制品并调用导出（FR47；骨架 fixture `testdata/component_add.wat`；**FR50** 真实 wasip2 WIT 客人 `testdata/wit_sample_add.wasm`，源码 `guests/wit-sample/`）。`ComponentPlugin` 接入同一 `Context` 生命周期（`provide(ComponentInvoker)` + dispose Effect **Drop Store**）。销毁语义为「一 Store 一实例」：`close`/dispose → Store Drop，`store_drop_count` 可观测（FR49）。`wasmtime` **不**进入默认依赖图（NFR14）；版本钉死与客人三元组见 [`docs/component-model-versions.md`](docs/component-model-versions.md)。与 Extism 路径经 `PluginBackend` **分路径、分制品**（FR48），勿假设同一 `.wasm` 两吃。

**池 vs `PluginHandle::dispose`**：`PluginHandle::dispose` 卸载 Context 已安装插件；池 Drop/destroy 只管理借出 WASM 实例——归还 ≠ 销毁 ≠ Context 精确卸载（详见 `dynamic_wasm` 模块文档表）。

### 逻辑 InstancePool ≠ Wasmtime 资源 pooling（FR46）

| 层 | 是什么 | 本仓库入口 |
| --- | --- | --- |
| **逻辑 InstancePool** | 应用层有界 checkout / 超时 / 归还 reset / 显式 destroy | feature `dynamic-wasm`；`WasmInstancePool` / `WasmPoolConfig` / `WasmCheckoutGuard`；验收 `acceptance_story_7_1`、`7_2` |
| **Wasmtime 资源 pooling** | 运行时 `PoolingAllocationConfig` 等内存/表槽复用 | **本 crate 不封装**；Extism 底层或使用 Wasmtime，但不向宿主暴露该配置 |

概念与 Feature 对照见 [`docs/feature-matrix.md`](docs/feature-matrix.md)。文档门禁：`cargo test -p plugctx --test acceptance_story_7_3`。

上手指南（最小插件、native 热插拔、CLI/Web/游戏示例命令）：[`docs/guide.md`](docs/guide.md)。

## 写一个插件（C ABI 脚手架）

1. 新建 `cdylib` crate，依赖 `plugin-api`。
2. 导出 `#[no_mangle] pub extern "C" fn plugin_entry() -> PluginVTable`。
3. 实现 `create` / `name` / `init` / `call` / `free_buffer` / `destroy`。
4. 输出缓冲必须用 `plugin_api::alloc_output`，由宿主通过 vtable `free_buffer` 释放。

当前 ABI 版本：`PLUGIN_ABI_VERSION = 1`（native）/ `WASM_ABI_VERSION = 1`（wasm / Extism）。布局变更时递增版本，宿主会拒绝不匹配的制品。
