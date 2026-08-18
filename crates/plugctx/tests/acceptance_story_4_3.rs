//! Acceptance tests for story 4.3 — dynamic-wasm（实例加载与显式关闭）。
//!
//! Extism 真实引擎 + 内置 `testdata/echo.wasm`。
//! `cargo test -p plugctx --features dynamic-wasm --test acceptance_story_4_3`

#![cfg(feature = "dynamic-wasm")]

use plugctx::{
    bundled_echo_wasm, load_wasm_plugin, wasm_artifact_with_meta, Context, Error, WasmInvoker,
    WasmLoadConfig, WASM_ABI_VERSION,
};

fn sample_artifact(name: &str) -> Vec<u8> {
    wasm_artifact_with_meta(bundled_echo_wasm(), name, WASM_ABI_VERSION)
}

/// AC#1: 合法制品可加载并安装进 Context，参与 start 与调用。
#[test]
fn load_install_start_and_call() {
    let plugin =
        load_wasm_plugin(sample_artifact("demo"), WasmLoadConfig::default()).expect("load wasm");
    assert_eq!(plugin.name(), "demo");
    assert!(!plugin.is_closed());
    assert!(!plugin.runtime_freed());

    let ctx = Context::new();
    let handle = ctx.plugin(plugin).expect("install");
    ctx.start().expect("start");
    assert!(handle.is_alive());

    let out = {
        let invoker = ctx
            .get::<WasmInvoker>()
            .expect("wasm invoker provided in build");
        invoker.call("echo", b"ping").expect("echo")
    };
    assert_eq!(out, b"ping");
}

/// AC#1: 安装前可直接经 WasmPlugin::call。
#[test]
fn call_via_plugin_before_install() {
    let plugin = load_wasm_plugin(sample_artifact("pre"), WasmLoadConfig::default()).expect("load");
    let out = plugin.call("echo", b"hi").expect("call");
    assert_eq!(out, b"hi");
}

/// AC#2: dispose → Effect 显式 close；服务撤销；close 后不可 call。
#[test]
fn dispose_explicitly_closes_and_revokes_invoker() {
    let plugin =
        load_wasm_plugin(sample_artifact("close-me"), WasmLoadConfig::default()).expect("load");

    let ctx = Context::new();
    let handle = ctx.plugin(plugin).expect("install");
    ctx.start().expect("start");

    let invoker = {
        let guard = ctx.get::<WasmInvoker>().expect("invoker");
        (*guard).clone()
    };
    assert_eq!(invoker.explicit_close_count(), 0);
    assert!(!invoker.runtime_freed());

    handle.dispose().expect("dispose");
    assert!(!handle.is_alive());
    assert!(
        ctx.get::<WasmInvoker>().is_none(),
        "dispose 后应撤销 WasmInvoker"
    );
    assert!(
        invoker.explicit_close_count() >= 1,
        "FR26: dispose 必须显式 close，不得仅依赖后续 Drop"
    );
    assert!(invoker.is_closed());
    assert!(
        invoker.runtime_freed(),
        "FR26: 实例关闭后运行时资源必须 freed"
    );

    let err = match invoker.call("echo", b"x") {
        Ok(_) => panic!("expected call after close to fail"),
        Err(e) => e,
    };
    assert!(
        matches!(err, Error::WasmClosed { .. }),
        "unexpected error: {err:?}"
    );
}

/// AC#2: 空制品拒绝加载。
#[test]
fn empty_artifact_rejects_load() {
    let err = match load_wasm_plugin([], WasmLoadConfig::default()) {
        Ok(_) => panic!("expected load failure"),
        Err(e) => e,
    };
    assert!(
        matches!(err, Error::WasmLoad { .. }),
        "expected WasmLoad, got {err:?}"
    );
}
