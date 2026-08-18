---
id: "1.1"
key: "1-1-trusted-publishing-发版工作流"
status: done
story_num: "1.1"
epic: 1
---

# Story 1.1: Trusted Publishing 发版工作流

Status: done

Ultimate context engine analysis completed - comprehensive developer guide created.

## Story

As a crate 维护者,
I want 用 GitHub OIDC 从 tag 发布 `plugctx` 与 `plugctx-derive`,
so that 后续发版不必把长期 `CARGO_REGISTRY_TOKEN` 当作默认路径。

## Acceptance Criteria

1. **Given** `plugctx` 与 `plugctx-derive` 已在 crates.io 上架  
   **When** 仓库增加 tag 触发的 GitHub Actions 工作流（`.github/workflows/release.yml`）  
   **Then** job 声明 `permissions: id-token: write`，用 `rust-lang/crates-io-auth-action`（或 crates.io 文档等价 OIDC 交换）取得短时 token，并按先 `plugctx` 后 `plugctx-derive` 的顺序 `cargo publish`  
   **And** 工作流**不**把 `secrets.CARGO_REGISTRY_TOKEN` 当作默认发版方式（FR1, NFR8）

2. **Given** 维护者打开 `docs/publishing.md`  
   **When** 按其 Trusted Publishing 步骤操作  
   **Then** 文档写明：crates.io 上须由 owner 配置 Trusted Publisher（仓库 owner/name + **精确** workflow 文件名 `release.yml`）；token 约 30 分钟过期；迁移期可与 API token 并存  
   **And** 写明首次占名已完成；若将来另起新 crate 名，该名仍须手工 token 发布一次（FR2）

3. **Given** 本切片不实际上传新版本  
   **When** 故事验收  
   **Then** 以工作流文件存在、步骤与 crates.io Trusted Publishing 文档一致、`./scripts/ci-publish-dry-run.sh` 仍绿为准；不要求执行真实 `cargo publish`

## Tasks / Subtasks

- [x] 新增 `.github/workflows/release.yml`（AC: #1）
  - [x] `on.push.tags: ['v*']`
  - [x] `permissions.id-token: write`
  - [x] `rust-lang/crates-io-auth-action@v1` → `CARGO_REGISTRY_TOKEN` 仅来自 `steps.auth.outputs.token`
  - [x] `cargo publish -p plugctx` 然后 `cargo publish -p plugctx-derive`
  - [x] 文件中不得出现 `secrets.CARGO_REGISTRY_TOKEN`
- [x] 更新 `docs/publishing.md` 为可执行 Trusted Publishing 步骤（AC: #2）
- [x] 验收测试 `acceptance_story_10_1` 守门；`ci-test.sh` 接入；dry-run 仍绿（AC: #3）

## Dev Notes

- **不要**改 `plugctx` 运行时、`default = []`、或现有 `ci.yml` 测试 job。发布是仓库操作面。
- 已有 `docs/publishing.md`「鉴权：registry token 或 trusted publishing」表格与「首次须手工发布」——本故事把 Trusted Publishing 从「见官方文档」升级为**可执行步骤**，并增加真实 workflow。
- 已有 `acceptance_story_9_3` 护 FR53 dry-run + 文档含 "trusted publishing"。新测试编号 **10.1**，避免覆盖 `acceptance_story_1_1.rs`（旧内核故事）。
- crates.io：每个 crate 的**第一次**发布已完成（0.1.0）；后续可用 OIDC。Trusted Publisher 必须在 crates.io Settings 配 `TangCan/plugin-system` + workflow 文件名 **`release.yml`**（精确匹配，不含路径前缀以外的别名）。
- 官方示例：`id-token: write` + `rust-lang/crates-io-auth-action@v1`；token ~30 分钟。[Source: https://crates.io/docs/trusted-publishing]
- `checkout` 与 CI 一致用 `actions/checkout@v4`；toolchain 用 `dtolnay/rust-toolchain@stable`（与 `ci.yml` 同）。
- **禁止**在 workflow 默认路径使用 `secrets.CARGO_REGISTRY_TOKEN`。迁移期文档可写「本地仍可用 API token」，但 GHA 发版走 OIDC。
- 本故事**不**执行真实 `cargo publish`，**不** bump 版本。
- 发版顺序不可反：derive 依赖同版本已上架的 `plugctx`。[Source: docs/publishing.md]
- NFR1：不得把 libloading/extism/wasmtime 拉进默认图。
- Windows GHA runner 不在范围（AD-6）。

### Project Structure Notes

- 新文件：`.github/workflows/release.yml`（与现有 `ci.yml` 并列）
- 更新：`docs/publishing.md` Release 工作流节
- 测试：`crates/plugctx/tests/acceptance_story_10_1.rs`（读仓库根文件，与 9_3 同模式）
- `scripts/ci-test.sh` 增加 `cargo test -p plugctx --test acceptance_story_10_1`

### References

- [Source: `_agile-output/planning-artifacts/epics.md` Story 1.1]
- [Source: `docs/publishing.md` Release 工作流 / FR53]
- [Source: `.github/workflows/ci.yml`]
- [Source: `crates/plugctx/tests/acceptance_story_9_3.rs`]
- [Source: crates.io Trusted Publishing]

## Dev Agent Record

### Agent Model Used

Composer

### Debug Log References

### Completion Notes List

- Tag 触发 `.github/workflows/release.yml`：OIDC `id-token: write` + `rust-lang/crates-io-auth-action@v1`，先 `plugctx` 再 `plugctx-derive`，不用 `secrets.CARGO_REGISTRY_TOKEN`。
- `docs/publishing.md` 写成可执行 Trusted Publisher 步骤（仓库名、`release.yml`、约 30 分钟、并存、新 crate 仍须手工首次）。
- ATDD `acceptance_story_10_1` 已接入 `ci-test.sh`；`automate` 未再扩测（工作流形状 + publishing 要点 + dry-run 已覆盖）。
- Code review：无阻塞问题。维护者仍须在 crates.io 上为两 crate 配置 Trusted Publisher（仓库外操作，本切片不实际上架）。

### File List

- `.github/workflows/release.yml`
- `docs/publishing.md`
- `docs/testing.md`
- `scripts/ci-test.sh`
- `crates/plugctx/tests/acceptance_story_10_1.rs`
- `_agile-output/implementation-artifacts/1-1-trusted-publishing-发版工作流.md`
- `_agile-output/implementation-artifacts/sprint-status.yaml`
