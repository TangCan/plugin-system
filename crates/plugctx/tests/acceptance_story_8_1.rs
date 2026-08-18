//! Acceptance tests for story 8.1 — `dynamic-wasm-component` 宿主嵌入骨架（FR47, NFR12, NFR14）。
//!
//! `cargo test -p plugctx --features dynamic-wasm-component --test acceptance_story_8_1`

#![cfg(feature = "dynamic-wasm-component")]

use std::process::Command;

use plugctx::{bundled_component_add_wat, load_wasm_component, Error};

/// AC#1: 启用 feature 后可加载合法组件并成功调用导出 `add`。
#[test]
fn load_component_and_call_export() {
    let plugin = load_wasm_component(bundled_component_add_wat().as_bytes()).expect("load");
    assert_eq!(plugin.call_add(40, 2).expect("add"), 42);
    assert_eq!(plugin.call_add(-1, 1).expect("add"), 0);
}

/// AC#1: 非法制品拒绝加载。
#[test]
fn invalid_artifact_rejects_load() {
    let err = match load_wasm_component(b"not-a-component") {
        Ok(_) => panic!("expected load failure"),
        Err(e) => e,
    };
    assert!(
        matches!(err, Error::WasmLoad { .. }),
        "expected WasmLoad, got {err:?}"
    );
}

/// AC#2: 默认 features 下依赖图不含 wasmtime（NFR14）。
#[test]
fn default_cargo_tree_excludes_wasmtime() {
    let output = Command::new(env!("CARGO"))
        .args(["tree", "-p", "plugctx", "-e", "normal", "--prefix", "none"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo tree");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
    assert!(
        !tree.lines().any(|l| l.starts_with("wasmtime ")),
        "default normal deps must not include wasmtime (NFR14):\n{tree}"
    );
}

/// AC#3: 版本钉死文档存在且含 wasmtime / wit-bindgen / CM 1.0 适配要点（NFR12）。
#[test]
fn version_pin_doc_covers_compat_matrix() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/component-model-versions.md");
    let text =
        std::fs::read_to_string(&root).unwrap_or_else(|e| panic!("read {}: {e}", root.display()));
    let lower = text.to_ascii_lowercase();
    assert!(
        lower.contains("wasmtime") && lower.contains("47"),
        "doc must pin wasmtime 47.x"
    );
    assert!(
        lower.contains("wit-bindgen"),
        "doc must mention wit-bindgen guest toolchain"
    );
    assert!(
        lower.contains("1.0") || lower.contains("component model"),
        "doc must state CM 1.0 adaptation expectation"
    );
}
