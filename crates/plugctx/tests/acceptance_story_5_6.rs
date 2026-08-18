#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 5.6 — trybuild 编译失败套件（ATDD / FR39）。
//!
//! 护栏：`tests/ui/` ≥3 例 compile_fail + stderr；中文文档标明已交付。

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
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

fn ui_dir() -> PathBuf {
    workspace_root().join("crates/plugctx/tests/ui")
}

fn ui_rs_cases() -> Vec<String> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(ui_dir()).expect("read tests/ui") {
        let entry = entry.expect("dirent");
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".rs") {
            names.push(name.into_owned());
        }
    }
    names.sort();
    names
}

/// AC#1: trybuild 目录至少 3 个 compile_fail 源文件（FR39）。
#[test]
fn trybuild_ui_has_at_least_three_cases() {
    assert!(
        workspace_root()
            .join("crates/plugctx/tests/ui.rs")
            .is_file(),
        "missing trybuild harness tests/ui.rs"
    );
    let cases = ui_rs_cases();
    assert!(
        cases.len() >= 3,
        "tests/ui must have ≥3 compile_fail .rs files (FR39), found {}: {cases:?}",
        cases.len()
    );
}

/// AC#1: 每个 UI `.rs` 均有配套 `.stderr` 快照。
#[test]
fn trybuild_ui_cases_have_stderr_snapshots() {
    for name in ui_rs_cases() {
        let stem = name.trim_end_matches(".rs");
        let stderr = ui_dir().join(format!("{stem}.stderr"));
        assert!(
            stderr.is_file(),
            "missing trybuild stderr snapshot for {name}: {}",
            stderr.display()
        );
    }
}

/// AC#1: 用例覆盖设计 §8.6 意图（Plugin 边界 / build 返回 / 非 static 或 feature 门控）。
#[test]
fn trybuild_cases_cover_api_misuse_intents() {
    let cases = ui_rs_cases();
    let joined = cases.join(" ");
    assert!(
        joined.contains("plugin_must_impl_trait"),
        "must retain plugin_must_impl_trait (未实现 Plugin)"
    );
    assert!(
        cases
            .iter()
            .any(|c| c.contains("build") || c.contains("return")),
        "must include Plugin::build wrong-return (or类似) case, got {cases:?}"
    );
    assert!(
        cases.iter().any(|c| {
            c.contains("static")
                || c.contains("event")
                || c.contains("start_async")
                || c.contains("emit_parallel")
                || c.contains("feature")
        }),
        "must include non-'static event or feature-gated API misuse case, got {cases:?}"
    );
}

/// 文档：testing.md 标明 trybuild ≥3 已交付（Story 5.6）。
#[test]
fn testing_doc_documents_trybuild_suite_5_6() {
    let doc = read_utf8("docs/testing.md");
    for needle in ["trybuild", "compile_fail", "tests/ui", "≥3", "5.6"] {
        assert!(
            doc.contains(needle),
            "docs/testing.md must mention `{needle}` for Story 5.6 delivery"
        );
    }
    assert!(
        doc.contains("已交付") || doc.contains("完整"),
        "docs/testing.md must mark trybuild suite as delivered"
    );
}

/// README / CI 指向完整 trybuild 套件（非仅骨架文案）。
#[test]
fn readme_and_ci_mention_trybuild_suite() {
    let readme = read_utf8("README.md");
    assert!(
        readme.contains("cargo test -p plugctx --test ui"),
        "README must show trybuild command"
    );
    assert!(
        readme.contains("≥3") && readme.contains("已交付"),
        "README must mark trybuild ≥3 as delivered"
    );
    assert!(
        !readme.contains("扩容见 Story 5.6"),
        "README must not still defer trybuild expansion to Story 5.6"
    );

    let script = read_utf8("scripts/ci-test.sh");
    assert!(
        script.contains("--test ui"),
        "ci-test.sh must run plugctx --test ui"
    );
    assert!(
        script.contains("trybuild UI suite") || script.to_lowercase().contains("compile_fail"),
        "ci-test.sh should label trybuild as suite (not skeleton-only)"
    );
}

/// Automate：stderr 快照非空，且 harness 使用 compile_fail。
#[test]
fn trybuild_harness_and_stderr_nonempty() {
    let harness =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/ui.rs"))
            .expect("ui.rs");
    assert!(
        harness.contains("compile_fail"),
        "tests/ui.rs must call trybuild compile_fail"
    );
    for name in ui_rs_cases() {
        let stem = name.trim_end_matches(".rs");
        let stderr = ui_dir().join(format!("{stem}.stderr"));
        let body = std::fs::read_to_string(&stderr).unwrap_or_else(|e| {
            panic!("read {}: {e}", stderr.display());
        });
        assert!(
            body.contains("error"),
            "stderr for {name} should contain rustc error output"
        );
    }
}
