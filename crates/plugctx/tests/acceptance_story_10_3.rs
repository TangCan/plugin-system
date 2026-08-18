//! Acceptance tests for post-0.1.1 story 1.3 — crates.io README 不可变与 yank（ATDD / FR6 / NFR8）。

use std::path::PathBuf;

fn plugin_system_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("plugin-system root")
        .to_path_buf()
}

fn read_required(rel: &str) -> String {
    let path = plugin_system_root().join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("missing required artifact {rel} at {}: {e}", path.display()))
}

/// AC#1–#2: publishing.md 写清 README 绑版本、yank 非删除、docs.rs 重建不换 README。
#[test]
fn publishing_docs_readme_immutability_and_yank() {
    let publishing = read_required("docs/publishing.md");
    assert!(
        publishing.contains("README")
            && (publishing.contains(".crate") || publishing.contains("crate")),
        "须写明 crates.io README 绑在该版本制品上（FR6）"
    );
    assert!(
        publishing.contains("bump")
            || publishing.contains("升版本")
            || publishing.contains("新版本"),
        "须写明改 README 要 bump 再 publish（FR6）"
    );
    assert!(
        publishing.contains("yank") && (publishing.contains("删除") || publishing.contains("≠")),
        "须写明 yank ≠ 删除（NFR8）"
    );
    assert!(
        publishing.contains("lockfile")
            || publishing.contains("已下载")
            || publishing.contains("副本"),
        "须写明已有 lockfile / 已下载副本仍在（NFR8）"
    );
    assert!(
        publishing.contains("轮换") && publishing.contains("token"),
        "密钥泄露靠轮换 token，不以 yank 代替（NFR8）"
    );
    assert!(
        publishing.contains("docs.rs")
            && (publishing.contains("重建") || publishing.contains("rebuild"))
            && (publishing.contains("rustdoc") || publishing.contains("文档")),
        "须写明 docs.rs 重建只刷新 rustdoc（FR6）"
    );
    assert!(
        publishing.contains("README")
            && (publishing.contains("不")
                || publishing.contains("不会")
                || publishing.contains("≠")),
        "须写明 docs.rs 重建不替换该版本 README（FR6）"
    );
}

/// AC#3: 举例 0.1.1 → 0.1.2 锁步；后续 patch 仍走同一 workspace.package.version。
#[test]
fn readme_fix_example_is_0y_lockstep_without_live_publish() {
    let publishing = read_required("docs/publishing.md");
    assert!(
        publishing.contains("0.1.1") && publishing.contains("0.1.2"),
        "举例须用 0.1.1 → 0.1.2，不得强制写成 0.2.0（NFR7 / FR6）"
    );
    assert!(publishing.contains("锁步"), "须写明两 crate 锁步（FR6）");

    let workspace = read_required("Cargo.toml");
    assert!(
        workspace.contains("[workspace.package]") && workspace.contains("version = \""),
        "须保持 workspace.package.version 锁步（FR6）"
    );
}
