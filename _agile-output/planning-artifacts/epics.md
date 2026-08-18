---
stepsCompleted: [1, 2, 3, 4]
inputDocuments:
  - _agile-output/planning-artifacts/prd.md
  - _agile-output/planning-artifacts/architecture.md
  - _agile-output/planning-artifacts/research/technical-rust-in-process-plugin-framework-post-0-2026-08-18/research.md
  - docs/publishing.md
  - docs/testing.md
  - docs/feature-matrix.md
  - docs/api-freeze.md
archivedPriorEpics: _agile-output/planning-artifacts/epics-落地补齐-2026-08-18.md
slice: post-0.1.1-publish-quality
---

# plugin-system - Epic Breakdown

## Overview

本文件将 **0.1.x 已上架之后** 的下一步拆成可实现故事。上一轮「落地补齐」PRD FR-1…FR-11 已交付（归档：`epics-落地补齐-2026-08-18.md`）。本切片的 FR/NFR **重新编号**，来源是结项后的技术调研建议 + 仍生效的 Architecture 不变量；**不是**把已完成的热插拔/指南/示例外再做一遍。

无 UX 合同（库 + 示例，无产品 UI）。无 starter template（brownfield）。

## Requirements Inventory

### Functional Requirements

FR1: 为已上架的 `plugctx` 与 `plugctx-derive` 增加基于 GitHub OIDC 的 Trusted Publishing 发版工作流（tag 触发；`id-token: write`；`rust-lang/crates-io-auth-action` 或等价；约 30 分钟短时 token）。仓库文档写清：crates.io 上须由 owner 配置 Trusted Publisher（repo + workflow 文件名）；**不**把长期 `CARGO_REGISTRY_TOKEN` 当作后续发版的默认路径。
FR2: `docs/publishing.md` 将 Trusted Publishing 写成可执行步骤（工作流路径、发版顺序仍先 `plugctx` 后 `plugctx-derive`、迁移期可与 API token 并存）。明确：首次占名已完成；新 crate 名仍须手工 token 发布一次。
FR3: CI 用 `cargo-hack`（`--feature-powerset` 或 `--each-feature`）表达互斥 feature：至少声明 `thread-safe` 与默认同步验收互斥（`--mutually-exclusive-features`）；多 feature 时 `--exclude-all-features`，避免工具隐式再跑 `--all-features`。禁止把单独的 `cargo test --all-features` 作为唯一测试门槛（延续既有门禁）。
FR4: 中文用户文档（至少 `docs/guide.md`；`docs/requirements/4. 扩展模块设计.md` §4.3 与之一致）写清 native 物理卸载限度：`FreeLibrary`/`dlclose` 成功 ≠ 文件一定可覆盖；macOS 带非平凡 TLS 析构的 dylib 可能永不卸载；sound 卸载要求无残留引用/出站函数指针/库内线程。热插拔仍为 load → use → dispose → load。
FR5: `docs/component-model-versions.md`（及 WIT guest README）钉死**当前工具链实际使用的 WIT 版本**；在 wasmtime / wit-bindgen 刷新到已发布 `wasi@0.3.0` 之前，**禁止**把样例 guest 改钉到该发布标签。保持 Extism（`dynamic-wasm`）与 Component Model（`dynamic-wasm-component`）分 feature、分制品。
FR6: 发布文档写明：要更新 crates.io 上该版本展示的 README，必须 bump 版本再 publish；`yank` ≠ 删除；docs.rs 从 crates.io 触发的重建只刷新 rustdoc，不替换该版本 `.crate` 内的 README。
FR7: `plugctx-derive` 如需 docs.rs 定制，使用 `[package.metadata.docs.rs]`；**两公开 crate 均不得** `all-features = true`。`plugctx` 现有轻量子集（`async,parallel,thread-safe,tracing,stages`，不含 native/wasm/component）必须保持，并由既有验收继续守门。

### NonFunctional Requirements

NFR1: `plugctx` `default = []`；本切片不得把 `extism` / `wasmtime` / `libloading` 拉进默认依赖图；示例保持 `publish = false`。
NFR2: 不以 `abi_stable` 为基线；禁止跨 DSO 传递不稳定 `dyn Trait`。
NFR3: 不新增 `reload()`；不把 `hot-lib-reloader` 收进生产公开 API。
NFR4: WASM 卸载仍为实例 `close`/`free`（或 Component 路径 Drop Store）；不把 WASM 改成 native 式 `dlclose`。
NFR5: 新的公开 `Error` 变体或核心签名必须更新 `docs/api-freeze.md`；核心 `Error` 保持单元变体；`get` / `get_trait` 返回 `Option`。
NFR6: 用户可见文档中文；代码标识符英文。
NFR7: 本切片不强制把 workspace `version` 写成 `0.2.0`；若为让 crates.io README 生效而发版，按 0.y.z 惯例 bump（例如 0.1.1 → 0.1.2），两 crate 锁步。
NFR8: Trusted Publishing 短时 token 到期后不可复用；密钥泄露靠轮换 token，不以 yank 代替。

### Additional Requirements

- 无 starter/greenfield 模板；在现有仓库操作面（`.github/workflows/`、`scripts/`、`docs/`）上增量。
- Architecture AD-1…AD-3 仍约束本切片：dispose 后必须 Drop `Library`；失效 invoker 不可调用；热插拔 = load → dispose → load。
- Architecture AD-6 仍为 ubuntu 托管 CI；**Windows 矩阵仍延期**（本切片不把 Windows GHA runner 列为必须交付）。
- 不把 Fidius 式 trait→稳定 C ABI→签名 WASM 包做成 0.1.x 补丁（新产品史诗，非本切片）。
- 不合并 Extism 字节 ABI 与 Wasmtime Component Model 为单一加载路径。
- 不拆 `ContextData`（既有 ADR defer）。
- `plugctx` 的 docs.rs 轻量子集与「禁止 all-features」已在 Story 9.2 / FR52 落地；本切片 FR7 是保持 + 补 derive，不是重做。
- 既有 `./scripts/ci-test.sh` 与 `ci-extension-matrix.sh` 显式矩阵可保留；cargo-hack 是新增或替换门，不得削弱 clippy `-D warnings` 与 fmt。

### UX Design Requirements

无 UX 合同；本切片无 UX-DR。

### FR Coverage Map

FR1: Epic 1 - OIDC Trusted Publishing 发版工作流
FR2: Epic 1 - publishing.md 可执行 Trusted Publishing 步骤
FR3: Epic 1 - cargo-hack 互斥 feature 门禁
FR4: Epic 2 - native 卸载限度写入中文指南
FR5: Epic 2 - WIT pin 与 Extism/CM 分路径
FR6: Epic 1 - crates.io README 不可变；yank ≠ 删除
FR7: Epic 1 - docs.rs 禁止 all-features（含 derive）；保持 plugctx 轻量子集

## Epic List

### Epic 1: 维护者可以安全发下一版
发完后：用 GitHub OIDC 发版（不再默认长期 registry token）；CI 用 cargo-hack 测互斥 feature；docs.rs 按轻量子集构建；publishing 写清 yank ≠ 删除、改 crates.io README 必须升版本。
**FRs covered:** FR1, FR2, FR3, FR6, FR7

### Epic 2: 应用作者能按真实限度用 native / WASM
读完后：知道 `dlclose`/`FreeLibrary` 成功 ≠ 一定能覆盖文件、macOS TLS 可能永不卸载、sound 卸载要无残留引用；WIT 跟当前工具链 pin，不提前钉 `wasi@0.3.0`；Extism 与 Component Model 继续分家。热插拔仍是 load → dispose → load。
**FRs covered:** FR4, FR5

## Epic 1: 维护者可以安全发下一版

发完后：用 GitHub OIDC 发版（不再默认长期 registry token）；CI 用 cargo-hack 测互斥 feature；docs.rs 按轻量子集构建；publishing 写清 yank ≠ 删除、改 crates.io README 必须升版本。

### Story 1.1: Trusted Publishing 发版工作流

As a crate 维护者,
I want 用 GitHub OIDC 从 tag 发布 `plugctx` 与 `plugctx-derive`,
So that 后续发版不必把长期 `CARGO_REGISTRY_TOKEN` 当作默认路径。

**Acceptance Criteria:**

**Given** `plugctx` 与 `plugctx-derive` 已在 crates.io 上架
**When** 仓库增加 tag 触发的 GitHub Actions 工作流（例如 `.github/workflows/release.yml`）
**Then** 该 job 声明 `permissions: id-token: write`，用 `rust-lang/crates-io-auth-action`（或 crates.io 文档中的等价 OIDC 交换）取得短时 token，并按先 `plugctx` 后 `plugctx-derive` 的顺序 `cargo publish`
**And** 工作流**不**把仓库 secret 里的长期 `CARGO_REGISTRY_TOKEN` 当作默认发版方式（FR1, NFR8）

**Given** 维护者打开 `docs/publishing.md`
**When** 按其 Trusted Publishing 步骤操作
**Then** 文档写明：crates.io 上须由 owner 配置 Trusted Publisher（仓库 owner/name + **精确** workflow 文件名）；token 约 30 分钟过期；迁移期可与 API token 并存
**And** 写明首次占名已完成；若将来另起新 crate 名，该名仍须手工 token 发布一次（FR2）

**Given** 本切片不实际上传新版本
**When** 故事验收
**Then** 以工作流文件存在、步骤与 crates.io Trusted Publishing 文档一致、`./scripts/ci-publish-dry-run.sh` 仍绿为准；不要求在本故事内执行真实 `cargo publish`

### Story 1.2: cargo-hack 互斥 feature 门禁

As a 维护者,
I want CI 用 cargo-hack 表达 `thread-safe` 与默认同步验收的互斥,
So that 不会有人把 `cargo test --all-features` 当成唯一门槛而打红默认同步测试。

**Acceptance Criteria:**

**Given** `thread-safe` 与部分默认同步验收互斥（既有 feature-matrix / `ci-test.sh`）
**When** 回归门禁增加 `cargo hack`（`--feature-powerset` 或 `--each-feature`）
**Then** 至少声明 `--mutually-exclusive-features` 覆盖 `thread-safe` 与会和它冲突的默认同步测试组合
**And** 启用多 feature 时加上 `--exclude-all-features`，避免 cargo-hack 隐式再跑 `--all-features`（FR3）

**Given** 现有 `./scripts/ci-test.sh` 含 fmt、clippy `-D warnings`、扩展矩阵
**When** 接入 cargo-hack
**Then** 不得删除或削弱这些门；不得把单独的 `cargo test --all-features` 作为唯一测试 job
**And** `docs/testing.md`（及如有需要的 README 回归说明）写明如何本地跑该 hack 命令

**Given** Architecture AD-6 仍为 ubuntu 托管
**When** 本故事完成
**Then** 不要求增加 Windows GHA runner

### Story 1.3: crates.io README 不可变与 yank 说明

As a 维护者,
I want 发布文档写清「改该版本在 crates.io 上的 README 必须升版本」,
So that 不会误以为 yank 或 docs.rs 重建能改掉已发布制品。

**Acceptance Criteria:**

**Given** 某版本已 `cargo publish`
**When** 维护者阅读 `docs/publishing.md`（及若 README 有发布节则同步一句）
**Then** 文档写明：crates.io 展示的 README 绑在该版本 `.crate` 上，要更新必须 bump 版本再 publish（FR6）
**And** 写明 `yank` ≠ 删除：yank 只阻止新解析，已有 lockfile / 已下载副本仍在；密钥泄露靠轮换 token，不以 yank 代替（NFR8）

**Given** crates.io 可从版本列表触发 docs.rs 重建
**When** 文档描述该能力
**Then** 写明重建只刷新 rustdoc，**不**替换该版本 `.crate` 内的 README（FR6）

**Given** NFR7：不强制 workspace 写成 `0.2.0`
**When** 文档举例为让 README 生效而发版
**Then** 使用 0.y.z 惯例（例如 0.1.1 → 0.1.2），两 crate 锁步；本故事不执行实际上架

### Story 1.4: docs.rs metadata 守门

As a crate 使用者,
I want docs.rs 按可同时打开的轻量 feature 构建文档,
So that 文档构建不会因 `all-features` 拉进 native/wasm 运行时而失败。

**Acceptance Criteria:**

**Given** `plugctx` 已有 `[package.metadata.docs.rs]` 轻量子集 `async,parallel,thread-safe,tracing,stages`（不含 native/wasm/component）
**When** 本故事验收既有 `acceptance_story_9_2`（或等价护栏）
**Then** 该子集仍在，且规范化后不得出现 `all-features=true`（FR7）

**Given** `plugctx-derive` 当前无 `[package.metadata.docs.rs]`
**When** 需要 docs.rs 定制或为与核心 crate 对齐而补表
**Then** 使用 `[package.metadata.docs.rs]`；**不得**设 `all-features = true`（FR7）
**And** 若 derive 无需额外 features，可只显式禁止 all-features 或保持默认（无 all-features 旗标），并在 `docs/publishing.md` 一句说明两公开 crate 的 docs.rs 约定

**Given** NFR1 空 default
**When** 本故事改 Cargo.toml metadata
**Then** 不得把 `libloading` / `extism` / `wasmtime` 拉进 `plugctx` 默认依赖图

## Epic 2: 应用作者能按真实限度用 native / WASM

读完后：知道 `dlclose`/`FreeLibrary` 成功 ≠ 一定能覆盖文件、macOS TLS 可能永不卸载、sound 卸载要无残留引用；WIT 跟当前工具链 pin，不提前钉 `wasi@0.3.0`；Extism 与 Component Model 继续分家。热插拔仍是 load → dispose → load。

### Story 2.1: native 卸载限度写入用户指南

As a 应用作者,
I want 中文指南写清物理卸载的平台限度,
So that 我不会在 `dispose` 成功后仍无法覆盖 `.dll`，或误以为一定能 `dlclose`。

**Acceptance Criteria:**

**Given** 现有 `docs/guide.md` 已写 load → use → dispose → load、无 `reload()`、Windows 可能锁文件
**When** 本故事补充卸载限度
**Then** 至少写明三点：（1）`FreeLibrary`/`dlclose` 成功 ≠ 文件一定可覆盖；（2）macOS 上带非平凡 TLS 析构的 dylib 可能永不卸载；（3）sound 卸载要求无残留引用、出站函数指针、库内线程（FR4）
**And** `docs/requirements/4. 扩展模块设计.md` §4.3 与指南一致，不互相矛盾

**Given** Architecture AD-1…AD-3 与 NFR3
**When** 本故事改文档
**Then** 不新增 `reload()` API，不把 `hot-lib-reloader` 写成生产公开 API
**And** 不把 WASM 卸载改成 native 式 `dlclose`（NFR4）

### Story 2.2: WIT 版本钉死与双 WASM 路径

As a 使用 Component Model 适配器的宿主作者,
I want 文档钉死**当前工具链实际使用的 WIT / wasmtime / wit-bindgen 版本**,
So that 我不会把样例 guest 提前钉到已发布的 `wasi@0.3.0` 而导致实例化失败。

**Acceptance Criteria:**

**Given** 现有 `docs/component-model-versions.md` 钉 wasmtime 47.x、wit-bindgen 0.60.x、`wasm32-wasip2`
**When** 本故事对照当前 workspace / 客人工具链复核
**Then** 文档写明**实际** WIT/WASI pin（与 `guests/wit-sample/` 及检入 `.wasm` 一致）；在 wasmtime / wit-bindgen 刷新到已发布 `wasi@0.3.0` 之前，**禁止**把样例 guest 改钉该发布标签（FR5）
**And** `guests/wit-sample/` README（或等价）含同一 pin 与「不要提前改钉」的说明

**Given** Extism 字节 ABI 与 Wasmtime Component Model 不能互相加载
**When** 文档描述两条动态 WASM 路径
**Then** 保持 `dynamic-wasm` 与 `dynamic-wasm-component` 分 feature、分制品；禁止暗示一份 `.wasm` 两吃（FR5）
**And** WASM 卸载仍为实例 `close`/`free` 或 Component 路径 Drop Store，不写成 native `dlclose`（NFR4）

**Given** NFR1
**When** 本故事完成
**Then** 不把 `extism` / `wasmtime` 拉进 `plugctx` 默认依赖图；不把 Fidius 式签名包做成补丁
