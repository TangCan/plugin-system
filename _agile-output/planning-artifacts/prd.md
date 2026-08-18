---
title: plugctx 落地补齐（发布、文档、示例、native 热插拔）
created: 2026-08-18
updated: 2026-08-18
status: draft
scope: remaining-gaps-2026-08
---

# PRD: plugctx 落地补齐

*工作标题：把已实现内核送出仓库门，并兑现引言中的 native 热插拔。*

## 0. Document Purpose

给 Richard（产品/实现）、后续史诗拆分与开发代理使用。本文**不**重写已交付的同步内核；只覆盖缺口调研中「真正还没落地」的项，并纳入用户确认的 **native 真正 `dlclose` / 换新 `.so`**（将修订既有 FR25）。词汇以 §3 Glossary 为准。上游依据：`docs/requirements/`、`docs/publishing.md`、`docs/feature-matrix.md`、`docs/api-freeze.md`、`CHANGELOG.md`、`README.md`、缺口调研 `research.md`。无 UX 合同（库 + 示例，无产品 UI）。

## 1. Vision

`plugctx` 内核与扩展 feature 已在仓库内可测。外部 Rust 用户仍无法 `cargo add plugctx`，也没有托管 CI、用户指南，以及 CLI / Web / 游戏三条「像应用一样用」的示例。引言承诺的「动态库热插拔」被 FR25（逻辑卸载 ≠ `dlclose`）收窄，与用户现在的产品决定不一致。

本切片让框架**可发布、可被 CI 守门、可被读懂、可被三个场景抄作业**，并让 native 插件在卸载后真正释放动态库映射，从而加载新的 `.so`。

## 2. Target User

### 2.1 Jobs To Be Done

- **功能：** 把自研能力打成可替换的 native 插件，运行中卸掉旧库、换上新库。
- **功能：** 用 crates.io 依赖，而不是 git path。
- **功能：** 从用户指南和场景示例复制出 CLI / Web / 游戏接入方式。
- **情感：** 相信 `dispose` 之后旧代码不会还占着进程映射。
- **社会：** 仓库在 GitHub Actions 绿了再合并。

### 2.2 Non-Users (v1)

- 不需要跨 DSO 传 `dyn Trait` 或不稳定 Rust ABI 的用户（仍不以 `abi_stable` 为基线）。
- 不把本切片当成 WASM 热替换（WASM 仍走实例 `close`/`free`，FR26 不变）。

### 2.3 Key User Journeys

- **UJ-1. 维护者上架并让 CI 守门。** 补 `repository`，dry-run 绿，首次手工 `cargo publish -p plugctx` 再发 `plugctx-derive`；每次 push 跑 GitHub Actions（含 clippy）。
- **UJ-2. 应用作者读指南并跑通示例。** 打开用户指南，分别跑 CLI / Web / 游戏示例，看到插件安装、调用、卸载。
- **UJ-3. 插件作者热替换 native 库。** 加载 `libfoo.so`，调用成功；`PluginHandle::dispose`（或等价卸载）后映射释放；构建新的 `libfoo.so`（或另一路径）再 `load`，调用得到新行为，旧符号不可用。

## 3. Glossary

- **逻辑卸载** — 撤销 Context 注册与 Effect，不保证 `dlclose`。这是**旧** native 正确性前提（FR25），本 PRD 将其废止于 native 路径。
- **物理卸载** — native `Library` Drop 导致的 `dlclose`（或平台等价），映射不再可用于调用。
- **热插拔** — 物理卸载之后，再加载**同一路径或新路径**的 cdylib，得到独立的新实例。
- **公开 crate** — `plugctx`、`plugctx-derive`。
- **用户指南** — `docs/` 下面向应用作者的叙述文档，不是 rustdoc 索引，也不是需求正文。
- **场景示例** — 可 `cargo run` 的小型应用：CLI、Web、游戏各一，`publish = false`。

## 4. Features

### 4.1 Native 热插拔（修订 FR25）

**Description:** native 动态插件在 Context 级精确卸载或适配器销毁时必须物理卸载。随后允许再加载新 `.so`。实现 UJ-3。`[ASSUMPTION: 热插拔 API 以现有 load + dispose + load 组合交付，本切片不强制新增 reload() 方法，除非架构证明没有它无法安全 dlclose。]`

**Functional Requirements:**

#### FR-1: Native 物理卸载

启用 `dynamic-native` 的宿主在 `PluginHandle::dispose` 成功（以及 Context `dispose` 级联卸掉该插件）之后，对应 `libloading::Library` 被 Drop，平台执行 `dlclose`（Windows `FreeLibrary` / macOS `dlclose`）。

**Consequences:**
- 卸载后对已失效 `NativeInvoker` 的 `call` 返回可诊断错误，不进程崩溃为成功路径。
- 测试可用平台手段或探针证明映射不再作为该插件入口（至少：再 load 同一路径得到新实例且旧 invoker 不可用）。

#### FR-2: 换新 .so 再加载

物理卸载完成后，宿主可用 `load_native_plugin` / `DylibLoader` 加载同一路径覆盖后的制品或另一路径制品，并 `Context::plugin` 安装。

**Consequences:**
- 验收：先 load hello 行为 A → dispose → 替换/加载 echo 或重建后的 hello 行为 B → call 得到 B。
- ABI 不匹配仍返回 `Error::AbiMismatch`，且不 `create`/`init`（NFR6 不变）。

#### FR-3: 文档与测试同步修订 FR25

所有声称「native 逻辑卸载 ≠ dlclose」的用户可见文档（README、feature-matrix、requirements §4.3、testing、CHANGELOG）改为物理卸载契约；保留 WASM FR26 不变。

**Consequences:**
- 旧验收若断言「保留 Library 映射」必须改写或删除。
- CHANGELOG Unreleased 记录 breaking：`dynamic-native` 卸载语义。

### 4.2 发布元数据与 crates.io

**Description:** 公开 crate 可被 crates.io 与 docs.rs 识别。实现 UJ-1。

#### FR-4: repository 字段

`plugctx` 与 `plugctx-derive`（经 workspace 继承或各自声明）写入真实 `repository`（及可选 `homepage`），指向当前 GitHub 远端。

#### FR-5: 上架就绪与首次发布清单

`docs/publishing.md` 反映真实 origin；`cargo publish --workspace --dry-run` 仍阻断失败。维护者可按「先 `plugctx` 后 `plugctx-derive`」完成首次手工发布。本切片**要求交付可执行发布清单与元数据**；若环境无 crates.io token，故事 AC 以 dry-run + 清单为准，实际上传可记为维护者操作。

**Consequences:**
- publishing.md 不再写「本地暂无 origin」。
- 复验 crates.io 名仍空闲（404 或无 already exists）。

### 4.3 托管 CI

**Description:** 每次提交在 GitHub Actions 跑与本地等价的门禁。实现 UJ-1。

#### FR-6: GitHub Actions 工作流

仓库根存在 `.github/workflows/ci.yml`（或等价），在 push/PR 调用 `./scripts/ci-test.sh`（或经文档证明的等价拆分 job）。

#### FR-7: clippy 门禁

CI 包含 `cargo clippy`（至少默认 features 的 `plugctx` / 工作区可发布成员）。**禁止**盲目 `cargo test --all-features` 作为唯一测试 job（`thread-safe` 与部分默认同步验收互斥）。扩展矩阵保持 `ci-extension-matrix.sh`。

### 4.4 用户指南

**Description:** 应用作者不读需求正文也能接入。实现 UJ-2。

#### FR-8: 独立用户指南

新增面向用户的指南（建议 `docs/guide.md`），覆盖：最小同步插件、feature 矩阵入口、native 热插拔步骤、动态 WASM 与核心路径的差异、链接到三个场景示例。README 链到该指南。

### 4.5 场景示例

**Description:** 三条可运行示例，证明框架能进真实应用形状。实现 UJ-2。`publish = false`。

#### FR-9: CLI 场景示例

可 `cargo run` 的 CLI：安装 ≥1 个插件，执行至少一次用户可见命令，再卸载（native 示例须走物理卸载）。

#### FR-10: Web 场景示例

可 `cargo run` 的最小 HTTP 服务（具体框架由 Architecture 钉死）：请求路径触发 `get`/`emit` 或 native call；文档说明如何启动。

#### FR-11: 游戏场景示例

可 `cargo run` 的最小游戏循环示例（不要求引擎；可以是 tick + 插件驱动的状态机）：插件贡献行为，卸载后行为消失。

## 5. Non-Goals (Explicit)

- 不以 `abi_stable` 为基线；不跨 DSO 传递不稳定 `dyn Trait`。
- 不改默认同步内核 API 冻结面（除 `dynamic-native` 卸载语义 breaking）。
- 不把 WASM 卸载改成「映射级」；FR26 保持实例 close。
- 不要求 `emit_parallel` 的 `buffer_unordered` 并发上限。
- 不在本切片拆 `ContextData`（已有 ADR defer）。
- 不强制 workspace `version` 写成 `0.2.0` 字符串（FR54 仍有效）。
- 不为 Web/游戏示例做生产级引擎或完整网站。

## 6. MVP Scope

### 6.1 In Scope

FR-1 … FR-11。

### 6.2 Out of Scope for MVP

- crates.io trusted publishing / release-plz 全自动发版（有文档即可）。
- Windows CI 矩阵（Architecture 可声明仅 ubuntu 托管，Windows `FreeLibrary` 仍须单测或 cfg 覆盖）。
- 独立 `reload()` 糖 API（除非架构认定必需）。

## 7. Success Metrics

**Primary**
- **SM-1**：`dynamic-native` 验收证明 dispose 后可换新 `.so` 且旧 invoker 失效。Validates FR-1, FR-2。
- **SM-2**：GitHub Actions 在默认分支 PR 上绿。Validates FR-6, FR-7。
- **SM-3**：三则场景示例 `cargo run` 有文档命令。Validates FR-9–11。

**Counter-metrics**
- **SM-C1**：不要为热插拔牺牲默认 features 的零 `libloading` 依赖。

## 8. Open Questions

1. Web 示例框架：`tiny_http` vs `axum`（架构钉死，避免重运行时进默认图）。
2. 游戏示例是纯 tick 循环还是最小宏内核。
3. 首次 crates.io 上传是否在本切片由维护者当场执行。

## 9. Assumptions Index

- 热插拔以 load + dispose + load 组合交付，不强制 `reload()`。
- 无 token 时「上架」AC 以 dry-run + 清单为准。
- UX 合同不适用。
