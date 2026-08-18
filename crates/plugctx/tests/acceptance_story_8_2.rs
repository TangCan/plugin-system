//! Acceptance tests for story 8.2 — PluginBackend 双路径共存 / 分制品（FR48）。
//!
//! ```bash
//! cargo test -p plugctx --features "dynamic-wasm,dynamic-wasm-component" --test acceptance_story_8_2
//! ```

#![cfg(all(feature = "dynamic-wasm", feature = "dynamic-wasm-component"))]

use plugctx::{
    bundled_component_add_wat, bundled_echo_wasm, wasm_artifact_with_meta, ComponentInvoker,
    ComponentLoader, Context, DynamicLoader, DynamicSource, Error, PluginBackend,
    PluginBackendKind, WasmInvoker, WasmLoader, WASM_ABI_VERSION,
};

fn extism_artifact(name: &str) -> Vec<u8> {
    wasm_artifact_with_meta(bundled_echo_wasm(), name, WASM_ABI_VERSION)
}

/// AC#1: 同 Context 安装 Extism 与 Component 插件，均可 start / 调用 / dispose。
#[test]
fn dual_backends_share_one_context_lifecycle() {
    let extism = PluginBackend::Extism(WasmLoader::default())
        .load(DynamicSource::Bytes(&extism_artifact("fr48-extism")))
        .expect("extism load");
    let component = PluginBackend::Component(ComponentLoader)
        .load(DynamicSource::Bytes(bundled_component_add_wat().as_bytes()))
        .expect("component load");

    let ctx = Context::new();
    let h_extism = ctx.plugin(extism).expect("install extism");
    let h_component = ctx.plugin(component).expect("install component");
    ctx.start().expect("start");
    assert!(h_extism.is_alive());
    assert!(h_component.is_alive());

    let echo = ctx
        .get::<WasmInvoker>()
        .expect("WasmInvoker")
        .call("echo", b"ping")
        .expect("echo");
    assert_eq!(echo, b"ping");

    let sum = ctx
        .get::<ComponentInvoker>()
        .expect("ComponentInvoker")
        .call_add(20, 22)
        .expect("add");
    assert_eq!(sum, 42);

    h_extism.dispose().expect("dispose extism");
    assert!(!h_extism.is_alive());
    assert!(ctx.get::<WasmInvoker>().is_none());
    // Component 路径仍可用
    assert_eq!(
        ctx.get::<ComponentInvoker>()
            .expect("component still alive")
            .call_add(1, 1)
            .expect("add"),
        2
    );

    h_component.dispose().expect("dispose component");
    assert!(!h_component.is_alive());
    assert!(ctx.get::<ComponentInvoker>().is_none());
}

/// AC#1: PluginBackend::kind 区分后端；经统一 load 入口。
#[test]
fn plugin_backend_selects_by_kind() {
    let extism = PluginBackend::Extism(WasmLoader::default());
    let component = PluginBackend::Component(ComponentLoader);
    assert_eq!(extism.kind(), PluginBackendKind::Extism);
    assert_eq!(component.kind(), PluginBackendKind::Component);

    let as_loader: &dyn DynamicLoader = &extism;
    let boxed = as_loader
        .load(DynamicSource::Bytes(&extism_artifact("via-backend")))
        .expect("load");
    let ctx = Context::new();
    ctx.plugin(boxed).expect("install");
    ctx.start().expect("start");
    assert!(ctx.get::<WasmInvoker>().is_some());
}

/// AC#1 / FR48: Extism 制品不得被 Component 后端「两吃」。
#[test]
fn component_backend_rejects_extism_artifact() {
    let err = match PluginBackend::Component(ComponentLoader)
        .load(DynamicSource::Bytes(&extism_artifact("not-a-component")))
    {
        Ok(_) => panic!("Extism PDK wasm must not load via Component backend"),
        Err(e) => e,
    };
    assert!(
        matches!(err, Error::WasmLoad { .. }),
        "expected WasmLoad, got {err:?}"
    );
}

/// AC#1 / FR48: Component WAT 不得被 Extism 后端「两吃」。
#[test]
fn extism_backend_rejects_component_wat() {
    let err = match PluginBackend::Extism(WasmLoader::default())
        .load(DynamicSource::Bytes(bundled_component_add_wat().as_bytes()))
    {
        Ok(_) => panic!("Component WAT must not load via Extism backend"),
        Err(e) => e,
    };
    assert!(
        matches!(err, Error::WasmLoad { .. } | Error::WasmAbiMismatch { .. }),
        "expected load rejection, got {err:?}"
    );
}

/// AC#2: 公开文档钉死分制品，禁止暗示一份二进制两吃。
#[test]
fn docs_forbid_single_wasm_dual_use() {
    let docs_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let readme = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../README.md");
    let files = [
        docs_root.join("feature-matrix.md"),
        docs_root.join("component-model-versions.md"),
        docs_root.join("testing.md"),
        readme,
    ];
    let mut combined = String::new();
    for path in &files {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        combined.push_str(&text);
        combined.push('\n');
    }
    let lower = combined.to_ascii_lowercase();
    assert!(
        lower.contains("分制品")
            || lower.contains("双制品")
            || lower.contains("不得") && lower.contains("两吃"),
        "docs must state dual artifacts / forbid dual-eat"
    );
    assert!(
        lower.contains("pluginbackend") || lower.contains("plugin backend"),
        "docs must mention PluginBackend"
    );
    assert!(
        !(lower.contains("同一.wasm两吃") || lower.contains("一份.wasm两吃且可用")),
        "docs must not claim one wasm works on both backends"
    );
}
