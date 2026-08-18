# crates.io 发布元数据与边界（FR51）

**现状（2026-08-18）：** [`plugctx`](https://crates.io/crates/plugctx) 与 [`plugctx-derive`](https://crates.io/crates/plugctx-derive) **0.1.2 已上架**（0.1.0 / 0.1.1 仍保留）。本文档列出必填（或推荐）元数据、工作区内 **不可发布** 成员，以及**后续版本**的发版顺序。

## 公开 crate 必填 / 等价项

| 字段 | 要求 | 本仓库落点 |
|------|------|------------|
| `license` | crates.io 必填 | `workspace.package.license` → 两 crate 继承 |
| `description` | crates.io 必填 | 各 crate `[package].description` |
| `repository` | crates.io 推荐；本仓库必填 | `workspace.package.repository` = `https://github.com/TangCan/plugin-system`（`plugctx` / `plugctx-derive` 继承） |
| `documentation` | 推荐 | `https://docs.rs/<crate>` |
| 路径依赖 `version` | 发布时必填 | `[workspace.dependencies]` 中 `plugctx` 等带 `version` |

## `publish = false` 边界

下列成员/客人**不得**上架（fixture / 内部脚手架 / 样例）：

| 成员 | 说明 |
|------|------|
| `plugin-api` | C ABI 脚手架；ABI 正文由 `plugctx` 的 `c_abi` 同源 `include!` |
| `plugin-host` | 演示 CLI |
| `hello_plugin` / `echo_plugin` | 示例 `cdylib` |
| `wasm_echo`（独立 workspace） | Extism PDK fixture |
| `wit-sample-guest`（非 workspace member） | wasip2 样例客人 |
| `plugctx-examples`（`examples/`） | 工作区演示包（derive / wasm / component） |

## Dry-run

在 `plugin-system/` 下，推荐与 CI 一致（Cargo ≥1.90）：

```bash
cargo publish --workspace --dry-run
```

亦可单独验证主包：

```bash
cargo publish -p plugctx --dry-run
```

> **注意：** `plugctx` 已上架后，单独 `cargo publish -p plugctx-derive --dry-run` 可从 crates.io 解析 `plugctx`。**后续发版仍须先发 `plugctx` 再发 `plugctx-derive`**（derive 依赖同版本已上架的 `plugctx`）。若只 bump 了 workspace 版本却先发 derive，registry 上尚无新 `plugctx` 时 dry-run / publish 会失败。首次占名阶段曾因 `plugctx` 不存在而无法单独 dry-run derive——属当时预期。

默认 features 下 `plugctx` **不**依赖未上架的 workspace 成员（`dynamic-native` 使用包内 `c_abi`，不再 path-依赖 `plugin-api`）。

> 包名曾为 `pluggable`，因 crates.io 上已有无关方占用而改为 **`plugctx`**（见下文 FR54）。本库 **不是** crates.io 上的他方 `pluggable`。

## 上架记录与后续发版清单

**已完成（2026-08-18）：** 维护者手工 `cargo publish -p plugctx`，再 `cargo publish -p plugctx-derive`，均为 **0.1.0**。crates.io 同一 crate 版本不可重复上传；`yank` ≠ 删除。

后续版本：

1. bump `workspace.package.version`（`plugctx` 与 `plugctx-derive` **锁步**）。
2. `./scripts/ci-publish-dry-run.sh` 非零失败必须阻断。
3. 干净工作树上去掉 `--allow-dirty` 再跑一次 dry-run。
4. **先发 `plugctx`，再发 `plugctx-derive`**（derive 依赖已上架的同版本 `plugctx`）。
5. 不要把 `plugin-api`、host、示例、WIT guest 标成可发布（保持 `publish = false`）。

## 空 default 与 docs.rs 构建子集（FR52 / NFR14）

| 项 | 约定 |
|----|------|
| `default` | **空**（`default = []`）：默认同步核心不拉 Extism / libloading / wasmtime |
| 重能力 | 仅具名 feature + `dep:`：`dynamic-native`→`libloading`，`dynamic-wasm`→`extism`，`dynamic-wasm-component`→`wasmtime` |
| docs.rs | `[package.metadata.docs.rs]` **不用** `all-features` |

**docs.rs 安全子集**（与 `crates/plugctx/Cargo.toml` 一致）：

```toml
[package.metadata.docs.rs]
features = ["async", "parallel", "thread-safe", "tracing", "stages"]
targets = ["x86_64-unknown-linux-gnu"]
```

两公开 crate **均不得** `all-features = true`。`plugctx-derive` 无可选 features，只声明 `[package.metadata.docs.rs]`（含 `targets`，不含 all-features）。

**排除** `dynamic-native` / `dynamic-wasm` / `dynamic-wasm-component`：docs.rs 环境易因 Extism / libloading / wasmtime 系统依赖失败；这些 API 仍可通过本地 `--features` 与 Feature 矩阵文档查阅（见 [`feature-matrix.md`](feature-matrix.md)）。

本地对齐 docs.rs 构建：

```bash
cargo doc -p plugctx --no-deps --features async,parallel,thread-safe,tracing,stages
```

验收：`cargo test -p plugctx --test acceptance_story_9_2`。

## Release 工作流（FR53 / NFR13）

目标：发布可重复；CI 至少阻断失败的 `cargo publish --dry-run`；实际上架遵守 crates.io 约束。

### CI dry-run 门禁

在 `plugin-system/`：

```bash
./scripts/ci-publish-dry-run.sh
# 或完整门禁（已接入 dry-run）：
./scripts/ci-test.sh
```

脚本使用 `set -euo pipefail`：`cargo publish --workspace --dry-run` 失败即以非零退出**阻断流水线**（FR53）。  
`--workspace`（Cargo ≥1.90）只打包可发布成员；`publish = false` 的 fixture 自动跳过。  
CI / 脏工作树使用 `--allow-dirty`；**真正 upload 前**须在干净树、无该旗标下再跑一次。

可选 GitHub Actions 示例：仓库根 [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)（调用 `ci-test.sh`）。

### 鉴权：registry token 或 trusted publishing

| 方式 | 用途 | 注意 |
|------|------|------|
| `CARGO_REGISTRY_TOKEN` | 本机 `cargo publish`、新 crate **首次**占名 | 最小权限；泄漏后**轮换 token**，**不以 yank 代替轮换**（NFR13） |
| Trusted Publishing（crates.io ↔ GitHub OIDC） | **后续版本**的默认 CI 发版 | 短时 token 约 **30** 分钟；不把长期 registry token 存进 GitHub secrets |

迁移期两种方式可**并存**（本机 token 与 CI OIDC 不互斥）。后续发版以 Trusted Publishing 为默认路径，不以仓库 secret `CARGO_REGISTRY_TOKEN` 为默认。

### Trusted Publishing 可执行步骤（后续发版）

公开 crate 首次占名已完成。后续版本用 tag 触发 [`.github/workflows/release.yml`](../.github/workflows/release.yml)（OIDC，`id-token: write`）。

1. crates.io 打开 [`plugctx`](https://crates.io/crates/plugctx) 与 [`plugctx-derive`](https://crates.io/crates/plugctx-derive) → Settings → **Trusted Publisher**（Trusted Publishing）→ Add GitHub：
   - Repository owner / name：`TangCan` / `plugin-system`
   - Workflow filename：**`release.yml`**（必须与仓库文件名精确一致）
   - 两个 crate **都要**配（OIDC 只给已配置的 crate 发 token）
2. 发版前：`./scripts/ci-publish-dry-run.sh` 绿；干净工作树再 dry-run 一次（无 `--allow-dirty`）。
3. bump `workspace.package.version`（两 crate 锁步），打 tag：`git tag v0.1.2 && git push origin v0.1.2`（版本号按 0.y.z，不必写成 `0.2.0`）。
4. `release.yml` 用 `rust-lang/crates-io-auth-action` 换约 30 分钟 token，**先** `cargo publish -p plugctx` **再** `cargo publish -p plugctx-derive`。
5. 不在该工作流里使用 `secrets.CARGO_REGISTRY_TOKEN`。

crates.io Trusted Publishing 正文：<https://crates.io/docs/trusted-publishing>

### 新 crate 名：首次须手工发布

对**从未上架过的 crate 名**，crates.io 要求至少一次**手工** `cargo publish`（或等价人工确认）建立所有权；**不可**仅靠 trusted publishing / 纯 CI 完成首次创建（FR53）。

本工作区公开包 `plugctx`、`plugctx-derive` 的**首次手工发布已完成**（2026-08-18，`0.1.0`）。后续版本可用 token 或 trusted publishing。若将来另起新 crate 名，仍须对该名再做一次手工首次 publish。

> 诚实现状：曾用名 `pluggable` 在 crates.io 上已被无关方占用；现名 `plugctx` / `plugctx-derive` 已采用并上架 `0.1.0`。2026-08-17 API 探测曾为 404（404 ≠ 预订）；占名窗口已关闭。

### 速率限制与永久发布（NFR13）

- crates.io 对**新 crate / 新版本**有速率限制；密集试发易 429，应退避而非硬刷。
- 发布**永久**：`yank` ≠ 删除；yank 只阻止新依赖解析，已有 lockfile / 已下载副本仍在。密钥泄露靠**轮换 token**，不要指望 yank「收回」包。

### crates.io README 不可变与 docs.rs 重建（FR6）

crates.io 该版本页上的 README 来自该版本 `.crate` 里打包的 `readme` 文件，**绑死在该版本**。已 `cargo publish` 之后，改 Git 里的 README **不会**改掉已上架的那一版展示文案；要更新必须 **bump 版本再 publish**。

举例（0.y.z，**不要**为此写成 `0.2.0`）：例如 **0.1.1 → 0.1.2**；`plugctx` 与 `plugctx-derive` **锁步**同一版本。之后若再改该版本 README，须继续 bump。

`yank` 仍然 ≠ 删除（见上节）：只阻止**新**依赖解析选中该版本；已有 lockfile 与已下载副本仍在。密钥泄露靠**轮换 token**，不以 yank 代替。

crates.io 版本列表可以触发 **docs.rs 重建**：只刷新该版本的 **rustdoc** HTML，**不**替换该版本 `.crate` 内的 README。想换 README 仍须升版本。

### release-plz（或等价）操作说明

推荐 [release-plz](https://release-plz.dev/docs/github/quickstart)（GitHub Action + PR 发版），亦可用 `cargo-release` 等等价工具。最小流程：

1. **本地/CI**：`./scripts/ci-publish-dry-run.sh` 绿。
2. **首次每个新名**：维护者本机 `cargo login` 后手工 `cargo publish -p <crate>`（`plugctx` / `plugctx-derive` 已做过；见上节）。
3. **后续版本（默认）：** 按上文「Trusted Publishing 可执行步骤」推 `v*` tag；`release.yml` 走 OIDC。本机仍可用 API token（并存），但 CI 发版不以长期 `CARGO_REGISTRY_TOKEN` 为默认。
4. **文档：** 发版 PR 同步 `CHANGELOG.md` 与工作区 `version`（能力清单 vs SemVer 见下文 FR54）。

验收：`cargo test -p plugctx --test acceptance_story_9_3`。

## 0.y 版本策略、锁步与改名（FR54）

### SemVer `0.y` vs 能力清单

| 项 | 约定 |
|----|------|
| 加性能力 | 新 Cargo feature / 文档能力可留在**同一** `0.y` |
| Breaking | 破坏性（breaking）变更才 bump `0.y`（或按 `0.y.z` 惯例） |
| CHANGELOG `[0.2.0]` | **能力清单**标题，**不等于**强制把 `workspace.package.version` 写成 `0.2.0` |
| 当前工作区 | `version = "0.1.2"` 可同时承载 0.2 清单已交付的扩展（见 [`CHANGELOG.md`](../CHANGELOG.md)） |

### `plugctx` ↔ `plugctx-derive` 版本耦合

- 两 crate 共享 `[workspace.package].version`，**锁步**发布：同一次发版使用**同一版本号**。
- `plugctx-derive` 仅在 `dev-dependencies` 依赖路径上的 `plugctx`（宏 crate 本身不强制运行时依赖）；对外文档仍要求消费方将二者视为**同版本配套**。
- 若将来拆分兼容范围，须在本段显式改为范围依赖并更新本验收；**当前策略是锁步，不是宽松 `^` 矩阵**。

### crates.io 包名：`pluggable` → `plugctx`（诚实现状）

| 名称 | 状态（决策依据） |
|------|------------------|
| `pluggable` | crates.io **已占用**（无关方 `0.1.0` async plugin system）——**不可**作为本库上架名 |
| `plugctx` / `plugctx-derive` | **已采用并上架**；当前最新 **0.1.2**（2026-08-18）。0.1.0 / 0.1.1 仍保留。更早 2026-08-17 API 探测为 404（研究卷宗 `technical-pluggable-crate-rename-2026-08-17`）；该空闲探测已过期。 |

**后续发版：**

1. **复验当前版本**：`GET https://crates.io/api/v1/crates/plugctx` 应返回已发布 crate；新版本须 bump `workspace.package.version` 后再 `cargo publish`（`0.1.0` 已存在，不可重传）。
2. 锁步：同一次发版两 crate 使用同一版本号；**先发 `plugctx` 再发 `plugctx-derive`**。
3. 若将来不得不改名：另选未占用名，更新 `[package].name` 与文档（新名仍须首次手工 publish）。
4. README / docs 须一句区分：本库 **不是** crates.io 上的历史包名 `pluggable`（他方）。

验收：`cargo test -p plugctx --test acceptance_story_9_4`。
