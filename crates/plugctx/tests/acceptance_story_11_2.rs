//! Acceptance tests for post-0.1.1 story 2.2 — WIT pin 与双 WASM 路径（ATDD / FR5 / NFR4 / NFR1）。

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

/// AC#1: 文档钉死实际 WIT / wasmtime / wit-bindgen，并禁止提前 wasi@0.3.0。
#[test]
fn docs_pin_actual_wit_and_forbid_early_wasi_030() {
    let versions = read_required("docs/component-model-versions.md");
    let guest_readme = read_required("guests/wit-sample/README.md");
    let wit = read_required("guests/wit-sample/wit/world.wit");
    let guest_manifest = read_required("guests/wit-sample/Cargo.toml");
    let workspace = read_required("Cargo.toml");

    assert!(
        wit.contains("plugctx:sample@0.1.0"),
        "样例 WIT 实际 pin 为 plugctx:sample@0.1.0（FR5）\n{wit}"
    );
    assert!(
        !wit.contains("wasi@0.3.0") && !wit.contains("wasi:cli"),
        "样例 WIT 不得提前改钉 wasi@0.3.0（FR5）\n{wit}"
    );
    assert!(
        guest_manifest.contains("wit-bindgen") && guest_manifest.contains("0.60"),
        "客人须钉 wit-bindgen 0.60.x（FR5）"
    );
    assert!(
        workspace.contains("wasmtime") && workspace.contains("47"),
        "工作区须钉 wasmtime 47.x（FR5）"
    );

    for (name, text) in [
        ("component-model-versions.md", versions.as_str()),
        ("guests/wit-sample/README.md", guest_readme.as_str()),
    ] {
        assert!(
            text.contains("wasmtime") && text.contains("47"),
            "{name} 须钉 wasmtime 47.x（FR5）"
        );
        assert!(
            text.contains("wit-bindgen") && text.contains("0.60"),
            "{name} 须钉 wit-bindgen 0.60.x（FR5）"
        );
        assert!(
            text.contains("wasm32-wasip2"),
            "{name} 须钉 wasm32-wasip2（FR5）"
        );
        assert!(
            text.contains("plugctx:sample@0.1.0") || text.contains("world.wit"),
            "{name} 须写明实际 WIT pin（FR5）"
        );
        assert!(
            text.contains("wasi@0.3.0")
                && (text.contains("禁止")
                    || text.contains("不要")
                    || text.contains("不得")
                    || text.contains("提前")),
            "{name} 须写明不要提前改钉 wasi@0.3.0（FR5）"
        );
    }
}

/// AC#2: 双路径分 feature 分制品；WASM 卸载不是 native dlclose。
#[test]
fn dual_wasm_paths_stay_split() {
    let versions = read_required("docs/component-model-versions.md");
    let guest_readme = read_required("guests/wit-sample/README.md");
    for (name, text) in [
        ("component-model-versions.md", versions.as_str()),
        ("guests/wit-sample/README.md", guest_readme.as_str()),
    ] {
        assert!(
            text.contains("dynamic-wasm") && text.contains("dynamic-wasm-component"),
            "{name} 须分写两条 WASM feature（FR5）"
        );
        assert!(
            text.contains("两吃") || text.contains("不能互相") || text.contains("分制品"),
            "{name} 须禁止一份 .wasm 两吃（FR5）"
        );
    }
    assert!(
        (versions.contains("close") && versions.contains("free")) || versions.contains("Drop"),
        "WASM 卸载须为 close/free 或 Drop Store，不是把 CM 写成 dlclose（NFR4）"
    );
    assert!(
        !versions.contains("dlclose")
            || versions.contains("不是")
            || versions.contains("不是 native"),
        "不得把 WASM 卸载写成 native dlclose（NFR4）"
    );
}

/// AC#3: 默认图仍不拉 extism/wasmtime；不做 Fidius 签名包。
#[test]
fn default_graph_and_no_fidius_patch() {
    let manifest = read_required("crates/plugctx/Cargo.toml");
    let features = manifest
        .split("[features]")
        .nth(1)
        .unwrap_or("")
        .split("\n[")
        .next()
        .unwrap_or("");
    assert!(
        features.contains("default = []"),
        "plugctx default 须仍为空（NFR1）\n{features}"
    );
    let default_line = features
        .lines()
        .find(|l| l.trim_start().starts_with("default"))
        .unwrap_or("");
    assert!(
        !default_line.contains("extism")
            && !default_line.contains("wasmtime")
            && !default_line.contains("dynamic-wasm"),
        "不得把 wasm 运行时拉进 default（NFR1）: {default_line}"
    );

    let versions = read_required("docs/component-model-versions.md");
    assert!(
        !versions.to_ascii_lowercase().contains("fidius")
            || versions.contains("不是")
            || versions.contains("不做"),
        "不得把 Fidius 式签名包做成补丁（NFR1）"
    );
}
