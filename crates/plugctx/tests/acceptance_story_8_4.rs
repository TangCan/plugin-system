//! Acceptance tests for story 8.4 — 最小 WIT world + wasm32-wasip2 样例客人（FR50, NFR12）。
//!
//! `cargo test -p plugctx --features dynamic-wasm-component --test acceptance_story_8_4`

#![cfg(feature = "dynamic-wasm-component")]

use plugctx::{bundled_wit_sample_add_wasm, load_wasm_component, ComponentInvoker, Context, Error};

/// AC#1: 检入的 wasip2 WIT 客人制品可被宿主加载并成功调用 `add`。
#[test]
fn wit_sample_guest_loads_and_host_calls_add() {
    let plugin = load_wasm_component(bundled_wit_sample_add_wasm()).expect("load wit sample");
    assert_eq!(plugin.call_add(40, 2).expect("add"), 42);
    assert_eq!(plugin.call_add(-1, 1).expect("add"), 0);
}

/// AC#1: 经 Context 安装后仍可调用（与 8.2/8.3 路径一致）。
#[test]
fn wit_sample_guest_via_context_plugin() {
    let plugin = load_wasm_component(bundled_wit_sample_add_wasm()).expect("load");
    let ctx = Context::new();
    let _handle = ctx.plugin(plugin).expect("install");
    ctx.start().expect("start");
    let invoker = ctx.get::<ComponentInvoker>().expect("invoker");
    assert_eq!(invoker.call_add(7, 8).expect("add"), 15);
}

/// AC#2: WIT 源与构建/版本文档存在（FR50, NFR12）。
#[test]
fn wit_world_and_toolchain_docs_exist() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let wit = manifest.join("../../guests/wit-sample/wit/world.wit");
    let wit_text =
        std::fs::read_to_string(&wit).unwrap_or_else(|e| panic!("read {}: {e}", wit.display()));
    assert!(
        wit_text.contains("world sample") && wit_text.contains("add"),
        "WIT must declare sample world with add"
    );

    let versions = manifest.join("../../docs/component-model-versions.md");
    let text = std::fs::read_to_string(&versions)
        .unwrap_or_else(|e| panic!("read {}: {e}", versions.display()));
    let lower = text.to_ascii_lowercase();
    assert!(
        lower.contains("wasm32-wasip2"),
        "doc must name target triple wasm32-wasip2"
    );
    assert!(
        lower.contains("wit-bindgen") && lower.contains("0.60"),
        "doc must pin wit-bindgen 0.60.x"
    );
    assert!(
        lower.contains("build-wit-sample-guest") || lower.contains("wit-sample"),
        "doc must point to guest rebuild path"
    );

    let testing = manifest.join("../../docs/testing.md");
    let testing_text = std::fs::read_to_string(&testing)
        .unwrap_or_else(|e| panic!("read {}: {e}", testing.display()));
    assert!(
        testing_text.contains("acceptance_story_8_4")
            || testing_text.contains("8_4")
            || testing_text.contains("FR50"),
        "testing.md must list 8.4 / FR50 entry"
    );
}

/// 护栏：空字节仍拒绝（与 8.1 一致，确保 WIT 路径未改坏加载错误面）。
#[test]
fn empty_bytes_still_reject() {
    assert!(matches!(
        load_wasm_component([]),
        Err(Error::WasmLoad { .. })
    ));
}
