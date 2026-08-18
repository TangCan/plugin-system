# Changelog

本文件记录 `plugin-system` / `plugctx` 的**发布切片**（FR42 / FR54 / 设计 §9）。

## Unreleased

无。

## crates.io 0.1.2 — 2026-08-18

锁步：`plugctx` / `plugctx-derive` **0.1.2**。发布质量切片：tag 触发 Trusted Publishing（OIDC，`release.yml`）；CI `cargo-hack` 表达 `thread-safe` 与空 default 互斥；crates.io README 不可变与 yank 说明；docs.rs 两公开 crate 均不得 `all-features`；指南写清 native 卸载限度；WIT 钉 `plugctx:sample@0.1.0` / wasmtime 47 / wit-bindgen 0.60，不提前 `wasi@0.3.0`。

## crates.io 0.1.1 — 2026-08-18

锁步：`plugctx` / `plugctx-derive` **0.1.1**。`authors` 从占位 `release_check` 改为 `Tang Can <tang_can@qq.com>`。**不**改写已上架的 0.1.0 包（yank ≠ 删除）。

## crates.io 0.1.0 — 2026-08-18

首次上架（锁步）：[`plugctx`](https://crates.io/crates/plugctx) / [`plugctx-derive`](https://crates.io/crates/plugctx-derive) **0.1.0**。同一版本不可再传；后续发版见 [`docs/publishing.md`](docs/publishing.md)。

Cargo `version` 为 `0.1.0`，同时已交付下文 **[0.1.0] 核心** 与 **[0.2.0] 扩展**能力清单（FR54：清单标题 ≠ 强制把字符串改成 `0.2.0`）。

- **breaking（仅 `dynamic-native`）**：native 插件 `PluginHandle::dispose` / Context dispose 后 Drop `libloading::Library`（`dlclose` / `FreeLibrary`）。热插拔 = load → dispose → load；**不**新增 `reload()`。默认同步内核与 WASM FR26（实例 `close`/`free`）不变；**不**因此把 workspace `version` 改成 `0.2.0`。
- **包名**：`pluggable` / `pluggable-derive` → **`plugctx` / `plugctx-derive`**（crates.io 上 `pluggable` 已被无关方占用；见 `docs/publishing.md`）。
- **示例**：扩展 `crates/plugctx/examples`（`async-start` / `stages-lifecycle`）；新增工作区包 `examples/`（`plugctx-examples`，`publish = false`：derive / wasm / component）。

## 0.y 版本策略（FR54）

| 概念 | 含义 |
|------|------|
| **能力清单** | 下文 `[0.1.0]` / `[0.2.0]` 标题：路线图可验收的功能集合 |
| **crates.io / Cargo `version`** | `workspace.package.version`；crates.io 当前为 **`0.1.2`**（2026-08-18 上架；0.1.0 / 0.1.1 仍保留） |

**规则：**

1. **加性** Cargo feature / 文档能力可留在同一 `0.y`（不必为「清单写了 0.2」而把 `version` 改成 `0.2.0`）。
2. **破坏性（breaking）** API / 语义变更才 bump `0.y`（或按 Cargo SemVer 对 `0.y.z` 的惯例处理兼容）。
3. 文档「0.2 能力清单」**≠** 强制 `version = "0.2.0"`；清单已交付而版本字符串仍为 `0.1.0` 是**允许且预期**的。
4. 实际上架名/版本还受 crates.io 占用约束——见 [`docs/publishing.md`](docs/publishing.md)「0.y 与改名」。

Feature 对照见 [`docs/feature-matrix.md`](docs/feature-matrix.md)。测试门禁见 [`docs/testing.md`](docs/testing.md)。

---

## [0.1.0] — 核心版（Epic 1–2 同步内核）

**范围**：默认同步 `plugctx` 内核；**不要求**启用任何扩展 Cargo feature。

### 能力（Epic 1–2）

| 域 | 交付 |
| --- | --- |
| 骨架 | workspace、`plugctx` crate、模块边界 |
| 生命周期 | `Context::new` / `start` / `dispose`（幂等 dispose） |
| 插件 | `Plugin` trait、延迟/立即安装、`slotmap` 稳定插件 ID |
| DI | `TypeId` `provide` / `get`、依赖排序、`MissingDependency` / `CircularDependency` |
| 事件 | 类型化 `on` / `emit`（含重入）、内置 `ReadyEvent` / `DisposeEvent` |
| Effect | 登记 / 取消、dispose 时**逆序** cleanup |
| 隔离 | `isolate` 继承与级联 dispose |
| 作用域 | `PluginScope` 构建期自动记录 |
| 卸载 | `PluginHandle::dispose` 精确卸载；§5.3 回滚细则（索引/区间） |
| Trait 服务 | `provide_trait` / `get_trait` |
| 拦截 | `ContextInterceptor`（build / emit 切入） |

### 测试门禁最小集

```bash
cd plugin-system
cargo test -p plugctx          # 默认 features：Epic 1–2 关键路径
./scripts/ci-test.sh             # 默认门 + trybuild + bench 编译 + rustdoc + FR41 矩阵
```

关键路径映射（生命周期 / DI / 事件重入 / Effect / isolate 等）见 [`docs/testing.md`](docs/testing.md)。  
核心 API / Error 冻结对照：[`docs/api-freeze.md`](docs/api-freeze.md)。

### 非目标（0.1.0）

- `async` / `parallel` / `thread-safe` / `dynamic-*` / `stages` / `tracing`
- 独立 `plugctx-derive` crate（属 0.2.0 扩展切片）

---

## [0.2.0] — 扩展版（feature + derive）

**范围**：在 0.1.0 核心之上，按 Cargo feature / 独立 crate 交付扩展；默认 features 下核心测试仍须保持绿色（NFR5）。

### 扩展清单与测试映射（FR42 + FR41）

| 切片项 | Feature / crate | 对应测试入口 |
| --- | --- | --- |
| async | `async` | `cargo test -p plugctx --features async --test acceptance_story_3_1` |
| parallel | `parallel`（隐含 `async`） | `cargo test -p plugctx --features parallel --test acceptance_story_3_2` |
| thread-safe | `thread-safe` | `cargo test -p plugctx --features thread-safe --test acceptance_story_4_1` |
| dynamic-native | `dynamic-native` | 先 `cargo build -p hello_plugin -p echo_plugin`，再 `--features dynamic-native --test acceptance_story_4_2` |
| dynamic-wasm | `dynamic-wasm` | `--features dynamic-wasm --test acceptance_story_4_3` |
| 混合动态 + ABI / Loader | `dynamic-native,dynamic-wasm` | `acceptance_story_4_4` / `4_5` |
| derive | `plugctx-derive` crate | `cargo test -p plugctx-derive` |
| stages | `stages` | `cargo test -p plugctx --features stages --test acceptance_story_6_1` |
| tracing（诊断，建议随扩展文档） | `tracing` | `cargo test -p plugctx --features tracing --test acceptance_story_5_4` |
| FR41 扩展矩阵 | — | `./scripts/ci-extension-matrix.sh` |

### 一键扩展回归

```bash
cd plugin-system
./scripts/ci-extension-matrix.sh
```

### 相对 0.1.0 的说明

- 扩展 API 经 feature 隔离，不破坏默认同步 `Context` / `Plugin` 契约。
- 动态加载：native dispose 后 **物理卸载（Drop `Library` / `dlclose`）**；WASM 实例须显式 `close`/`free`（FR26）。默认同步内核契约不变。
- 完整 Feature 表与设计偏离：[`docs/feature-matrix.md`](docs/feature-matrix.md)。
