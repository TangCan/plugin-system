# plugctx Feature 矩阵（FR42）

本文档对齐设计 **§2.4 可插拔扩展架构** 与设计大纲 **§7.3 Feature 划分**（正文落点见下方「刻意偏离」），汇总当前 crate features、依赖与测试入口。

发布切片：[`../CHANGELOG.md`](../CHANGELOG.md)（0.1.0 核心 / 0.2.0 扩展）。  
测试门禁：[`testing.md`](testing.md)。

## 总表

| Feature / crate | 默认 | 额外依赖 | 能力摘要 | 测试入口 |
| --- | --- | --- | --- | --- |
| （默认核心） | 开 | 无扩展依赖 | 同步 `Plugin` / DI / 事件 / Effect / isolate / Ready·Dispose | `cargo test -p plugctx`；`acceptance_story_1_*` / `2_*` |
| `async` | 关 | `async-trait`、`futures` | `AsyncPlugin` + `start_async`（不绑运行时） | `acceptance_story_3_1` |
| `parallel` | 关 | 隐含 `async`（复用 `futures`） | 宿主侧 `emit_parallel` fan-out | `acceptance_story_3_2` |
| `thread-safe` | 关 | `parking_lot` | `Arc`+`RwLock`/`Mutex`；`Send+Sync` Context | `acceptance_story_4_1` |
| `dynamic-native` | 关 | `libloading`、`plugin-api` | C ABI 动态库；dispose 后 Drop `Library`（`dlclose`）；热插拔 = load → dispose → load；`PLUGIN_ABI_VERSION` | `acceptance_story_4_2`（+ `4_4`/`4_5`） |
| `dynamic-wasm` | 关 | `extism`（可选） | Extism WASM 适配器；单实例显式 close/free；**逻辑** `WasmInstancePool`（有界 checkout / 归还 reset / destroy）；`WASM_ABI_VERSION` | `acceptance_story_4_3`；池：`7_1` / `7_2`；概念门禁：`7_3` |
| `dynamic-wasm-component` | 关 | `wasmtime`（可选，**47.x**） | `wasmtime::component`；`ComponentPlugin`/`ComponentLoader`/`PluginBackend`；与 Extism **分路径、分制品**（FR48）；**一 Store 一实例**，销毁=Drop Store（FR49）；**WIT + wasip2 样例客人**（FR50，`wit_sample_add.wasm`） | `acceptance_story_8_1`；双后端：`8_2`（需 `dynamic-wasm`）；Store Drop：`8_3`；WIT 样例：`8_4`；版本钉死见 [`component-model-versions.md`](component-model-versions.md) |
| `tracing` | 关 | `tracing` 门面 | build/emit/dispose 诊断 span；无强制 subscriber | `acceptance_story_5_4` |
| `stages` | 关 | 无 | `InitEvent` / `PostStartEvent` / `PreDisposeEvent` | `acceptance_story_6_1`；与 `async` 组合见 `acceptance_story_6_1_async` |
| `plugctx-derive` | 独立 crate | `syn`/`quote`/… | `#[derive(Plugin)]`；核心不依赖 | `cargo test -p plugctx-derive` |

**依赖边**：`parallel` → `async`。其余扩展 feature 互不强制；`thread-safe` 与部分默认同步验收用例互斥，CI 用显式矩阵而非盲目 `--all-features`。

**空 default / docs.rs（FR52）**：`default = []`；重运行时仅 `dep:` 具名 feature。docs.rs 构建启用轻量子集 `async,parallel,thread-safe,tracing,stages`，**不含** native/wasm/component（见 [`publishing.md`](publishing.md)）。

**`dynamic-wasm` / 实例池（两层概念，FR46）**：

| 层 | 含义 | 本 crate |
| --- | --- | --- |
| **逻辑 InstancePool** | 应用层有界 checkout / 超时 / 归还 reset / 显式 destroy | 启用 `dynamic-wasm` 后的 `WasmInstancePool` / `WasmPoolConfig` / `WasmCheckoutGuard`；验收 `acceptance_story_7_1`、`7_2` |
| **Wasmtime 资源 pooling** | 运行时 `PoolingAllocationConfig` 等线性内存/表槽复用 | **不**作为本 crate 公开 API；Extism 路径底层可能使用 Wasmtime，宿主不配置该层 |

不得再将「宿主侧实例池」表述为「仅 NFR10 意图、无实现」——Epic 7 已交付逻辑池。

## 与设计 §2.4 对齐

§2.4 规定：扩展经 Cargo features + 独立 derive crate；核心保持轻量。

| §2.4 项 | 本仓库状态 |
| --- | --- |
| `async` | 已交付 |
| `parallel`（依赖 async） | 已交付 |
| `dynamic-native`（`libloading` + 稳定 C ABI） | 已交付；dispose 后物理卸载（`dlclose`） |
| `thread-safe`（`Arc` + `parking_lot::RwLock`） | 已交付 |
| `plugctx-derive` 独立 crate | 已交付；核心不依赖宏 crate |
| 不以 `abi_stable` 为基线 | 已遵守（NFR6） |

## 刻意偏离说明

1. **章节编号**：设计大纲将「Feature 划分」记为 **§7.3**；正文 `7. 技术选型与依赖.md` 中对应内容在 **§7.9**（§7.3 正文为「事件监听器与重入」）。本矩阵以大纲意图（Feature 表）为准，并同时引用正文 §7.9。
2. **相对 §2.4 图增补**：实现另提供 `dynamic-wasm`、`dynamic-wasm-component`、`tracing`、`stages`——§2.4 总图未逐一画出，属扩展切片增强，不改变核心默认路径。
3. **dynamic 拆分**：设计早期常写单一 `dynamic`；实现拆为 `dynamic-native` + `dynamic-wasm`（+ 可选 `dynamic-wasm-component`），Extism 与 CM 分 feature（FR34 / FR47）。双后端经 `PluginBackend` 共存时仍须**分制品**（FR48）：Extism PDK `.wasm` 与 Component 组件二进制不兼容，**禁止**暗示一份 `.wasm` 两吃。
4. **`tracing` 非默认**：设计 §7.6 推荐诊断门面；为满足 NFR1（默认依赖最小）故以可选 feature 提供，不引入 `tracing-subscriber`。
5. **版本号（FR54）**：工作区 `version = "0.1.1"` 可同时承载 0.2.0 **能力清单**（见 CHANGELOG）；文档切片 ≠ 强制 bump 到 `0.2.0` 字符串。加性 feature 留同 `0.y`；breaking 才 bump。详见 [`publishing.md`](publishing.md)「0.y 版本策略」。

## 启用示例

```bash
# 核心（0.1.0）
cargo test -p plugctx

# 扩展矩阵（0.2.0 / FR41）
./scripts/ci-extension-matrix.sh

# 单项
cargo test -p plugctx --features stages --test acceptance_story_6_1
cargo test -p plugctx-derive
```

## 参考文献

- [Source: `docs/requirements/2. 总体架构.md` §2.4]
- [Source: `docs/requirements/7. 技术选型与依赖.md` §7.9]
- [Source: `docs/requirements/设计方案大纲（v2）.md` §7.3]
- [Source: `docs/requirements/9. 实施路线图.md` §9]
- [Source: `crates/plugctx/Cargo.toml` `[features]`]
