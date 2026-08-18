#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 5.7 — 核心路径基准测试（ATDD / FR40）。
//!
//! 护栏：criterion `core_paths` bench 覆盖 get/emit/start；中文文档说明运行与基线流程。

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

/// AC#1: Cargo.toml 声明 harness=false 的 core_paths bench，且 criterion 仅在 dev-dependencies。
#[test]
fn cargo_toml_declares_core_paths_bench() {
    let toml = read_utf8("crates/plugctx/Cargo.toml");
    assert!(
        toml.contains("name = \"core_paths\"") || toml.contains("name = 'core_paths'"),
        "plugctx Cargo.toml must declare [[bench]] name = core_paths"
    );
    assert!(
        toml.contains("harness = false"),
        "core_paths bench must set harness = false for criterion"
    );
    assert!(
        toml.contains("criterion"),
        "plugctx must depend on criterion (dev)"
    );
    // criterion 不得进入正常 [dependencies] 段（粗检：dev-dependencies 之后才出现亦可；
    // 以「未在 [dependencies] 与 [dev-dependencies] 之间单独出现」更稳——此处要求 workspace/dev）。
    let deps_idx = toml.find("[dependencies]").expect("[dependencies]");
    let dev_idx = toml.find("[dev-dependencies]").expect("[dev-dependencies]");
    let runtime_deps = &toml[deps_idx..dev_idx];
    assert!(
        !runtime_deps.contains("criterion"),
        "criterion must not be in [dependencies] (NFR1)"
    );
}

/// AC#1: benches/core_paths.rs 存在且覆盖 get / emit / start。
#[test]
fn core_paths_bench_covers_get_emit_start() {
    let bench_path = workspace_root().join("crates/plugctx/benches/core_paths.rs");
    assert!(bench_path.is_file(), "missing benches/core_paths.rs (FR40)");
    let src = std::fs::read_to_string(&bench_path).expect("read core_paths.rs");
    for needle in ["get", "emit", "start"] {
        assert!(
            src.to_lowercase().contains(needle),
            "benches/core_paths.rs must cover `{needle}` (FR40)"
        );
    }
    assert!(
        src.contains("criterion") || src.contains("Criterion"),
        "bench must use criterion"
    );
    for id in ["core_get", "core_emit", "core_start"] {
        assert!(
            src.contains(id),
            "benches/core_paths.rs must register criterion id `{id}`"
        );
    }
}

/// Automate：workspace 声明 criterion；CI 文案标明仅编译。
#[test]
fn workspace_and_ci_label_bench_compile_only() {
    let ws = read_utf8("Cargo.toml");
    assert!(
        ws.contains("criterion"),
        "workspace Cargo.toml must declare criterion (Story 5.7)"
    );
    let script = read_utf8("scripts/ci-test.sh");
    assert!(
        script.contains("FR40") || script.to_lowercase().contains("core path"),
        "ci-test.sh should label core path bench compile gate"
    );
}

/// 文档：testing.md 标明基准已交付，含运行与基线说明。
#[test]
fn testing_doc_documents_bench_baseline_5_7() {
    let doc = read_utf8("docs/testing.md");
    for needle in ["bench", "criterion", "get", "emit", "start", "5.7"] {
        assert!(
            doc.to_lowercase().contains(&needle.to_lowercase()) || doc.contains(needle),
            "docs/testing.md must mention `{needle}` for Story 5.7"
        );
    }
    assert!(
        doc.contains("已交付"),
        "docs/testing.md must mark benches as delivered"
    );
    assert!(
        doc.contains("save-baseline") || doc.contains("基线") || doc.contains("--baseline"),
        "docs/testing.md must document baseline procedure"
    );
    assert!(
        doc.contains("cargo bench"),
        "docs/testing.md must show cargo bench command"
    );
}

/// README / CI：说明如何跑；默认 CI 不跑满量 bench，可 --no-run。
#[test]
fn readme_and_ci_document_bench_policy() {
    let readme = read_utf8("README.md");
    assert!(
        readme.contains("cargo bench") && readme.contains("core_paths"),
        "README must document cargo bench -p plugctx --bench core_paths"
    );
    assert!(
        readme.contains("已交付") || readme.contains("FR40"),
        "README should mark core path benches delivered"
    );

    let script = read_utf8("scripts/ci-test.sh");
    assert!(
        script.contains("bench") && script.contains("--no-run"),
        "ci-test.sh must compile benches with cargo bench --no-run (not full run)"
    );
    // 避免默认脚本执行满量 bench（慢）
    let without_comments: String = script
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !without_comments.contains("cargo bench -p plugctx --bench core_paths\n")
            && !without_comments.contains("cargo bench -p plugctx\n"),
        "ci-test.sh must not run full cargo bench by default"
    );
}
