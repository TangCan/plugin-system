# Component Model 版本矩阵（NFR12 / FR47）

本文档钉死 `dynamic-wasm-component` 路径的宿主 / 客人工具链兼容要点，并说明相对
Component Model 1.0 路线图的适配预期。

关联：[`feature-matrix.md`](feature-matrix.md)、[`testing.md`](testing.md)、
Epic 8 Story 8.1。

## 宿主（本 crate）

| 组件 | 钉死版本 | 说明 |
| --- | --- | --- |
| **wasmtime** | **47.x**（workspace `wasmtime = "47"`；复核线 crates.io **47.0.3**，2026-07-31） | 经 Cargo feature `dynamic-wasm-component` 可选依赖；默认 feature **不含** wasmtime（NFR14） |
| API 面 | `wasmtime::component` | 需启用 component-model（wasmtime 默认 feature 已含；骨架显式 `Config::wasm_component_model(true)`） |

**不**进入默认依赖图：`cargo tree -p plugctx -e normal` 不得出现 `wasmtime`。

## 客人工具链（Story 8.4 / FR50 已交付）

| 组件 | 钉死 / 预期 | 说明 |
| --- | --- | --- |
| 目标三元组 | **`wasm32-wasip2`** | 官方 Rust 组件指南主线；`rustup target add wasm32-wasip2` |
| **wit-bindgen** | **0.60.x**（crates.io 复核线 **0.60.0**；客人生成；宿主**不**依赖该 crate） | 与 wasip2 + WIT world 配套；升级时与 wasmtime 47 同检 |
| cargo-component | 可选 / 过渡 | BA 文档存在张力：部分场景仍需；本仓库以 **wit-bindgen + wasm32-wasip2** 为准 |
| 样例源码 | `guests/wit-sample/` | WIT：`wit/world.wit`（`world sample` 导出 `add`）；README 含照抄步骤 |
| CI 制品 | `crates/plugctx/testdata/wit_sample_add.wasm` | **真实** wasip2 构建产物检入；CI **不**安装完整客人工具链 |
| 重建脚本 | `scripts/build-wit-sample-guest.sh` | 改 WIT/客人后本地重建并覆盖检入 `.wasm` |

Story 8.1 骨架 fixture（`testdata/component_add.wat`）仍保留：无 WASI、无 WIT 源，用于最小宿主路径。
FR50 验收走 **wasip2 WIT 样例**（`bundled_wit_sample_add_wasm` / `acceptance_story_8_4`）。

## 实际 WIT pin（FR5）

与 `guests/wit-sample/` 及检入 `wit_sample_add.wasm` **一致**的钉死值：

| 项 | 实际 pin |
| --- | --- |
| WIT 包 | **`plugctx:sample@0.1.0`**（`guests/wit-sample/wit/world.wit`，`world sample` 导出 `add`） |
| WASI 导入 | **无**。样例不 `use wasi:*`，也不钉已发布的 **`wasi@0.3.0`** |
| 客人 `wit-bindgen` | **0.60.x**（`guests/wit-sample/Cargo.toml`） |
| 目标 | **`wasm32-wasip2`** |
| 宿主 wasmtime | **47.x** |

WASI 0.3.0 规范已发布，但当前 wasmtime 47 / wit-bindgen 0.60 **尚未**把样例客人切到该发布标签。在这两条工具链刷新之前，**禁止**把 `guests/wit-sample` 提前改钉 `wasi@0.3.0`（改了会在实例化时报错）。需要 WASI 时继续跟 **wasip2 + 0.60** 走，不要跳标签。

`dynamic-wasm`（Extism 字节 ABI）与 `dynamic-wasm-component`（本路径）继续 **分 feature、分制品**；**禁止**暗示一份 `.wasm` 两吃。WASM 卸载仍是实例 `close`/`free` 或 Drop Store，**不是** native `dlclose`。本切片**不做** Fidius 式签名包。

**诚实说明**：本机已验证 `cargo build --target wasm32-wasip2 --release` 可生成组件；产物仅导出 `add`，宿主现有无 WASI 的 `Linker` 即可实例化。CI 默认加载检入 `.wasm`，避免把 wasip2 工具链设为硬依赖。

## 相对 Component Model 1.0 的适配预期

| 现状 | 预期 |
| --- | --- |
| WASI P2 / Component Model **已可生产使用** | 本路径基于当前 wasmtime 47 `component` API 开工，**不等待** CM 1.0 |
| CM **1.0 无船期**（ABI lazy、浏览器、WIT 表达力等仍在演进） | 升级 wasmtime 大版本时复查 `component` API 与 guests；必要时经 adapters 保兼容 |
| Extism **无近期待吞 CM** | 继续双 feature：`dynamic-wasm`（Extism）与 `dynamic-wasm-component`（wasmtime）；经 `PluginBackend` 共存于同一宿主 / Context，但客人须**分制品**或另建适配层；**禁止**暗示一份 `.wasm` 两吃（FR48） |

## 双后端与分制品（FR48）

| 路径 | Feature | 制品形态 | 统一入口 |
| --- | --- | --- | --- |
| Extism PDK | `dynamic-wasm` | Extism 编译的 core WASM（如 `testdata/echo.wasm`） | `PluginBackend::Extism(WasmLoader)` |
| Component Model | `dynamic-wasm-component` | CM 组件：WAT 骨架（`component_add.wat`）或 wasip2 WIT 客人（`wit_sample_add.wasm`） | `PluginBackend::Component(ComponentLoader)` |

同一 `Context` 可同时安装两个后端的插件（服务类型不同：`WasmInvoker` / `ComponentInvoker`）。
**不得**把 Extism 制品交给 Component 后端（或反过来）并期望成功——ATDD 见 `acceptance_story_8_2`。

## 一 Store 一实例销毁（FR49）

Component 路径**无**单独 `free` API：销毁 = **Drop `wasmtime::Store`**（及绑定实例）。

| 路径 | 行为 | 可观测性 |
| --- | --- | --- |
| `ComponentPlugin::close` / dispose Effect | 将内部 `Option<ComponentState>` 置 `None` → Store Drop | `store_drop_count() ≥ 1`；再 `call_add` → `WasmClosed` |
| 未 close | Store 仍存活 | `store_drop_count() == 0`；`call_add` 成功 |

ATDD：`acceptance_story_8_3`（对照「未 Drop 仍可用 / Drop 后不可用」）。

## 复查窗

- **versions/compat**：约每月复核 crates.io `wasmtime` / `wit-bindgen` 最新稳定线（参见 technical research staleness map）。
- 下一机械复查建议：**2026-09-17**。

## 参考文献

- [Source: wasmtime crates.io / docs.rs `wasmtime::component`](https://docs.rs/wasmtime/latest/wasmtime/component/)
- [Source: BA Road to Component Model 1.0](https://bytecodealliance.org/articles/the-road-to-component-model-1-0)
- [Source: `_agile-output/planning-artifacts/research/technical-plugctx-instance-pool-component-model-2026-08-17/research.md` §2]
