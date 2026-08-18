# plugctx 测试金字塔与回归门禁

本文档对齐设计方案第 8 章，汇总 **已交付** 的测试入口与关键路径映射，便于本地与 CI 回归。扩展专项见 Story 5.8。

## 测试金字塔

| 层次 | 目的 | 本仓库落点 |
|------|------|------------|
| 单元 | 组件细节、快速反馈 | `crates/plugctx/src/**` 内 `#[cfg(test)]`（若有）+ 细粒度断言 |
| 集成（acceptance） | 组件协作与故事 AC | `crates/plugctx/tests/acceptance_story_*.rs` |
| 属性（proptest） | 随机序列不变量 | **已交付** Story 5.5：`acceptance_story_5_5` |
| 编译失败（trybuild） | API 误用在编译期失败 | **已交付** Story 5.6：`crates/plugctx/tests/ui/`（≥3 `compile_fail`） |
| 扩展模块 | async/parallel/thread-safe/dynamic/`tracing` | **已交付** Story 5.8（FR41）：`acceptance_story_3_*` / `4_*` + `scripts/ci-extension-matrix.sh`；`tracing` 见 `5_4` |
| 基准（bench） | 性能回归 | **已交付** Story 5.7：`cargo bench -p plugctx --bench core_paths` |

```text
        bench（已交付） / 扩展专项（已交付 FR41）
     trybuild（已交付） / proptest（已交付）
        集成 acceptance_story_*
              单元 / 模块内测
```

## 关键路径映射（Epic 1 已交付能力）

| 能力 | 验收文件 | 说明 |
|------|----------|------|
| 生命周期 `new` / `start` / `dispose` | `acceptance_story_1_2.rs` | Started/Disposed、重复 start 错误、幂等 dispose |
| DI 依赖错误 | `acceptance_story_1_4.rs` | `MissingDependency` / `CircularDependency` |
| Error / API 冻结 | `acceptance_story_6_2.rs` | §6.2.5 七变体 + `docs/api-freeze.md`（FR35） |
| 0.1 / 0.2 发布切片 | `acceptance_story_6_3.rs` | CHANGELOG + `docs/feature-matrix.md`（FR42） |
| 事件重入 | `acceptance_story_1_5.rs` | 监听器内 `emit`/`on` 不 panic |
| effect 逆序清理 | `acceptance_story_1_6.rs` | dispose 时 cleanup 逆序 |
| isolate 继承与级联 | `acceptance_story_1_7.rs` | 子上下文级联 dispose |

默认 features 下执行：

```bash
cd plugin-system
cargo test -p plugctx
```

即可覆盖上表关键路径（及后续默认同步故事）。feature 专项须显式传 `--features`，见 README「回归门禁」。

```bash
# tracing 诊断（Story 5.4 / FR37；已纳入 ci-test.sh）
cargo test -p plugctx --features tracing --test acceptance_story_5_4

# 扩展生命周期 stages（Story 6.1 / FR32）
cargo test -p plugctx --features stages --test acceptance_story_6_1

# async + stages 组合（start_async 阶段顺序 / 失败路径）
cargo test -p plugctx --features "async,stages" --test acceptance_story_6_1_async

# Error / API 冻结对照（Story 6.2 / FR35）
cargo test -p plugctx --test acceptance_story_6_2

# 0.1/0.2 发布切片文档护栏（Story 6.3 / FR42）
cargo test -p plugctx --test acceptance_story_6_3

# crates.io 元数据与 publish=false 边界（Story 9.1 / FR51）
cargo test -p plugctx --test acceptance_story_9_1

# 空 default / docs.rs 构建子集（Story 9.2 / FR52）
cargo test -p plugctx --test acceptance_story_9_2

# dry-run CI 门禁与 release 工作流文档（Story 9.3 / FR53）
cargo test -p plugctx --test acceptance_story_9_3
./scripts/ci-publish-dry-run.sh

# Trusted Publishing 工作流（Story 1.1 / FR1）
cargo test -p plugctx --test acceptance_story_10_1

# cargo-hack 互斥 feature（Story 1.2 / FR3）
cargo test -p plugctx --test acceptance_story_10_2
./scripts/ci-cargo-hack.sh

# 0.y 版本策略与 CHANGELOG 对齐（Story 9.4 / FR54）
cargo test -p plugctx --test acceptance_story_9_4
# 或直接：
# cargo publish --workspace --dry-run
# cargo publish -p plugctx-derive --dry-run
# cargo publish -p plugctx --dry-run```

发布说明、Feature 矩阵与 crates.io 边界：[`../CHANGELOG.md`](../CHANGELOG.md)、[`feature-matrix.md`](feature-matrix.md)、[`publishing.md`](publishing.md)。

## 属性测试（proptest / Story 5.5 / FR38）

随机 **安装 / 卸载 / emit / start / dispose** 序列，断言不变量：

- dispose 后无悬挂插件注册（`PluginHandle` 均 `!is_alive()`）
- 已构建插件的 effect cleanup **恰好一次**（NFR3）
- 重入 `emit` 不 panic（NFR9）
- `is_started` / `is_disposed` 与非法状态转换返回明确错误（非 panic）

```bash
cd plugin-system
cargo test -p plugctx --test acceptance_story_5_5
```

设计依据：`docs/requirements/8. 测试策略.md` §8.5。

## trybuild 编译失败套件（Story 5.6 / FR39）

- 入口：`cargo test -p plugctx --test ui`
- 用例目录：`crates/plugctx/tests/ui/`（每个 `.rs` 配套 `.stderr`）
- **已交付 ≥3** 个 `compile_fail`，钉死常见 API 误用（设计 §8.6 / NFR4）：

| 用例 | 意图 |
|------|------|
| `plugin_must_impl_trait` | 未实现 `Plugin` 不得 `Context::plugin` |
| `plugin_build_wrong_return` | `Plugin::build` 返回类型须为 `Result<(), Error>` |
| `event_must_be_static` | 事件/`on` 闭包须满足 `'static` |
| `start_async_requires_feature` | 默认 features 下无 `start_async`（需 `async`） |

护栏：`acceptance_story_5_6`。宏侧误用仍由 `plugctx-derive` 自有 trybuild 覆盖（Story 5.1）。

## 核心路径基准（criterion / Story 5.7 / FR40）

使用 **criterion** 对默认同步内核三条热路径建基线（设计 §8.8）：

| 基准 id | 路径 | 说明 |
|---------|------|------|
| `core_get` | 服务 `get` | 已 `provide` 后反复查找 |
| `core_emit` | 事件 `emit` | 有界监听器（默认 16）fan-out |
| `core_start` | 小规模 `start` | 每轮新建 Context + 少量插件依赖构建后 dispose |

```bash
cd plugin-system
# 完整基准（本地 / 性能回归；默认 CI 不跑，因较慢）
cargo bench -p plugctx --bench core_paths

# 仅编译防腐化（CI 友好）
cargo bench -p plugctx --bench core_paths --no-run

# 基线记录与对比（criterion）
cargo bench -p plugctx --bench core_paths -- --save-baseline main
cargo bench -p plugctx --bench core_paths -- --baseline main
```

基线产物默认在 `target/criterion/`（HTML 报告可用浏览器打开）。设计表中更大负载（如 10 万次迭代、100 监听器）由 criterion 采样与本地长跑覆盖意图；默认配置优先可重复、可本地快速验证。

护栏：`acceptance_story_5_7`。

## 扩展模块专项测试（Story 5.8 / FR41）

对齐设计 §8.7：在启用对应 Cargo feature 时运行专项验收；**默认 features 核心门保持独立且必须仍绿**（NFR5）。功能用例已由 Epic 3/4 交付；本故事巩固 **CI 矩阵 + 文档门禁**。

| Feature | 验收测试 | 命令摘要 |
|---------|----------|----------|
| `async` | `acceptance_story_3_1` | `cargo test -p plugctx --features async --test acceptance_story_3_1` |
| `parallel` | `acceptance_story_3_2` | `cargo test -p plugctx --features parallel --test acceptance_story_3_2` |
| `thread-safe` | `acceptance_story_4_1` | `cargo test -p plugctx --features thread-safe --test acceptance_story_4_1` |
| `dynamic-native` | `acceptance_story_4_2` | 先 `cargo build -p hello_plugin -p echo_plugin`，再 `--features dynamic-native --test acceptance_story_4_2`。native dispose 为物理卸载（`dlclose`），覆盖热插拔 load→dispose→load。 |
| `dynamic-wasm` | `acceptance_story_4_3` | `--features dynamic-wasm --test acceptance_story_4_3` |
| `dynamic-wasm`（InstancePool） | `acceptance_story_7_1` | `--features dynamic-wasm --test acceptance_story_7_1` |
| `dynamic-wasm`（Pool 归还/destroy） | `acceptance_story_7_2` | `--features dynamic-wasm --test acceptance_story_7_2` |
| 池概念文档（FR46） | `acceptance_story_7_3` | `cargo test -p plugctx --test acceptance_story_7_3` |
| `dynamic-wasm-component` | `acceptance_story_8_1` | `--features dynamic-wasm-component --test acceptance_story_8_1`；版本见 [`component-model-versions.md`](component-model-versions.md) |
| `dynamic-wasm` + `dynamic-wasm-component`（PluginBackend / 分制品） | `acceptance_story_8_2` | `--features "dynamic-wasm,dynamic-wasm-component" --test acceptance_story_8_2` |
| `dynamic-wasm-component`（一 Store 一实例 Drop） | `acceptance_story_8_3` | `--features dynamic-wasm-component --test acceptance_story_8_3`（FR49：未 Drop 可用 / Drop 后探针+调用失败） |
| `dynamic-wasm-component`（WIT + wasip2 样例） | `acceptance_story_8_4` | `--features dynamic-wasm-component --test acceptance_story_8_4`（FR50：检入 `wit_sample_add.wasm`；重建见 `scripts/build-wit-sample-guest.sh` / [`component-model-versions.md`](component-model-versions.md)） |
| `dynamic-native` + `dynamic-wasm` | `acceptance_story_4_4` / `4_5` | `--features "dynamic-native,dynamic-wasm" --test acceptance_story_4_4`（及 `4_5`） |
| `tracing` | `acceptance_story_5_4` | **已纳入** `./scripts/ci-test.sh`：`cargo test -p plugctx --features tracing --test acceptance_story_5_4` |
| `async` + `stages` | `acceptance_story_6_1_async` | `cargo test -p plugctx --features "async,stages" --test acceptance_story_6_1_async` |

一键矩阵（FR41）：

```bash
cd plugin-system
./scripts/ci-extension-matrix.sh
# 或完整回归（先 rustfmt，再默认门 + 扩展矩阵 + tracing 5.4）
just test
./scripts/ci-test.sh
```

> 注意：勿用盲目 `--all-features` 替代显式矩阵——`thread-safe` 会与部分默认同步验收（`#![cfg(not(feature = "thread-safe"))]`）互斥；显式按 feature 跑更清晰。CI 另用 `./scripts/ci-cargo-hack.sh`：`cargo hack check -p plugctx --feature-powerset --depth 1 --mutually-exclusive-features thread-safe,default --exclude-all-features`（排除 native/wasm；本地需 `cargo install cargo-hack`）。hack 用 `check` 而非全量 `test`，以免 `async` 下 trybuild `start_async_requires_feature` 从 compile_fail 变成通过。

护栏：`acceptance_story_5_8`。

## 文档门禁

- 用户要点：根 `README.md`（生命周期、常见错误、Feature 矩阵、回归命令）
- API 说明：`cargo doc -p plugctx --no-deps --open`
- 一键脚本：`just test`（`cargo fmt --all` + `./scripts/ci-test.sh`）
