//! Acceptance tests for story 6.3 — 0.1 核心版与 0.2 扩展版发布切片（ATDD / FR42）。
//!
//! Red phase: fails until CHANGELOG / feature-matrix 中文文档存在且含 0.1.0、0.2.0
//! 能力清单与设计对齐关键字。

use std::path::PathBuf;

fn plugin_system_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .and_then(|p| p.parent()) // plugin-system/
        .expect("plugin-system root")
        .to_path_buf()
}

fn read_required(rel: &str) -> String {
    let path = plugin_system_root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "missing required release artifact {rel} at {}: {e}",
            path.display()
        )
    })
}

/// AC#1: CHANGELOG（或等价）含 0.1.0 节，覆盖 Epic 1–2 能力与测试门禁最小集。
#[test]
fn changelog_0_1_0_lists_core_epic_1_2_and_test_gate() {
    let text = read_required("CHANGELOG.md");
    assert!(
        text.contains("0.1.0") || text.contains("[0.1.0]"),
        "CHANGELOG 须标注 0.1.0（FR42）"
    );
    // Epic 1–2 能力关键字（中文或英文 API 名均可）
    for needle in [
        "Context",
        "Plugin",
        "事件",
        "Effect",
        "PluginHandle",
        "Interceptor",
        "测试",
    ] {
        assert!(
            text.contains(needle),
            "0.1.0 节须提及核心能力/门禁关键字「{needle}」"
        );
    }
    assert!(
        text.contains("cargo test") || text.contains("ci-test"),
        "0.1.0 节须包含测试门禁最小集（cargo test / ci-test）"
    );
}

/// AC#2: 0.2.0 清单列出扩展 feature 与对应测试。
#[test]
fn changelog_0_2_0_lists_extensions_and_tests() {
    let text = read_required("CHANGELOG.md");
    assert!(
        text.contains("0.2.0") || text.contains("[0.2.0]"),
        "CHANGELOG 须标注 0.2.0（FR42）"
    );
    for feat in [
        "async",
        "parallel",
        "thread-safe",
        "dynamic-native",
        "dynamic-wasm",
        "stages",
        "derive",
    ] {
        assert!(
            text.contains(feat),
            "0.2.0 清单须列出扩展项「{feat}」（FR42）"
        );
    }
    assert!(
        text.contains("acceptance_story")
            || text.contains("ci-extension-matrix")
            || text.contains("plugctx-derive"),
        "0.2.0 须映射对应测试 / FR41 矩阵入口"
    );
}

/// AC#3: Feature 矩阵文档与设计 §2.4 / §7.3（正文 §7.9）对齐，含刻意偏离。
#[test]
fn feature_matrix_aligns_with_design_and_documents_deviations() {
    let text = read_required("docs/feature-matrix.md");
    assert!(
        text.contains("2.4") || text.contains("§2.4"),
        "feature-matrix 须引用设计 §2.4"
    );
    assert!(
        text.contains("7.3") || text.contains("7.9") || text.contains("§7"),
        "feature-matrix 须引用设计 Feature 划分（大纲 §7.3 / 正文 §7.9）"
    );
    for feat in [
        "async",
        "parallel",
        "thread-safe",
        "dynamic-native",
        "dynamic-wasm",
        "stages",
    ] {
        assert!(
            text.contains(feat),
            "feature-matrix 须包含 feature「{feat}」"
        );
    }
    assert!(
        text.contains("偏离") || text.contains("刻意"),
        "feature-matrix 须含刻意偏离说明（FR42）"
    );
}

/// 护栏：README 链到发布切片文档，便于发现。
#[test]
fn readme_links_release_slice_docs() {
    let text = read_required("README.md");
    assert!(
        text.contains("CHANGELOG") || text.contains("changelog"),
        "README 须链到 CHANGELOG"
    );
    assert!(
        text.contains("feature-matrix") || text.contains("Feature 矩阵"),
        "README 须指向 feature-matrix 或 Feature 矩阵文档"
    );
}

/// 护栏：testing.md 登记 Story 6.3 门禁。
#[test]
fn testing_md_maps_story_6_3() {
    let text = read_required("docs/testing.md");
    assert!(
        text.contains("6.3") || text.contains("acceptance_story_6_3") || text.contains("FR42"),
        "testing.md 须映射 Story 6.3 / FR42 / acceptance_story_6_3"
    );
}

/// Automate：Cargo.toml 声明的扩展 feature 须出现在 feature-matrix（防文档漂移）。
#[test]
fn cargo_features_listed_in_feature_matrix() {
    let cargo = read_required("crates/plugctx/Cargo.toml");
    let matrix = read_required("docs/feature-matrix.md");
    for feat in [
        "async",
        "parallel",
        "thread-safe",
        "dynamic-native",
        "dynamic-wasm",
        "tracing",
        "stages",
    ] {
        assert!(cargo.contains(feat), "Cargo.toml 须声明 feature「{feat}」");
        assert!(
            matrix.contains(feat),
            "feature-matrix.md 须覆盖 Cargo feature「{feat}」"
        );
    }
}
