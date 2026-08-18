//! Acceptance tests for story 4.5 — DynamicLoader trait 与统一加载 API。
//!
//! ```bash
//! cargo build -p hello_plugin -p echo_plugin
//! cargo test -p plugctx --features "dynamic-native,dynamic-wasm" --test acceptance_story_4_5
//! ```

#![cfg(all(feature = "dynamic-native", feature = "dynamic-wasm"))]

use std::path::PathBuf;

use plugctx::{
    bundled_echo_wasm, wasm_artifact_with_meta, Context, DylibLoader, DynamicLoader, DynamicSource,
    Error, NativeInvoker, WasmInvoker, WasmLoadConfig, WasmLoader, PLUGIN_ABI_VERSION,
    WASM_ABI_VERSION,
};

fn example_lib(name: &str) -> PathBuf {
    let mut candidates = Vec::new();
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(dir));
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    candidates.push(PathBuf::from("target"));

    let file = if cfg!(target_os = "windows") {
        format!("{name}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{name}.dylib")
    } else {
        format!("lib{name}.so")
    };

    for root in candidates {
        for profile in ["debug", "release"] {
            let p = root.join(profile).join(&file);
            if p.exists() {
                return p;
            }
        }
    }
    panic!("built plugin `{file}` not found; run `cargo build -p {name}` first");
}

fn sample_artifact(name: &str) -> Vec<u8> {
    wasm_artifact_with_meta(bundled_echo_wasm(), name, WASM_ABI_VERSION)
}

/// AC#1: DylibLoader → Box<dyn Plugin> → Context::plugin。
#[test]
fn dylib_loader_installs_via_context_plugin() {
    let path = example_lib("hello_plugin");
    let boxed = DylibLoader
        .load(DynamicSource::Path(&path))
        .expect("dylib load");
    let ctx = Context::new();
    let handle = ctx.plugin(boxed).expect("install");
    ctx.start().expect("start");
    assert!(handle.is_alive());
    let out = ctx
        .get::<NativeInvoker>()
        .expect("NativeInvoker")
        .call("greet", b"loader")
        .expect("greet");
    assert_eq!(String::from_utf8_lossy(&out), "hello, loader");
}

/// AC#1: WasmLoader（字节）→ Context::plugin。
#[test]
fn wasm_loader_bytes_installs_via_context_plugin() {
    let artifact = sample_artifact("via-loader");
    let boxed = WasmLoader::default()
        .load(DynamicSource::Bytes(&artifact))
        .expect("wasm load");
    let ctx = Context::new();
    let handle = ctx.plugin(boxed).expect("install");
    ctx.start().expect("start");
    assert!(handle.is_alive());
    let out = ctx
        .get::<WasmInvoker>()
        .expect("WasmInvoker")
        .call("echo", b"ok")
        .expect("echo");
    assert_eq!(out, b"ok");
}

/// AC#1: WasmLoader 从路径读文件后加载。
#[test]
fn wasm_loader_path_reads_then_loads() {
    let dir = std::env::temp_dir();
    let path = dir.join("plugctx_story45_wasm.artifact");
    std::fs::write(&path, sample_artifact("from-path")).expect("write artifact");
    let boxed = WasmLoader::new(WasmLoadConfig::default())
        .load(DynamicSource::Path(&path))
        .expect("path load");
    let _ = std::fs::remove_file(&path);

    let ctx = Context::new();
    ctx.plugin(boxed).expect("install");
    ctx.start().expect("start");
    assert_eq!(
        ctx.get::<WasmInvoker>().expect("invoker").name(),
        "from-path"
    );
}

/// AC#2: 缺失路径 — 可诊断错误；Context 仍可用。
#[test]
fn missing_native_path_does_not_half_init_context() {
    let err = match DylibLoader.load(DynamicSource::Path(std::path::Path::new(
        "/nonexistent/libmissing_plugin.so",
    ))) {
        Ok(_) => panic!("expected load failure"),
        Err(e) => e,
    };
    assert!(matches!(err, Error::NativeLoad { .. }));

    let ctx = Context::new();
    let artifact = sample_artifact("after-fail");
    let boxed = WasmLoader::default()
        .load(DynamicSource::Bytes(&artifact))
        .expect("wasm ok after native fail");
    ctx.plugin(boxed).expect("install after fail");
    ctx.start().expect("start");
    assert!(ctx.get::<WasmInvoker>().is_some());
}

/// AC#2: WASM ABI 不符 — 拒绝加载；Context 未半初始化。
#[test]
fn wasm_abi_mismatch_via_loader_leaves_context_clean() {
    let bad = wasm_artifact_with_meta(bundled_echo_wasm(), "bad", WASM_ABI_VERSION.wrapping_add(7));
    let err = match WasmLoader::default().load(DynamicSource::Bytes(&bad)) {
        Ok(_) => panic!("expected abi mismatch"),
        Err(e) => e,
    };
    assert!(matches!(err, Error::WasmAbiMismatch { .. }));

    let ctx = Context::new();
    ctx.start().expect("empty start still works");
    assert!(ctx.get::<WasmInvoker>().is_none());
}

/// AC#2: DylibLoader 拒绝 Bytes；不触碰 Context。
#[test]
fn dylib_loader_rejects_bytes_source() {
    let err = match DylibLoader.load(DynamicSource::Bytes(b"not-a-dylib")) {
        Ok(_) => panic!("expected rejection"),
        Err(e) => e,
    };
    assert!(matches!(err, Error::NativeLoad { .. }));
    let _ = PLUGIN_ABI_VERSION; // 文档/契约可达
}
