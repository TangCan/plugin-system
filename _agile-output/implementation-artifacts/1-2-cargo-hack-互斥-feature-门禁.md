---
id: "1.2"
key: "1-2-cargo-hack-互斥-feature-门禁"
status: done
story_num: "1.2"
epic: 1
---

# Story 1.2: cargo-hack 互斥 feature 门禁

Status: done

Ultimate context engine analysis completed - comprehensive developer guide created.

## Story

As a 维护者,
I want CI 用 cargo-hack 表达 `thread-safe` 与默认同步验收的互斥,
so that 不会有人把 `cargo test --all-features` 当成唯一门槛而打红（或漏跑）默认同步测试。

## Acceptance Criteria

1. **Given** `thread-safe` 与部分默认同步验收互斥（既有 feature-matrix / `ci-test.sh`）  
   **When** 回归门禁增加 `cargo hack`（`--feature-powerset` 或 `--each-feature`）  
   **Then** 至少声明 `--mutually-exclusive-features` 覆盖 `thread-safe` 与会和它冲突的默认同步测试组合  
   **And** 启用多 feature 时加上 `--exclude-all-features`，避免 cargo-hack 隐式再跑 `--all-features`（FR3）

2. **Given** 现有 `./scripts/ci-test.sh` 含 fmt、clippy `-D warnings`、扩展矩阵  
   **When** 接入 cargo-hack  
   **Then** 不得删除或削弱这些门；不得把单独的 `cargo test --all-features` 作为唯一测试 job  
   **And** `docs/testing.md`（及如有需要的 README 回归说明）写明如何本地跑该 hack 命令

3. **Given** Architecture AD-6 仍为 ubuntu 托管  
   **When** 本故事完成  
   **Then** 不要求增加 Windows GHA runner

## Tasks / Subtasks

- [x] 新增 `scripts/ci-cargo-hack.sh`：`cargo hack test -p plugctx --each-feature`（或 powerset）+ `--mutually-exclusive-features thread-safe` + `--exclude-all-features`；排除 `dynamic-native` / `dynamic-wasm` / `dynamic-wasm-component`（仍由 `ci-extension-matrix.sh` 覆盖）（AC: #1）
- [x] `ci-test.sh` 调用该脚本；`.github/workflows/ci.yml` 安装 `cargo-hack`（`taiki-e/install-action`）（AC: #2）
- [x] 保留 fmt / clippy `-D warnings` / `ci-extension-matrix.sh`；CI 无 Windows runner、无唯一 job `cargo test --all-features`（AC: #2 #3）
- [x] `docs/testing.md` 写明本地命令；验收 `acceptance_story_10_2` 接入 `ci-test.sh`

## Dev Notes

- **不要**改 `plugctx` 运行时、`default = []`、或把 native/wasm 拉进默认图。
- 互斥语义：若干验收文件顶部 `#![cfg(not(feature = "thread-safe"))]`。`cargo test --all-features` 会启用 `thread-safe`，这些用例被编掉，不能当唯一门。`async`+`thread-safe` 等组合是合法的，不要把 `thread-safe` 与 `async` 标成互斥。
- cargo-hack：`--each-feature` 默认还会再跑一次 `--all-features`；互斥场景必须加 `--exclude-all-features`。[Source: cargo-hack README]
- `--mutually-exclusive-features thread-safe` 声明该互斥；空 feature 集（默认同步）由 `--each-feature` 单独跑覆盖。
- 重运行时 feature 继续走 `ci-extension-matrix.sh`（需先 `cargo build -p hello_plugin` 等）；hack 脚本 `--exclude-features` 它们以免重复且拖慢。
- 本地若无 `cargo-hack`：脚本应安装或给出明确失败信息；CI 用 `taiki-e/install-action` 装 `cargo-hack`。
- 测试编号 **10.2**，勿覆盖旧 `acceptance_story_1_2.rs`。
- 不增加 Windows GHA runner（AD-6）。

### Project Structure Notes

- 新：`scripts/ci-cargo-hack.sh`、`crates/plugctx/tests/acceptance_story_10_2.rs`
- 改：`scripts/ci-test.sh`、`.github/workflows/ci.yml`、`docs/testing.md`（可选 README 一句）

### References

- [Source: `_agile-output/planning-artifacts/epics.md` Story 1.2 / FR3]
- [Source: `docs/testing.md`、`docs/feature-matrix.md`]
- [Source: `scripts/ci-test.sh`、`scripts/ci-extension-matrix.sh`]
- [Source: `_agile-output/planning-artifacts/research/technical-rust-in-process-plugin-framework-post-0-2026-08-18/research.md` P0 cargo-hack]

## Dev Agent Record

### Agent Model Used

Composer

### Debug Log References

### Completion Notes List

- `--mutually-exclusive-features` 只适用于 `--feature-powerset` 且至少两个名字，故声明 `thread-safe,default`（空 default = 默认同步验收），并用 `--depth 1` 控制时间。
- 第一次用 `cargo hack test` 会在 `--features async` 下让 trybuild `start_async_requires_feature` 从 compile_fail 变成通过；改为 `cargo hack check` 守编译面，验收测试仍走既有矩阵。
- `--exclude-all-features` 阻止 cargo-hack 隐式 `--all-features`。native/wasm 仍走扩展矩阵。
- CI 用 `taiki-e/install-action` 装 cargo-hack；本地脚本缺失时 `cargo install cargo-hack --locked`。
- Code review：无阻塞。automate 未再扩测（10_2 已守门脚本/文档/无 Windows）。

### File List

- `scripts/ci-cargo-hack.sh`
- `scripts/ci-test.sh`
- `.github/workflows/ci.yml`
- `docs/testing.md`
- `README.md`
- `crates/plugctx/tests/acceptance_story_10_2.rs`
- `_agile-output/implementation-artifacts/1-2-cargo-hack-互斥-feature-门禁.md`
- `_agile-output/implementation-artifacts/sprint-status.yaml`
