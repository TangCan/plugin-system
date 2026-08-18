---
epic: 1
date: 2026-08-18
verdict: accepted-with-open-items
criteria: declared
headless: true
slice: post-0.1.1-publish-quality
---

# Epic 1 Retrospective — 维护者可以安全发下一版（post-0.1.1）

## Epic summary

- **史诗：** 当前 `epics.md` 切片 `post-0.1.1-publish-quality` 的 Epic 1（FR1、FR2、FR3、FR6、FR7）。**不是**落地补齐切片的 Epic 1（Native 热插拔，见已有 `epic-1-retro-2026-08-18.md`）。
- **Diff 范围：** `48e2823..76df4b6`（第一则实现提交 `c9b0de6` 的父提交 .. 最后一则 `76df4b6`）
- **提交（`git_evidence.py`）：** `c9b0de6` Trusted Publishing；`6ed3590` cargo-hack；`386de4a` README/yank；`76df4b6` docs.rs。`commit_count: 4`，`merge_count: 0`
- **故事：** `1-1-trusted-publishing-发版工作流`、`1-2-cargo-hack-互斥-feature-门禁`、`1-3-crates-io-readme-不可变与-yank-说明`、`1-4-docs-rs-metadata-守门` 均为 `done`；`pending_stories: []`（`detect-epic --epic 1`：`story_count: 4`，`retro_status: optional`）
- **sprint-status：** `epic-1: done`，本 retro 前 `epic-1-retrospective: optional`
- **证据库存**
  - 有：`epics.md` Epic 1 声明 AC、四则故事文件、`git_evidence.py` JSON、工作流/脚本/文档、验收 `acceptance_story_10_1`…`10_4`（本 retro 重跑均 ok）
  - 缺：提交 subject 不含故事 id，故 `commits[].stories` 全空——归属按一提交一故事 + File List 手工映射
  - 缺：未调用独立 `bmad-review` 子技能；透镜对本范围 inline
  - 缺：会话日志未当过程证据源
  - 上一则 retro（落地补齐）：`epic-3-retro-2026-08-18.md`；sprint-status 一条落地 Epic 2 action（authors）已是 `done`
  - 无 `project-context.md`（customize persistent_facts glob 未命中）

## Findings

### Architecture delta

- **观察：** 新增 tag 触发 `.github/workflows/release.yml`（`id-token: write`，`rust-lang/crates-io-auth-action@v1`，先 `plugctx` 再 `plugctx-derive`）。`ci.yml` 增加 `taiki-e/install-action` 装 cargo-hack。`plugctx` 默认依赖图仍无 `extism` / `libloading` / `wasmtime`。无新运行时 crate。
  - **来源：** `.github/workflows/release.yml` L1–L25；`.github/workflows/ci.yml` L15–L17；本 retro `cargo tree -p plugctx -e normal` 无重运行时；`c9b0de6` / `6ed3590`
  - **处置：** accept as-is
  - **课：** 无

### Duplication map

- **观察：** 发布文档在 `docs/publishing.md` 一处加厚（Trusted Publishing + README 不可变），`README.md` / `docs/guide.md` 只补一句指向，不是第二套流程。cargo-hack 脚本与 `ci-extension-matrix.sh` 并存：hack 排除 native/wasm，矩阵仍覆盖重 feature。
  - **来源：** `docs/publishing.md` FR6 节；`scripts/ci-cargo-hack.sh` L21–L26；`scripts/ci-extension-matrix.sh`
  - **处置：** accept as-is
  - **课：** 无

### God-class / size growth

- **观察：** 范围内最大 churn 是故事文件与验收测试（`acceptance_story_10_2.rs` net +102），不是内核。`context.rs` 仍 1297 行、`dynamic_native.rs` 仍 286 行，本史诗 `files` 不含这两路径。
  - **来源：** `git_evidence.py` `files`；本 retro `wc -l`
  - **处置：** accept as-is
  - **课：** 无

### Pattern divergence

- **观察：** FR3 允许 `--feature-powerset` 或 `--each-feature`。实现用 `--feature-powerset --depth 1`（cargo-hack 规定 `--mutually-exclusive-features` 只能配 powerset，且至少两个名字，故声明 `thread-safe,default`）。第一次用 `cargo hack test` 会在 `--features async` 下让 trybuild `start_async_requires_feature` 从 compile_fail 变成通过，改为 `cargo hack check`。
  - **来源：** `scripts/ci-cargo-hack.sh` L16–L26；故事 `1-2-cargo-hack-互斥-feature-门禁.md` Completion Notes；`crates/plugctx/tests/ui/start_async_requires_feature.rs`
  - **处置：** accept as-is（编译面互斥 + 既有矩阵仍跑验收；符合「不得削弱 fmt/clippy/矩阵」）
  - **课：** 对含 trybuild compile_fail 的 crate，cargo-hack 默认不要 `test` 全套

### Spec-to-implementation

- **观察：** FR1/FR2 工作流与 publishing 可执行步骤存在；本切片未实际上架、未 bump `0.1.1`（符合 NFR7 / 故事 AC#3）。FR3 CI 无 `cargo test --all-features`、无 Windows runner。FR6 README 绑 `.crate`、yank ≠ 删除、docs.rs 重建 ≠ README。FR7 derive 有 `[package.metadata.docs.rs]` 且无 `all-features = true`。
  - **来源：** `acceptance_story_10_1.rs`…`10_4.rs` 本 retro 全绿；`Cargo.toml` `version = "0.1.1"`；`crates/plugctx-derive/Cargo.toml` L24–L26
  - **处置：** accept as-is
- **观察：** crates.io 上 Trusted Publisher 是否已由 owner 点选，本仓库无法证实。工作流在未配置时会在真实 tag 发版失败。故事 AC 明确不要求本切片 live publish。
  - **来源：** `docs/publishing.md` Trusted Publishing 步骤；无 crates.io Settings 证据
  - **处置：** defer → action item（仓库外配置）
  - **课：** 发版故事把「workflow 检入」和「registry 侧 Trusted Publisher」分成可独立勾选的验收，避免只绿文件、漏平台配置

### Diff-scope lenses（inline；未跑 `bmad-review` 子技能）

- **Adversarial：** `release.yml` 无 `secrets.CARGO_REGISTRY_TOKEN`；token 仅 `steps.auth.outputs.token`。OIDC 短时 token 过期后不可复用（NFR8）。未配置 Trusted Publisher 时 tag 会失败——失败安全，不会用长期 secret 偷偷发。
  - **来源：** `.github/workflows/release.yml`；`rg secrets.CARGO_REGISTRY_TOKEN .github/workflows` 无匹配
  - **处置：** accept as-is
- **Edge-case：** cargo-hack `--mutually-exclusive-features thread-safe` 单名会报错，必须成对；`default=[]` 与 `thread-safe` 成对后 `--features default` 仍是合法空 default 跑。`--depth 1` 不测 `async+thread-safe` 组合（合法组合），该组合仍可由显式矩阵/人工补。
  - **来源：** cargo-hack 0.6.45 help；`ci-cargo-hack.sh` L18–L20
  - **处置：** accept as-is（depth 1 是时间/AC 允许的 each-feature 等价）
- **Verification-gap：** 本 retro 未跑完整 `just test` / 未打开某一 GitHub Actions run；只重跑 `10_1`–`10_4` 与 `cargo tree`。未执行真实 `cargo publish`（故事禁止）。
  - **来源：** 本文件行为检查；范围收窄如上
  - **处置：** accept as-is

## Behavior verification

已在本机（Linux）实际跑：

```
cargo test -p plugctx --test acceptance_story_10_1   # 3 passed（含 dry-run）
cargo test -p plugctx --test acceptance_story_10_2   # 3 passed
cargo test -p plugctx --test acceptance_story_10_3   # 2 passed
cargo test -p plugctx --test acceptance_story_10_4   # 2 passed
cargo tree -p plugctx -e normal                      # 无 extism/libloading/wasmtime
```

未行使：推 `v*` tag、crates.io OIDC 换票、真实 `cargo publish`。Epic 1 不改变 `plugctx` 运行时热插拔行为，故未重跑 native `acceptance_story_4_2`。

## Previous-retro follow-through

sprint-status 在本切片规划时继承落地补齐一条：

- **id：** `epic-2-retro-item-1-下一版本将-workspace-package-authors-从-releas`
- **是否落地：** 是。`Cargo.toml` `authors = ["Tang Can <tang_can@qq.com>"]`；提交 `d0148c4` Release 0.1.1 with real crate authors；条目 `status: done`（`d21fe72`）
- **本 headless 运行拟提议的 `--set-action-status`：** 无（已是 `done`，按规则不写该旗标）

落地补齐 `epic-1-retro-2026-08-18.md` 无未完成 action。

## Action items

1. **（提议，未自动执行）** 在 crates.io 为 `plugctx` 与 `plugctx-derive` 配置 Trusted Publisher（仓库 `TangCan/plugin-system`，workflow 文件名精确为 `release.yml`），再用 `v*` tag 发下一版。  
   **Owner：** Richard

## Acceptance verdict

**accepted-with-open-items**（declared）

- 声明标准：Epic 1 读完后应用 OIDC 发版、cargo-hack 测互斥 feature、docs.rs 轻量子集、publishing 写清 yank/README 不可变。四则故事 AC 与 FR1/2/3/6/7 在仓库内可核对。
- `pending_stories: []`，无阻塞实现缺口。
- 命名的未完成项仅仓库外 Trusted Publisher 配置（上条 action）。

## Open questions

- crates.io 两 crate 的 Trusted Publisher 是否已经点过（本 retro 无 Settings 证据）。
- 是否要在某一后续故事把 `cargo hack check` 扩到 `async+thread-safe` 等 depth>1 组合。

## Assumptions

- Headless：用户要求完成 sprint-status 中**所有** Epic retrospective；本文件对应 **当前切片 Epic 1**（`--epic 1`），不是落地补齐 Epic 1。
- `detect-epic --epic 1`：`pending_stories: []`，`story_count: 4`。
- 文档路径写成 `epic-1-retro-2026-08-18-publish-quality.md`，避免覆盖落地补齐同日 `epic-1-retro-2026-08-18.md`。
- 机器裁决 **accepted-with-open-items**（无人类覆盖）。
- 提议的 action 即上节第 1 条；未 `--set-action-status`。
- Phase 3 团队讨论按默认跳过。
- `bmad-review` 子技能未单独拉起；透镜 inline，范围 = `48e2823..76df4b6`。
