---
stepsCompleted: [1, 2, 3, 4]
inputDocuments:
  - _agile-output/planning-artifacts/prd.md
  - _agile-output/planning-artifacts/architecture.md
  - _agile-output/planning-artifacts/research/technical-plugctx-requirements-gap-2026-08-18/research.md
  - docs/requirements/1. 引言.md
  - docs/requirements/2. 总体架构.md
  - docs/requirements/4. 扩展模块设计.md
  - docs/requirements/6. API 设计概览.md
  - docs/requirements/7. 技术选型与依赖.md
  - docs/requirements/8. 测试策略.md
  - docs/requirements/9. 实施路线图.md
  - docs/requirements/设计方案大纲（v2）.md
  - docs/publishing.md
  - docs/feature-matrix.md
  - docs/api-freeze.md
  - docs/testing.md
  - CHANGELOG.md
  - README.md
---

# plugin-system - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for plugin-system，将 PRD（落地补齐切片）、Architecture Spine 与既有需求/发布文档分解为可实现故事。全部史诗与故事已写入，可供开发。

## Requirements Inventory

### Functional Requirements

FR1: 启用 `dynamic-native` 时，`PluginHandle::dispose` 成功或 Context dispose 卸掉该 native 插件后，必须 Drop `libloading::Library`，触发平台 `dlclose` / `FreeLibrary`（物理卸载）。
FR2: 物理卸载后，对已失效 `NativeInvoker` 的 `call` 返回可诊断 `Error`，不得把 use-after-unmap 当成成功。
FR3: 物理卸载完成后，宿主可再次 `load_native_plugin` / `DylibLoader` 加载同一路径的新制品或另一路径制品，安装进 Context，调用得到新行为。
FR4: ABI 不匹配仍返回 `Error::AbiMismatch` 且不执行 `create`/`init`。
FR5: 用户可见文档（README、feature-matrix、requirements §4.3、testing、CHANGELOG）废除「native 逻辑卸载 ≠ dlclose」（旧 FR25）；WASM FR26 保持实例 close。
FR6: `plugctx` 与 `plugctx-derive` 写入真实 `repository`（可选 `homepage`），publishing.md 不再声称无 origin。
FR7: 保持 `cargo publish --workspace --dry-run` 失败即阻断；提供先发 `plugctx` 再发 `plugctx-derive` 的首次上架清单（无 token 时以 dry-run + 清单为故事完成标准）。
FR8: 仓库提供 GitHub Actions 工作流，在 push/PR 上执行与 `./scripts/ci-test.sh` 等价的门禁。
FR9: CI 含 clippy 门（至少可发布 crate 默认 features，`-D warnings`）；禁止用单独的 `cargo test --all-features` 作为唯一测试门槛。
FR10: 新增面向应用作者的独立用户指南（`docs/guide.md`），README 链接之；须包含最小同步用法、feature 入口、native 热插拔步骤、与 WASM 卸载差异、三条场景示例命令。
FR11: 提供可 `cargo run` 的 CLI 场景示例（`publish = false`）；native 路径演示须走物理卸载。
FR12: 提供可 `cargo run` 的最小 Web/HTTP 场景示例（`publish = false`，依赖不得进入 `plugctx` 默认图）。
FR13: 提供可 `cargo run` 的最小游戏循环场景示例（无引擎 tick + 插件状态；`publish = false`）。

### NonFunctional Requirements

NFR1: `plugctx` 默认 features 仍为零重运行时（不因本切片拉入 libloading/extism/wasmtime/tiny_http）。
NFR2: 不以 `abi_stable` 为基线；禁止跨 DSO 传递不稳定 `dyn Trait`。
NFR3: 热插拔不得以 panic 作为控制流；失败可 `match`。
NFR4: 物理卸载后的再加载必须平台正确处理库文件名（`.so` / `.dylib` / `.dll`）；文档写明 Windows 文件锁时须先 dispose 或换路径。
NFR5: 场景示例均为 `publish = false`，不把 Web/游戏依赖引入公开 crate 默认图。
NFR6: CI 在 ubuntu-latest + stable 上可重复；失败非零退出。
NFR7: 默认同步内核 API 冻结面不因热插拔而恢复 Error 结构化载荷大重构；breaking 仅限 `dynamic-native` 卸载语义。
NFR8: 不强制将 workspace `version` 改为 `0.2.0` 字符串（FR54）。
NFR9: 不盲目 `--all-features` 测试（`thread-safe` 互斥）。
NFR10: WASM 卸载语义（FR26）不变。

### Additional Requirements

- 无 greenfield starter template；在现有 workspace 上演进。
- Architecture AD-1：逻辑卸载（撤销注册）仍先于物理 Drop Library。
- Architecture AD-3：不强制新增 `reload()`；热插拔 = load → use → dispose → load。
- Architecture AD-6：clippy 加入 `ci-test.sh`；GHA 调该脚本。
- Architecture AD-8：Web 示例钉 `tiny_http`；游戏示例为无引擎 tick 循环；CLI 用 std。
- 首次 crates.io 实际上传是维护者操作，可与代码故事分离。
- 托管 CI 本切片只要求 ubuntu；Windows 矩阵推迟，但 Win 卸载 API 仍须 cfg 编译。
- 不在本切片拆分 `ContextData`（既有 ADR defer）。
- 示例与指南使用中文用户文档、英文代码标识符。

### UX Design Requirements

无 UX 设计合同（库 + CLI/示例，无产品 UI）。本切片不提取 UX-DR。

### FR Coverage Map

FR1: Epic 1 - Native 物理卸载 / dlclose
FR2: Epic 1 - 旧 NativeInvoker 失效
FR3: Epic 1 - 换新 .so 再加载
FR4: Epic 1 - ABI 不匹配仍拒绝 create/init
FR5: Epic 1 - 文档废止旧 FR25；WASM FR26 不变
FR6: Epic 2 - 公开 crate 写入 repository
FR7: Epic 2 - publish dry-run 与首次上架清单
FR8: Epic 2 - GitHub Actions 调用 ci-test.sh
FR9: Epic 2 - clippy 门；禁止唯一门槛 --all-features
FR10: Epic 3 - 独立用户指南 docs/guide.md
FR11: Epic 3 - CLI 场景示例（含物理卸载）
FR12: Epic 3 - Web 场景示例（tiny_http）
FR13: Epic 3 - 游戏 tick 循环场景示例

## Epic List

### Epic 1: Native 热插拔
插件作者能卸掉旧动态库并加载新的 `.so`；旧入口不可再调用。
**FRs covered:** FR1, FR2, FR3, FR4, FR5

### Epic 2: 可安装且可守门
维护者能把公开 crate 指到真实仓库，PR 被 GitHub Actions 与 clippy 拦住。
**FRs covered:** FR6, FR7, FR8, FR9

### Epic 3: 可跟做的接入路径
应用作者能按用户指南跑通 CLI、Web、游戏三条示例。
**FRs covered:** FR10, FR11, FR12, FR13

## Epic 1: Native 热插拔

插件作者能卸掉旧动态库并加载新的 `.so`；旧入口不可再调用。本史诗覆盖 FR1–FR5。

### Story 1.1: Native 插件 dispose 时物理卸载

As a 插件宿主开发者,
I want native 插件在精确卸载后释放动态库映射,
So that 旧代码不能再被调用，并为换新 `.so` 腾出进程。

**Acceptance Criteria:**

**Given** 已用 `dynamic-native` 加载并安装 `hello_plugin`，且 `start` 成功
**When** 对该插件 `PluginHandle::dispose`
**Then** 对应 `libloading::Library` 被 Drop（平台 `dlclose` / `FreeLibrary`）
**And** 撤销 Context 注册与 Effect 仍先于 Drop 发生（逻辑卸载顺序不变）

**Given** dispose 已成功
**When** 使用卸载前拿到的 `NativeInvoker`（或等价句柄）再 `call`
**Then** 返回可诊断 `Error`，进程不以成功路径完成该次调用
**And** 不以 panic 作为控制流（NFR3）

**Given** 仅启用默认 features
**When** 运行 `cargo test -p plugctx`
**Then** 不链接 `libloading`（NFR1）

**Given** 旧验收曾断言「保留 Library 映射 / 不以 dlclose 为前提」
**When** 本故事完成
**Then** 那些断言已删除或改写为物理卸载契约（FR1/FR2）

### Story 1.2: 物理卸载后换新 .so 再加载

As a 插件作者,
I want 卸掉旧 native 库之后再加载新的动态库,
So that 进程不必重启就能换上新行为。

**Acceptance Criteria:**

**Given** 已按 1.1 物理卸载某个 native 插件
**When** 用 `load_native_plugin` 或 `DylibLoader` 加载**另一路径**的合法 cdylib（例如先 hello 后 echo），再 `Context::plugin` 并 `start`（若尚未 start）
**Then** 对新插件 `call` 得到新插件行为
**And** 旧插件行为不再可调用

**Given** 已物理卸载
**When** 在同一路径写入/替换为新制品后再 `load`（Linux/macOS；Windows 若文件锁则文档允许换路径）
**Then** 新实例 `call` 成功且行为来自新制品（FR3、NFR4）

**Given** 制品 `PLUGIN_ABI_VERSION` 与宿主不一致
**When** `load_native_plugin` / `DylibLoader::load`
**Then** 返回 `Error::AbiMismatch`
**And** 不执行 `create`/`init`（FR4）

**Given** `thread-safe` 未启用的默认同步 Context
**When** 完成 unload → load 循环
**Then** 不要求 guest 内多线程；失败路径可 `match` Error（NFR2、NFR3）

### Story 1.3: 文档废止旧 FR25（native 逻辑卸载）

As a 框架使用者,
I want 文档与 CHANGELOG 写明 native 卸载会 `dlclose`,
So that 我不会按「映射一直保留」来设计热插拔。

**Acceptance Criteria:**

**Given** README、`docs/feature-matrix.md`、`docs/requirements/4. 扩展模块设计.md` §4.3、`docs/testing.md`、`CHANGELOG.md` 仍可能写「逻辑卸载 ≠ dlclose」
**When** 本故事完成
**Then** 上述用户可见处改为：native dispose 后物理卸载（Drop `Library`）；热插拔 = load → dispose → load
**And** WASM 仍为实例 `close`/`free`（FR26 / NFR10），不得被改成 dlclose 语义

**Given** `dynamic-native` 卸载语义相对 0.1 行为是 breaking
**When** 更新 CHANGELOG Unreleased
**Then** 明确记录 breaking，且不把默认同步内核标成 breaking（NFR7）
**And** 不强制把 workspace `version` 改成 `0.2.0` 字符串（NFR8）

**Given** 引言「热插拔」目标
**When** 更新 `docs/requirements/1. 引言.md` 中动态加载表述（若仍含混）
**Then** 与物理卸载契约一致，不再暗示「只撤 Context 注册即可换新 .so」

## Epic 2: 可安装且可守门

维护者能把公开 crate 指到真实仓库，PR 被 GitHub Actions 与 clippy 拦住。本史诗覆盖 FR6–FR9。

### Story 2.1: 公开 crate 指向 GitHub 并具备上架清单

As a crate 维护者,
I want `plugctx` / `plugctx-derive` 带上真实 `repository` 和可执行的发布清单,
So that 他人能找到源码，我能按步骤完成首次上架。

**Acceptance Criteria:**

**Given** 远端为 `https://github.com/TangCan/plugin-system`
**When** 查看 `plugctx` 与 `plugctx-derive` 的 package 元数据
**Then** `repository` 为该 URL（workspace 继承或各自声明均可）
**And** `docs/publishing.md` 不再写「本地暂无 origin」（FR6）

**Given** 工作区可发布成员
**When** 运行 `./scripts/ci-publish-dry-run.sh`（或文档中的等价 `cargo publish --workspace --dry-run`）
**Then** 失败必须以非零退出阻断
**And** 清单写明先发 `plugctx` 再发 `plugctx-derive`；无 token 时本故事以 dry-run + 清单为完成标准（FR7）

**Given** crates.io 包名约束
**When** 更新 publishing 说明
**Then** 仍要求上架前复验 `plugctx` / `plugctx-derive` 空闲
**And** 不把 `plugin-api`、示例、host 标成可发布（既有 `publish = false`）

### Story 2.2: GitHub Actions 与 clippy 门禁

As a 贡献者,
I want 每次 push/PR 在托管 CI 上跑格式、测试、clippy 和发布 dry-run,
So that 坏变更在合并前被拦住，而不靠本机记命令。

**Acceptance Criteria:**

**Given** 仓库根目录
**When** 本故事完成
**Then** 存在 `.github/workflows/ci.yml`（或等价），在 `push` 与 `pull_request` 于 `ubuntu-latest` 调用 `./scripts/ci-test.sh`（或经文档证明的等价拆分 job）（FR8、NFR6）

**Given** `scripts/ci-test.sh`
**When** 执行该脚本
**Then** 包含对可发布 crate 默认 features 的 `cargo clippy`（`-D warnings`）
**And** **没有**把单独的 `cargo test --all-features` 当作唯一测试门槛
**And** 扩展矩阵仍走 `ci-extension-matrix.sh`（FR9、NFR9）

**Given** clippy/测试失败
**When** CI job 结束
**Then** 工作流非零失败，不得标绿

## Epic 3: 可跟做的接入路径

应用作者能按用户指南跑通 CLI、Web、游戏三条示例。本史诗覆盖 FR10–FR13。

### Story 3.1: CLI 场景示例（含 native 热插拔）

As a 应用作者,
I want 一个可运行的 CLI 示例演示插件安装、调用和物理卸载,
So that 我能抄到自己的命令行工具里。

**Acceptance Criteria:**

**Given** 工作区 `examples/`（`plugctx-examples` 或 `examples/apps`）
**When** 按 README/注释中的命令 `cargo run`（启用所需 `dynamic-native` feature）
**Then** CLI 完成至少一次用户可见操作（例如 greet/echo）
**And** 随后卸载 native 插件（走 Epic 1 物理卸载），进程不以成功路径再调用旧 invoker

**Given** 该示例外包
**When** 查看 Cargo 元数据
**Then** `publish = false`
**And** 不把 `libloading`/示例依赖加入 `plugctx` 的默认 features（NFR1、NFR5）

**Given** Linux 上已构建 `hello_plugin`（及如需的第二插件）
**When** 示例执行 unload →（可选）再 load
**Then** 行为与 FR11 一致：native 路径必须物理卸载，不能只撤 Context 注册

### Story 3.2: Web 场景示例（tiny_http）

As a 应用作者,
I want 一个最小 HTTP 服务示例把请求交给 plugctx,
So that 我能看出插件如何接入 Web。

**Acceptance Criteria:**

**Given** Architecture 钉死的 **tiny_http**
**When** 按文档命令 `cargo run` 启动示例
**Then** 至少一条 HTTP 路径触发 `get` / `emit` 或插件 `call`，响应可观测（状态码 + 正文）
**And** 文档说明启动方式与试请求命令（如 curl）

**Given** `plugctx` 默认依赖图
**When** 本示例启用
**Then** `tiny_http` 只出现在示例外包；`plugctx` `default = []` 不变（NFR1、NFR5）
**And** 示例 `publish = false`

**Given** 服务进程退出
**When** 调用 Context `dispose`（或等价清理）
**Then** 不泄漏未关闭的监听；不以 panic 作为正常关闭路径

### Story 3.3: 游戏 tick 循环场景示例

As a 游戏/玩法作者,
I want 一个无引擎的固定步循环示例由插件改状态,
So that 我能看出插件如何接入游戏循环，而不是先上完整引擎。

**Acceptance Criteria:**

**Given** 无游戏引擎依赖的 tick 循环（Architecture AD-8）
**When** `cargo run` 该示例
**Then** 运行固定步数（或直到明确退出条件），插件在 tick 中改变可观察状态（打印或计数）
**And** 卸载该插件后，后续 tick 不再应用该插件行为

**Given** 示例外包
**When** 查看元数据
**Then** `publish = false`
**And** 不把引擎或重运行时加入 `plugctx` 默认 features（NFR5）

**Given** 循环结束
**When** Context `dispose`
**Then** effect cleanup 执行；正常路径不 panic

### Story 3.4: 独立用户指南

As a 应用作者,
I want 一篇不读需求正文也能上手的指南,
So that 我能从最小插件走到热插拔和三条场景示例。

**Acceptance Criteria:**

**Given** 仓库 `docs/`
**When** 本故事完成
**Then** 存在 `docs/guide.md`，覆盖：最小同步 `Plugin`、feature 矩阵入口、native 热插拔（load → dispose → load / 物理卸载）、与 WASM 实例 close 的差异
**And** 给出 3.1–3.3 的可复制 `cargo run`（及 Web 的试请求）命令（FR10）

**Given** 根 `README.md`
**When** 读者找「如何接入」
**Then** 在「写一个插件」或等价节之前能点到该指南
**And** 指南中文；代码标识符保持英文

**Given** 默认 features 与发布边界
**When** 指南描述动态加载
**Then** 不暗示默认依赖含 libloading/extism/wasmtime
**And** 不把场景示例写成可上架 crate






