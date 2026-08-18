#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 5.3 — 测试金字塔与文档门禁（ATDD）。
//!
//! 巩固既有关键路径覆盖，并以护栏锁定文档 / CI 入口 / trybuild 骨架。

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    // tests/ → plugctx/ → crates/ → plugin-system/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("plugin-system root")
        .to_path_buf()
}

fn read_utf8(rel: &str) -> String {
    let path = workspace_root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn path_exists(rel: &str) -> bool {
    workspace_root().join(rel).exists()
}

/// AC#1: 关键路径 acceptance 文件仍在（生命周期 / DI / 重入 / effect / isolate）。
#[test]
fn critical_path_acceptance_files_exist() {
    let required = [
        "crates/plugctx/tests/acceptance_story_1_2.rs",
        "crates/plugctx/tests/acceptance_story_1_4.rs",
        "crates/plugctx/tests/acceptance_story_1_5.rs",
        "crates/plugctx/tests/acceptance_story_1_6.rs",
        "crates/plugctx/tests/acceptance_story_1_7.rs",
    ];
    for rel in required {
        assert!(path_exists(rel), "missing critical-path acceptance: {rel}");
    }
}

/// AC#1: 测试策略文档描述金字塔并映射关键路径。
#[test]
fn testing_doc_maps_pyramid_and_critical_paths() {
    let doc = read_utf8("docs/testing.md");
    for needle in [
        "测试金字塔",
        "单元",
        "集成",
        "属性",
        "trybuild",
        "acceptance_story_1_2",
        "acceptance_story_1_4",
        "acceptance_story_1_5",
        "acceptance_story_1_6",
        "acceptance_story_1_7",
        "生命周期",
        "MissingDependency",
        "重入",
        "effect",
        "isolate",
    ] {
        assert!(
            doc.contains(needle),
            "docs/testing.md must mention `{needle}`"
        );
    }
}

/// AC#1/#2: CI 脚本与 README 回归命令。
#[test]
fn ci_test_entry_and_readme_regression_commands() {
    assert!(
        path_exists("scripts/ci-test.sh"),
        "missing scripts/ci-test.sh"
    );
    let script = read_utf8("scripts/ci-test.sh");
    assert!(
        script.contains("cargo test"),
        "ci-test.sh must invoke cargo test"
    );
    assert!(
        script.contains("cargo doc"),
        "ci-test.sh must invoke cargo doc gate"
    );

    let readme = read_utf8("README.md");
    assert!(
        readme.contains("ci-test.sh") || readme.contains("回归门禁"),
        "README must document CI/regression entry"
    );
    assert!(
        readme.contains("cargo test -p plugctx"),
        "README must show core test command"
    );
}

/// AC#2: README 说明 start/dispose 与常见错误。
#[test]
fn readme_documents_lifecycle_and_errors() {
    let readme = read_utf8("README.md");
    for needle in [
        "start",
        "dispose",
        "MissingDependency",
        "CircularDependency",
        "AlreadyStarted",
        "AlreadyDisposed",
    ] {
        assert!(
            readme.contains(needle),
            "README must document lifecycle/error `{needle}`"
        );
    }
}

/// AC#2: README Feature 矩阵覆盖默认同步与扩展 feature。
#[test]
fn readme_documents_feature_matrix() {
    let readme = read_utf8("README.md");
    for needle in [
        "Feature",
        "async",
        "parallel",
        "thread-safe",
        "dynamic-native",
        "dynamic-wasm",
    ] {
        assert!(
            readme.contains(needle),
            "README feature matrix must mention `{needle}`"
        );
    }
}

/// AC#2: 源码 rustdoc 含生命周期与常见错误要点（门禁护栏）。
#[test]
fn context_error_rustdoc_mentions_lifecycle() {
    let context =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context.rs"))
            .expect("context.rs");
    assert!(
        context.contains("ReadyEvent") && context.contains("DisposeEvent"),
        "Context rustdoc should reference Ready/Dispose events"
    );
    assert!(
        context.contains("AlreadyStarted") || context.contains("AlreadyDisposed"),
        "Context rustdoc should mention start/dispose errors"
    );

    let error =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/error.rs"))
            .expect("error.rs");
    for needle in [
        "MissingDependency",
        "CircularDependency",
        "AlreadyStarted",
        "AlreadyDisposed",
    ] {
        assert!(error.contains(needle), "error.rs must document `{needle}`");
    }

    let lib = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
        .expect("lib.rs");
    assert!(
        lib.contains("start") && lib.contains("dispose"),
        "crate-level rustdoc should mention start/dispose"
    );
}

/// AC#3: trybuild UI 套件存在（Story 5.3 骨架；≥3 例由 Story 5.6 / `acceptance_story_5_6` 锁定）。
#[test]
fn trybuild_ui_skeleton_exists() {
    assert!(
        path_exists("crates/plugctx/tests/ui.rs"),
        "missing trybuild harness tests/ui.rs"
    );
    let ui_dir = workspace_root().join("crates/plugctx/tests/ui");
    assert!(ui_dir.is_dir(), "missing tests/ui directory");
    let mut compile_fail = false;
    for entry in std::fs::read_dir(&ui_dir).expect("read ui dir") {
        let entry = entry.expect("dirent");
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".rs") {
            compile_fail = true;
            break;
        }
    }
    assert!(
        compile_fail,
        "tests/ui must contain at least one compile_fail .rs"
    );
    let stderr = ui_dir.join("plugin_must_impl_trait.stderr");
    assert!(
        stderr.is_file(),
        "trybuild stderr snapshot missing: {}",
        stderr.display()
    );
}

/// Automate 护栏：README 须指向测试金字塔文档。
#[test]
fn readme_links_testing_doc() {
    let readme = read_utf8("README.md");
    assert!(
        readme.contains("docs/testing.md"),
        "README must link docs/testing.md for pyramid mapping"
    );
}
