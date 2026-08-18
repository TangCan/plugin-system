#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 5.8 — 扩展模块专项测试（ATDD / FR41）。
//!
//! 巩固既有 3.x / 4.x feature 验收，并以护栏锁定 CI 矩阵与中文文档门禁。
//! 本文件在**默认 features**下运行；不引入扩展运行时依赖（NFR1 / NFR5）。

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

fn path_exists(rel: &str) -> bool {
    workspace_root().join(rel).exists()
}

/// AC#1: 各扩展 feature 已有专项 acceptance（设计 §8.7）。
#[test]
fn extension_acceptance_files_exist_for_fr41() {
    let required = [
        "crates/plugctx/tests/acceptance_story_3_1.rs", // async
        "crates/plugctx/tests/acceptance_story_3_2.rs", // parallel
        "crates/plugctx/tests/acceptance_story_4_1.rs", // thread-safe
        "crates/plugctx/tests/acceptance_story_4_2.rs", // dynamic-native
        "crates/plugctx/tests/acceptance_story_4_3.rs", // dynamic-wasm
        "crates/plugctx/tests/acceptance_story_4_4.rs", // native+wasm ABI
        "crates/plugctx/tests/acceptance_story_4_5.rs", // DynamicLoader
    ];
    for rel in required {
        assert!(
            path_exists(rel),
            "missing extension acceptance for FR41: {rel}"
        );
    }
}

/// AC#1: Cargo.toml 为扩展验收声明 required-features（避免默认门误跑）。
#[test]
fn cargo_toml_gates_extension_tests_with_required_features() {
    let toml = read_utf8("crates/plugctx/Cargo.toml");
    let pairs = [
        ("acceptance_story_3_1", "async"),
        ("acceptance_story_3_2", "parallel"),
        ("acceptance_story_4_1", "thread-safe"),
        ("acceptance_story_4_2", "dynamic-native"),
        ("acceptance_story_4_3", "dynamic-wasm"),
    ];
    for (name, feat) in pairs {
        assert!(
            toml.contains(name),
            "Cargo.toml must declare [[test]] {name}"
        );
        // 粗检：文件中存在该 feature 字符串于 required-features 语境
        assert!(
            toml.contains(feat),
            "Cargo.toml must mention feature `{feat}` for gated tests"
        );
    }
    assert!(
        toml.contains("acceptance_story_4_4") && toml.contains("acceptance_story_4_5"),
        "Cargo.toml must declare 4_4 / 4_5 dynamic combo tests"
    );
    // 4_4 / 4_5 须同时要求 native + wasm（自动化护栏）
    let combo = "required-features = [\"dynamic-native\", \"dynamic-wasm\"]";
    let combo_alt = "required-features = ['dynamic-native', 'dynamic-wasm']";
    assert!(
        toml.contains(combo) || toml.contains(combo_alt),
        "4_4/4_5 must require both dynamic-native and dynamic-wasm"
    );
}

/// AC#1: CI 扩展矩阵脚本按 feature 运行专项用例。
#[test]
fn ci_extension_matrix_script_covers_features() {
    assert!(
        path_exists("scripts/ci-extension-matrix.sh"),
        "missing scripts/ci-extension-matrix.sh (FR41)"
    );
    let script = read_utf8("scripts/ci-extension-matrix.sh");
    assert!(
        script.starts_with("#!/") || script.contains("#!/usr/bin/env bash"),
        "ci-extension-matrix.sh must be a bash script"
    );
    assert!(
        script.contains("set -euo pipefail"),
        "ci-extension-matrix.sh must use set -euo pipefail"
    );
    for needle in [
        "FR41",
        "--features async",
        "acceptance_story_3_1",
        "--features parallel",
        "acceptance_story_3_2",
        "--features thread-safe",
        "acceptance_story_4_1",
        "dynamic-native",
        "acceptance_story_4_2",
        "dynamic-wasm",
        "acceptance_story_4_3",
        "acceptance_story_4_4",
        "acceptance_story_4_5",
        "hello_plugin",
        "echo_plugin",
    ] {
        assert!(
            script.contains(needle),
            "ci-extension-matrix.sh must contain `{needle}`"
        );
    }
}

/// AC#1 / NFR5: 主 CI 仍先跑默认 features，并挂接扩展矩阵。
#[test]
fn ci_test_keeps_default_gate_and_invokes_extension_matrix() {
    let script = read_utf8("scripts/ci-test.sh");
    assert!(
        script.contains("cargo test --workspace") || script.contains("cargo test -p plugctx"),
        "ci-test.sh must keep default-features workspace/plugctx gate (NFR5)"
    );
    assert!(
        script.contains("ci-extension-matrix.sh"),
        "ci-test.sh must invoke scripts/ci-extension-matrix.sh"
    );
    assert!(
        script.contains("FR41"),
        "ci-test.sh must label the FR41 extension matrix"
    );
    assert!(
        script.contains("clippy") && script.contains("-D warnings"),
        "ci-test.sh must run clippy -D warnings on publishable crates"
    );
    assert!(
        !script.contains("cargo test --all-features"),
        "ci-test.sh must not use cargo test --all-features as a gate"
    );
}

/// 文档：testing.md / README 标明扩展专项已交付。
#[test]
fn docs_document_extension_matrix_fr41() {
    let testing = read_utf8("docs/testing.md");
    for needle in [
        "FR41",
        "扩展",
        "async",
        "parallel",
        "thread-safe",
        "dynamic",
        "5.8",
    ] {
        assert!(
            testing.contains(needle),
            "docs/testing.md must mention `{needle}` for Story 5.8 / FR41"
        );
    }
    assert!(
        testing.contains("已交付") || testing.contains("ci-extension-matrix"),
        "docs/testing.md must mark extension matrix as delivered"
    );

    let readme = read_utf8("README.md");
    assert!(
        readme.contains("ci-extension-matrix") || readme.contains("FR41"),
        "README must reference extension matrix / FR41"
    );
}
