//! Acceptance tests for story 9.4 — 0.y 版本策略与 CHANGELOG 对齐（ATDD / FR54）。

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

/// AC#1: CHANGELOG / feature-matrix / publishing 写清 0.y 与能力清单关系。
#[test]
fn semver_0y_and_capability_slices_documented() {
    let changelog = read_required("CHANGELOG.md");
    assert!(
        changelog.contains("0.1.0") && changelog.contains("0.2.0"),
        "CHANGELOG 须含 0.1.0 / 0.2.0 能力切片（FR54）"
    );
    for needle in ["能力清单", "0.y", "feature", "breaking"] {
        assert!(
            changelog.contains(needle)
                || changelog
                    .to_ascii_lowercase()
                    .contains(&needle.to_ascii_lowercase()),
            "CHANGELOG 须说明 0.y / 能力清单策略（缺「{needle}」；FR54）"
        );
    }
    assert!(
        changelog.contains("不等于")
            || changelog.contains("≠")
            || changelog.contains("不强制")
            || changelog.contains("不必"),
        "CHANGELOG 须写明能力清单 ≠ 强制 bump version 字符串（FR54）"
    );

    let matrix = read_required("docs/feature-matrix.md");
    assert!(
        matrix.contains("0.2.0")
            && (matrix.contains("version")
                || matrix.contains("能力清单")
                || matrix.contains("bump")),
        "feature-matrix 须对齐 0.2 能力清单与 version 关系（FR54）"
    );

    let publishing = read_required("docs/publishing.md");
    for needle in ["0.y", "能力清单", "breaking", "FR54"] {
        assert!(
            publishing.contains(needle),
            "docs/publishing.md 须记载版本策略「{needle}」（FR54）"
        );
    }
}

/// AC#2: plugctx / plugctx-derive 锁步；诚实记载 pluggable 占用与 plugctx 采用。
#[test]
fn crate_version_coupling_and_rename_honesty() {
    let publishing = read_required("docs/publishing.md");
    assert!(
        publishing.contains("plugctx-derive")
            && (publishing.contains("锁步")
                || publishing.contains("同版本")
                || publishing.contains("workspace.package.version")),
        "须说明 plugctx 与 plugctx-derive 版本耦合（FR54）"
    );

    // 诚实：旧名 pluggable 已被占用；现名 plugctx 已采用，发布前仍须复验
    assert!(
        publishing.contains("pluggable")
            && (publishing.contains("占用") || publishing.contains("已占用")),
        "须诚实记载 crates.io 上 pluggable 已被占用（FR54）"
    );
    assert!(
        publishing.contains("plugctx")
            && (publishing.contains("已采用")
                || publishing.contains("复验")
                || publishing.contains("404")),
        "须记载已采用 plugctx 及发布前复验策略（FR54）"
    );

    let ws = read_required("Cargo.toml");
    assert!(
        ws.contains("version = \"0.1.0\"") || ws.contains("workspace.package"),
        "workspace 须有统一 package.version（锁步基线；FR54）"
    );
    let pkg = read_required("crates/plugctx/Cargo.toml");
    assert!(
        pkg.contains("name = \"plugctx\""),
        "crates/plugctx 的 package.name 须为 plugctx（FR54）"
    );
}
