---
title: 'technical research: plugctx requirements gap'
type: 'technical'
topic: 'plugctx requirements gap'
decision: '相对 docs/requirements，识别未实现 / 部分实现 / 文档与代码不一致的功能'
source: 'native-run'
status: complete
preset: 'standard'
validation: 'normal'
created: '2026-08-18'
updated: '2026-08-18'
verified_claims: 8
unverified_claims: 1
---

# technical research: plugctx requirements gap

**Decision this research serves:** 相对 `docs/requirements`，当前代码还有哪些功能未实现 / 只部分实现 / 文档与代码不一致

## Executive summary

**结论：设计文档里的运行时功能面已经落地。** 大纲 §3 核心组件与 §4 扩展模块（async / parallel / thread-safe / dynamic-native / dynamic-wasm / derive / stages / interceptors / trait 服务）在 `plugctx` 中均有对应 API 与验收测试。当前缺口不在「缺一整块内核」，而在三类事：

1. **产品化未完成**：`plugctx` / `plugctx-derive` 尚未上架 crates.io（API 404）[19]；`repository` 字段未写 [11][15]；没有 GitHub Actions 工作流，只有本地 `scripts/ci-test.sh` [17]。
2. **文档与示例未完成**：没有独立用户指南 [8]；没有 CLI / Web / 游戏场景示例，只有框架演示 [16][20]。
3. **语义与原文不完全同义**：引言写「热插拔」[1]，但 native 路径明确 **逻辑卸载 ≠ `dlclose`** [4]，不能当作卸载 `.so` 后再加载新版本。若干 API 形状（Error 载荷、`get` 返回 `Option`）已在 `docs/api-freeze.md` 记为刻意偏差 [14]，不算漏实现。

**建议：** 把后续工作当成发布与体验切片，而不是再开一条「补核心功能」epic。若仍要把「热插拔」当需求，需要单独故事：WASM 已有实例 `close`；native 热替换则要改卸载语义，与现行 FR25 冲突。

最大 caveat：需求正文在迭代中已吸收大量「已交付 / 刻意偏离」注释，不能把 2024 年路线图日期或大纲 §7.1 的 `anymap` 清单当成未改过的冻结规格。

## 1. 需求能力清单与非目标

原始目标来自引言 mindmap：一切皆插件、TypeId DI、isolate、生命周期 + effect、轻量可扩展、异步/并行、动态库/WASM、过程宏与拦截器 [1]。大纲把能力拆成核心 §3 与扩展 §4 [9]。

**必须交付（核心）：** `Context`、`Plugin`、`provide`/`get`（含 trait 对象）、`on`/`emit`、内置 Ready/Dispose、`effect`、`PluginScope` 精确卸载、`isolate`、`start`/`dispose`、拦截器 [9][5]。

**按 feature / 独立 crate 交付：** `AsyncPlugin` + `start_async`、`emit_parallel`、动态加载、`#[derive(Plugin)]`、`thread-safe`、生命周期阶段事件 [4][9]。

**明确非目标 / 本版不做：**

| 项 | 出处 |
| --- | --- |
| `emit_parallel` 并发上限（`buffer_unordered`） | §4.2.3 / §5.5「本版不做」[4] |
| native 卸载以 `dlclose` 为正确性前提 | §4.3.1 FR25 [4] |
| `abi_stable` 作为 ABI 基线 | §4.3、NFR6 [4] |
| 严格阶段调度器 | §4.7.4「初期用事件即可」[4] |
| 绑定 tokio/async-std | §4.1.5 [4] |

**路线图另列、不属于运行时内核：** 用户指南、CLI/Web/游戏示例、RC 社区反馈、crates.io 正式发布 [8]。CI 流水线要求 fmt / clippy / `--all-features` / `--no-default-features` / `bench --no-run` [7]。

**范围边界说明：** §2.2.2 图里画了独立 crate `plugctx-async`、`plugctx-dynamic`，同时也允许 Cargo feature [2]。这是形态选项，不是「必须拆两个包」的硬门。

## 2. 实现对照

公开 crate 出口与 feature 表对齐 [10][11][12]：

| 需求项 | 实现落点 | 状态 |
| --- | --- | --- |
| 同步内核（plugin/DI/事件/effect/isolate/生命周期） | 默认 `plugctx`；`acceptance_story_1_*` / `2_*` | 已实现 |
| trait 对象服务 | `provide_trait` / `get_trait` | 已实现（核心 API） |
| 拦截器 | `ContextInterceptor` | 已实现 |
| `async` | `AsyncPlugin`、`plugin_async`、`start_async` | 已实现 |
| `parallel` | `on_async`、`emit_parallel` | 已实现 |
| `thread-safe` | `parking_lot` + `Send+Sync` | 已实现 |
| `dynamic-native` | `NativePlugin` / `DylibLoader` / C ABI | 已实现（逻辑卸载） |
| `dynamic-wasm` | Extism + `WasmInstancePool` | 已实现（超出早期大纲） |
| `dynamic-wasm-component` | wasmtime component | 已实现（需求后续增补） |
| `stages` | Init/PostStart/PreDispose | 已实现 |
| `tracing` | 可选 feature，非默认 | 已实现；相对 §7.8「必须」是偏离 [6][12] |
| `plugctx-derive` | 独立 crate，核心不依赖 | 已实现 |
| 测试金字塔（unit/ATDD/proptest/trybuild/bench/扩展矩阵） | `docs/testing.md` + `ci-test.sh` | 主体已实现 |

工作区 `version = "0.1.0"` 同时承载 CHANGELOG 的 0.1.0 核心清单与 0.2.0 扩展清单 [13]；这是 FR54 的刻意策略，不是漏 bump。

API 冻结对照 [14] 记录的差异（不算漏功能）：`Error` 核心变体多为单元变体；`get`/`get_trait` 返回 `Option` 而非 `ServiceNotFound`；`isolate` 返回 `Result`；`Plugin::build` 取 `&Context`。

## 3. 缺口与风险

### 3.1 仍未当成产品交付的项

这些是相对路线图 / 发布章节的真实缺口：

1. **crates.io 未上架。** `GET https://crates.io/api/v1/crates/plugctx` 与 `plugctx-derive` 均为 404 [19]。publishing 文档要求首次手工 publish 占名 [15]。
2. **`repository` 未写入 Cargo.toml**（远端现已存在，元数据未跟上）[11][15]。
3. **托管 CI 未接上。** 根目录无 `.github/workflows`；`ci-test.sh` 可本地跑，但不等于「每次提交」流水线 [7][17]。脚本也没有 `cargo clippy`。`--all-features` 被刻意避开，因为 `thread-safe` 与部分默认同步验收互斥 [12]。
4. **用户指南缺失。** 有 rustdoc、README、feature-matrix，没有路线图要求的独立用户指南 [8]。
5. **场景示例缺失。** 现有示例演示框架 API（combo / derive / wasm / component），不是 CLI、Web 服务或游戏逻辑应用 [8][16][20]。

### 3.2 部分实现或语义弱于原文

1. **「热插拔」。** 引言把它写成动态加载目标 [1]。实现：进程内 `PluginHandle::dispose` 可卸载注册；WASM 可 `close` 实例；native **保留** `Library` 映射，正确性不依赖 `dlclose` [4]。因此「换一份新 `.so` 并释放旧映射」未实现，且与 FR25 冲突。
2. **同一 `dyn Trait` 多个实现。** §4.5 写「通过不同 trait 类型区分」[4]，即同一 `TypeId` 仍单槽。不是漏表，但是灵活性上限。
3. **忘记 `dispose`。** `Context` 无 `Drop` 自动清理；§6.6 已声明 cleanup 可能不跑 [5]。属已知契约，不是静默缺陷。
4. **基准覆盖窄于设计表。** `core_paths` 覆盖 `get` / `emit` / `start` [21]；§8.5 还列了 isolate×10⁵、1000 个异步插件、`emit_parallel` 等 [7]。CI 只 `--no-run`，与路线图一致。
5. **`ContextData` 拆分** 已 ADR defer，不阻塞功能 [18]。

### 3.3 刻意偏离（不要当漏实现）

- 独立 crate `plugctx-async` / `plugctx-dynamic` → 并入 `plugctx` features [2][11]。
- 大纲 `anymap` / `downcast-rs` 核心依赖 → 未引入，正文改为可选 [6][9][12]。
- `tracing` 非默认，满足 NFR1 [12]。
- Error 形状与 `get` 的 `Option` [14]。
- `emit_parallel` 不限并发 [4]。

## Cross-dimension insights

功能矩阵已经「绿」与产品仍不可被外部 Rust 项目 `cargo add`，这两件事同时成立：内部验收把能力做完了，发布通道（crates.io 元数据、托管 CI、用户指南）还没关账。把 0.2.0 **能力清单**读成「已经正式发布」会误导。

另一交叉点：动态加载在扩展层完成度高于早期大纲（Component Model、实例池），但引言最吸引人的词「热插拔」在 native 路径被 FR25 收窄。文档内部一致，对外一句话仍可能过度承诺。

## Contrary evidence

未跑 red-team 扇出（`red_team=off`，validation=normal）。反向材料来自需求正文自身的「已交付 / 本版不做 / 刻意偏离」注释：它们降低了「对照一份冻结 2024 规格会找出大量未实现功能」的先验。本次以当前 `docs/requirements` + HEAD 代码为准，不以过期甘特日期为准。

## Recommendations

1. **不要为「补内核」开新 epic。** 下游架构/路线图应把剩余工作标成发布与体验（Feeds: architecture spine 的 operational constraints；roadmap 风险）。置信度：high（代码与验收对照）。
2. **若对外仍承诺热插拔：** 改引言措辞为「逻辑卸载 + WASM 实例销毁」，或单独立项 native `dlclose`/热替换（会打破 FR25）。置信度：high。
3. **发布切片优先：** 补 `repository`、GitHub Actions 调 `ci-test.sh`、首次 `cargo publish` 占名、clippy 门（不必盲目 `--all-features`）。置信度：high。
4. **体验切片：** 一篇用户指南 + 一个真实场景示例（CLI 即可）。置信度：medium（路线图要求明确，优先级相对发布可商量）。

## Open questions

1. 现有 GitHub 远端 `TangCan/plugin-system` 是否应立刻写入 `[package].repository`？（机械上只差字段；策略未在需求里写死。）
2. 是否接受「0.2 能力已齐、版本字符串仍 0.1.0、尚未上架」为当前对外叙事？（FR54 允许；市场沟通不一定允许。）
3. native 热替换是否有真实用户？没有则保持 FR25，只改文案。

## Source appendix

| [n] | 支撑的发现 | publisher | pub date | accessed | confidence |
| --- | --- | --- | --- | --- | --- |
| [1] | 设计目标含热插拔、DI、isolate、async/WASM | [docs/requirements/1. 引言.md](../../../../docs/requirements/1.%20引言.md) | n/a | 2026-08-18 | high |
| [2] | 独立 crate vs feature 形态 | [docs/requirements/2. 总体架构.md](../../../../docs/requirements/2.%20总体架构.md) | n/a | 2026-08-18 | high |
| [4] | 扩展模块；dlclose 非目标；buffer_unordered 本版不做 | [docs/requirements/4. 扩展模块设计.md](../../../../docs/requirements/4.%20扩展模块设计.md) | n/a | 2026-08-18 | high |
| [5] | 核心/扩展 API；dispose Drop 契约 | [docs/requirements/6. API 设计概览.md](../../../../docs/requirements/6.%20API%20设计概览.md) | n/a | 2026-08-18 | high |
| [6] | tracing/anymap 依赖表述 | [docs/requirements/7. 技术选型与依赖.md](../../../../docs/requirements/7.%20技术选型与依赖.md) | n/a | 2026-08-18 | medium |
| [7] | CI 步骤与 bench 场景表 | [docs/requirements/8. 测试策略.md](../../../../docs/requirements/8.%20测试策略.md) | n/a | 2026-08-18 | high |
| [8] | 阶段5 文档/示例/发布 | [docs/requirements/9. 实施路线图.md](../../../../docs/requirements/9.%20实施路线图.md) | n/a | 2026-08-18 | high |
| [9] | 大纲能力分解与 anymap 清单 | [docs/requirements/设计方案大纲（v2）.md](../../../../docs/requirements/设计方案大纲（v2）.md) | n/a | 2026-08-18 | high |
| [10] | 公开模块与 feature 出口 | [crates/plugctx/src/lib.rs](../../../../crates/plugctx/src/lib.rs) | 2026-08-18 | 2026-08-18 | high |
| [11] | features；无 repository | [crates/plugctx/Cargo.toml](../../../../crates/plugctx/Cargo.toml) | 2026-08-18 | 2026-08-18 | high |
| [12] | Feature 矩阵与刻意偏离 | [docs/feature-matrix.md](../../../../docs/feature-matrix.md) | n/a | 2026-08-18 | high |
| [13] | 0.1/0.2 能力清单 | [CHANGELOG.md](../../../../CHANGELOG.md) | n/a | 2026-08-18 | high |
| [14] | API/Error 冻结偏差 | [docs/api-freeze.md](../../../../docs/api-freeze.md) | n/a | 2026-08-18 | high |
| [15] | 发布边界与首次手工 publish | [docs/publishing.md](../../../../docs/publishing.md) | n/a | 2026-08-18 | high |
| [16] | 示例与门禁说明 | [README.md](../../../../README.md) | n/a | 2026-08-18 | high |
| [17] | 本地 CI 脚本无 clippy | [scripts/ci-test.sh](../../../../scripts/ci-test.sh) | 2026-08-18 | 2026-08-18 | high |
| [18] | ContextData 拆分推迟 | [docs/adr-contextdata-split.md](../../../../docs/adr-contextdata-split.md) | 2026-08-17 | 2026-08-18 | high |
| [19] | crates.io 未发布 | [crates.io API plugctx](https://crates.io/api/v1/crates/plugctx) | 2026-08-18 | 2026-08-18 | high |
| [20] | 工作区示例范围 | [examples/README.md](../../../../examples/README.md) | n/a | 2026-08-18 | high |
| [21] | bench 仅 get/emit/start | [crates/plugctx/benches/core_paths.rs](../../../../crates/plugctx/benches/core_paths.rs) | 2026-08-18 | 2026-08-18 | high |

## Staleness map

| claim class | window | note |
| --- | --- | --- |
| versions/compatibility（crates.io 是否空闲/已上架） | ≤ 1 month | 最早复查：2026-09-18 |
| release 元数据（repository、GHA） | ≤ 1 month | 与代码同步，改完即失效 |
| capability-inventory（需求正文） | ≤ 12 months | 需求若再改「已交付」注释需 Refresh |
| implementation（HEAD API） | 随仓库 | 下一 major 前 Refresh |

最早复查日：**2026-09-01**（crates.io 占名与发布元数据窗口）。
