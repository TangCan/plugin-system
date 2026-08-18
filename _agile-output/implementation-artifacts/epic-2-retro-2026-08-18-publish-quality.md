---
epic: 2
date: 2026-08-18
verdict: accepted
criteria: declared
headless: true
slice: post-0.1.1-publish-quality
---

# Epic 2 Retrospective — 应用作者能按真实限度用 native / WASM（post-0.1.1）

## Epic summary

- **史诗：** 当前 `epics.md` 切片 `post-0.1.1-publish-quality` 的 Epic 2（FR4、FR5）。**不是**落地补齐切片的 Epic 2（可安装且可守门，见已有 `epic-2-retro-2026-08-18.md`）。
- **Diff 范围：** `76df4b6..6aa244d`（第一则实现提交 `c6a181b` 的父提交 .. 最后一则 `6aa244d`）
- **提交（`git_evidence.py`）：** `c6a181b` native 卸载限度；`6aa244d` WIT pin。`commit_count: 2`，`merge_count: 0`
- **故事：** `2-1-native-卸载限度写入用户指南`、`2-2-wit-版本钉死与双-wasm-路径` 均为 `done`；`pending_stories: []`（`detect-epic --epic 2`：`story_count: 2`，`retro_status: optional`）
- **sprint-status：** `epic-2: done`，本 retro 前 `epic-2-retrospective: optional`
- **证据库存**
  - 有：`epics.md` Epic 2 声明 AC、两则故事文件、`git_evidence.py` JSON、`docs/guide.md`、`docs/requirements/4. 扩展模块设计.md` §4.3、`docs/component-model-versions.md`、`guests/wit-sample/`、验收 `acceptance_story_11_1` / `11_2`（本 retro 重跑均 ok）
  - 缺：提交 subject 不含故事 id，`commits[].stories` 空——按一提交一故事手工映射
  - 缺：未调用独立 `bmad-review` 子技能；透镜 inline
  - 缺：会话日志未摘录
  - 上一则 **本切片** retro：`epic-1-retro-2026-08-18-publish-quality.md`（Trusted Publisher action 拟写入 sprint-status，本文件定稿时与 Epic 1 同批）
  - 落地补齐 previous：`epic-3-retro-2026-08-18.md`；authors action 已 `done`

## Findings

### Architecture delta

- **观察：** 本史诗只改文档与验收测试，未改 `plugctx` 加载器、未加 `reload()`、未把 `extism`/`wasmtime` 拉进 default。`dynamic-wasm` 与 `dynamic-wasm-component` 仍分 feature。客人 WIT 仍是 `plugctx:sample@0.1.0`，无 `wasi@0.3.0` import。
  - **来源：** `git_evidence.py` `files`（无 `crates/plugctx/src/`）；`guests/wit-sample/wit/world.wit`；`crates/plugctx/Cargo.toml` `default = []`；本 retro `rg 'fn reload' crates/` 无匹配
  - **处置：** accept as-is（AD-1…AD-3、NFR1、NFR3、NFR4）
  - **课：** 无

### Duplication map

- **观察：** 卸载三点在 `docs/guide.md` 与 `docs/requirements/4. 扩展模块设计.md` §4.3 各写一遍，有意对齐而非两套说法。WIT pin 在 `component-model-versions.md` 与 `guests/wit-sample/README.md` 同步。
  - **来源：** `c6a181b`、`6aa244d`；`acceptance_story_11_1.rs` / `11_2.rs` 对两处同时断言
  - **处置：** accept as-is
  - **课：** 用户指南与 requirements 双写时用同一验收测两份，避免以后只改一处

### God-class / size growth

- **观察：** 最大 churn 是 `acceptance_story_11_2.rs`（net +139）与故事文件。`guide.md` net +12，`4. 扩展模块设计.md` net +10。无内核类增长。
  - **来源：** `git_evidence.py` `files`
  - **处置：** accept as-is
  - **课：** 无

### Pattern divergence

- **观察：** 测试文件编号 `acceptance_story_11_*` 对应新切片 Epic 2，避免覆盖旧内核 `acceptance_story_2_*`。与 Epic 1 的 `10_*` 同一约定。
  - **来源：** `crates/plugctx/tests/acceptance_story_11_1.rs`；`AGENTS.md` 既有 `1_1` 内核测试
  - **处置：** accept as-is
  - **课：** 新切片故事编号与旧 acceptance 文件号会撞车，继续用 10+/11+ 偏移

### Spec-to-implementation

- **观察：** FR4 三点（FreeLibrary/dlclose ≠ 可覆盖；macOS TLS；sound 卸载无残留引用）写进指南与 §4.3；热插拔仍 load → dispose → load；无 `reload()`；WASM 仍 close/free / Drop Store。FR5 实际 pin（wasmtime 47、wit-bindgen 0.60、wasm32-wasip2、`plugctx:sample@0.1.0`）并写明禁止提前 `wasi@0.3.0`；双路径分制品。
  - **来源：** `acceptance_story_11_1.rs`、`11_2.rs` 本 retro 全绿；`docs/component-model-versions.md`「实际 WIT pin」节；`guests/wit-sample/README.md`
  - **处置：** accept as-is
- **观察：** pin 会过期（调研写 versions 约一个月）。文档已有复查窗 2026-09-17。本史诗 AC 是钉**当前**工具链，不是跟规范发布标签。
  - **来源：** `docs/component-model-versions.md` 复查窗；research `technical-rust-in-process-plugin-framework-post-0-2026-08-18`
  - **处置：** defer（不另开 action；复查窗已写在文档）
  - **课：** 无

### Diff-scope lenses（inline；未跑 `bmad-review` 子技能）

- **Adversarial：** 文档可能被读成「dispose 成功就能覆盖 Windows `.dll`」。指南现用「成功 ≠ 文件一定可覆盖」对冲。未把 `hot-lib-reloader` 收成生产 API（若提及则标明非生产）。
  - **来源：** `docs/guide.md` Native 卸载限度；`acceptance_story_11_1.rs` `asserts_unload_limits`
  - **处置：** accept as-is
- **Edge-case：** 无 macOS/Windows 托管 job 验证 TLS/文件锁（AD-6）。本史诗只要求中文文档，不要求新 runner。
  - **来源：** `.github/workflows/ci.yml` `ubuntu-latest`；`epics.md` Additional Requirements
  - **处置：** accept as-is
- **Verification-gap：** 护栏是文档字符串测试，不在 Darwin 上 `dlclose`、不重建 wasip2 guest。本 retro 未跑 `scripts/build-wit-sample-guest.sh`，以检入 `.wasm` + 既有 `8_4` 为制品真相。
  - **来源：** `11_1`/`11_2` 只 `read_required`；`git_evidence` 无 `.wasm` 变更
  - **处置：** accept as-is（FR5 要求钉死与检入一致，本范围未改 guest 源）

## Behavior verification

已跑：

```
cargo test -p plugctx --test acceptance_story_11_1   # 2 passed
cargo test -p plugctx --test acceptance_story_11_2   # 3 passed
```

未行使：真实 `dlclose`/`FreeLibrary`、macOS TLS 永不卸载、把 guest 改钉 `wasi@0.3.0` 的失败实例化。Epic 2 不改变运行时，故未重跑 `acceptance_story_4_2` / `8_4`。

## Previous-retro follow-through

- 落地补齐 `epic-2-retro-item-1-下一版本将-workspace-package-authors-从-releas`：已 `done`（`Cargo.toml` authors；`d0148c4`）。Headless **不**传 `--set-action-status`。
- 本切片 Epic 1 拟追加的 Trusted Publisher 配置项：属 Epic 1，不在本史诗关闭。

## Action items

无（FR4/FR5 在仓库内已满足；WIT 复查已有文档日期，不另建跟踪项）。

## Acceptance verdict

**accepted**（declared）

- 声明标准：读完后知道卸载限度、WIT 跟当前工具链、Extism/CM 分家、热插拔仍 load → dispose → load。两则故事 AC 与 FR4/FR5 可在文档 + `11_1`/`11_2` 核对。
- `pending_stories: []`。无阻塞发现。

## Open questions

- 2026-09 工具链刷新后是否仍禁止 `wasi@0.3.0`（文档复查窗，非本 retro 缺口）。

## Assumptions

- Headless：用户要求完成 sprint-status 中所有 Epic retrospective；本文件是 **当前切片 Epic 2**（`--epic 2`）。
- `detect-epic --epic 2`：`pending_stories: []`，`story_count: 2`。
- 文档路径 `epic-2-retro-2026-08-18-publish-quality.md`，避免覆盖落地补齐 `epic-2-retro-2026-08-18.md`。
- 机器裁决 **accepted**（无人类覆盖）。
- 无新 action item。未 `--set-action-status`。
- Phase 3 跳过。`bmad-review` 未单独拉起；透镜范围 = `76df4b6..6aa244d`。
