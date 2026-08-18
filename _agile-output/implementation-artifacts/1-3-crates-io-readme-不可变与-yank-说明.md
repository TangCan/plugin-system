---
id: "1.3"
key: "1-3-crates-io-readme-不可变与-yank-说明"
status: done
story_num: "1.3"
epic: 1
---

# Story 1.3: crates.io README 不可变与 yank 说明

Status: done

Ultimate context engine analysis completed - comprehensive developer guide created.

## Story

As a 维护者,
I want 发布文档写清「改该版本在 crates.io 上的 README 必须升版本」,
so that 不会误以为 yank 或 docs.rs 重建能改掉已发布制品。

## Acceptance Criteria

1. **Given** 某版本已 `cargo publish`  
   **When** 维护者阅读 `docs/publishing.md`（及若 README 有发布节则同步一句）  
   **Then** 文档写明：crates.io 展示的 README 绑在该版本 `.crate` 上，要更新必须 bump 版本再 publish（FR6）  
   **And** 写明 `yank` ≠ 删除：yank 只阻止新解析，已有 lockfile / 已下载副本仍在；密钥泄露靠轮换 token，不以 yank 代替（NFR8）

2. **Given** crates.io 可从版本列表触发 docs.rs 重建  
   **When** 文档描述该能力  
   **Then** 写明重建只刷新 rustdoc，**不**替换该版本 `.crate` 内的 README（FR6）

3. **Given** NFR7：不强制 workspace 写成 `0.2.0`  
   **When** 文档举例为让 README 生效而发版  
   **Then** 使用 0.y.z 惯例（例如 0.1.1 → 0.1.2），两 crate 锁步；本故事不执行实际上架

## Tasks / Subtasks

- [x] 扩展 `docs/publishing.md`：README 绑版本、必须 bump、yank ≠ 删除、docs.rs 重建 ≠ README（AC: #1 #2）
- [x] 举例 0.1.1 → 0.1.2 锁步；不实际上架、不改 workspace version（AC: #3）
- [x] README 发布相关处补一句；验收 `acceptance_story_10_3`

## Dev Notes

- 已有 yank ≠ 删除（NFR13 节）与「轮换 token」。本故事把 **crates.io README 不可变** 和 **docs.rs 重建范围** 写清楚。
- 不要 bump `workspace.package.version`，不要 `cargo publish`。
- 测试编号 **10.3**。

## Dev Agent Record

### Agent Model Used

Composer

### Debug Log References

### Completion Notes List

- publishing.md 新增 FR6：README 绑 `.crate`、docs.rs 重建只刷 rustdoc、yank ≠ 删除、0.1.1 → 0.1.2 锁步举例。未 bump 版本、未实际上架。
- Code review / automate：无额外测；10_3 守门文档要点。

### File List

- `docs/publishing.md`
- `README.md`
- `docs/guide.md`
- `docs/testing.md`
- `scripts/ci-test.sh`
- `crates/plugctx/tests/acceptance_story_10_3.rs`
- `_agile-output/implementation-artifacts/1-3-crates-io-readme-不可变与-yank-说明.md`
- `_agile-output/implementation-artifacts/sprint-status.yaml`
