//! Acceptance tests for story 4.4 — 动态插件接入 Context 与 ABI 版本协商。
//!
//! ```bash
//! cargo build -p hello_plugin -p echo_plugin
//! cargo test -p plugctx --features "dynamic-native,dynamic-wasm" --test acceptance_story_4_4
//! ```

#![cfg(all(feature = "dynamic-native", feature = "dynamic-wasm"))]

use std::any::TypeId;
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

use plugctx::{
    bundled_echo_wasm, load_native_plugin, load_native_plugin_with_host_abi, load_wasm_plugin,
    load_wasm_plugin_with_host_abi, wasm_artifact_with_meta, Context, Error, NativeInvoker, Plugin,
    WasmInvoker, WasmLoadConfig, PLUGIN_ABI_VERSION, WASM_ABI_VERSION,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token(&'static str);

struct ProviderPlugin;

impl Plugin for ProviderPlugin {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        ctx.provide(Token("from-inproc"));
        Ok(())
    }
}

struct NeedsWasmInvoker {
    saw: Rc<Cell<bool>>,
}

impl Plugin for NeedsWasmInvoker {
    fn dependencies(&self) -> Vec<TypeId> {
        vec![TypeId::of::<WasmInvoker>()]
    }

    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        let inv = ctx
            .get::<WasmInvoker>()
            .expect("WasmInvoker must be ordered before this consumer");
        assert_eq!(inv.name(), "dep-src");
        self.saw.set(true);
        Ok(())
    }
}

struct NeedsToken {
    saw: Rc<Cell<bool>>,
}

impl Plugin for NeedsToken {
    fn dependencies(&self) -> Vec<TypeId> {
        vec![TypeId::of::<Token>()]
    }

    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        let t = ctx.get::<Token>().expect("Token");
        assert_eq!(t.0, "from-inproc");
        self.saw.set(true);
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct Ping;

/// AC#1: 动态适配器延迟安装 — start 前不 build（无 WasmInvoker）。
#[test]
fn dynamic_adapter_delayed_until_start() {
    let plugin =
        load_wasm_plugin(sample_artifact("delayed"), WasmLoadConfig::default()).expect("load wasm");
    let ctx = Context::new();
    let handle = ctx.plugin(plugin).expect("install");
    assert!(
        ctx.get::<WasmInvoker>().is_none(),
        "延迟安装：start 前不得 build"
    );
    ctx.start().expect("start");
    assert!(handle.is_alive());
    assert!(ctx.get::<WasmInvoker>().is_some());
}

/// AC#1: start 后立即安装动态适配器。
#[test]
fn dynamic_adapter_immediate_after_start() {
    let ctx = Context::new();
    ctx.start().expect("start empty");
    assert!(ctx.get::<WasmInvoker>().is_none());

    let plugin =
        load_wasm_plugin(sample_artifact("immediate"), WasmLoadConfig::default()).expect("load");
    let handle = ctx.plugin(plugin).expect("immediate install");
    assert!(handle.is_alive());
    assert!(
        ctx.get::<WasmInvoker>().is_some(),
        "立即安装：plugin 后应已 build"
    );
}

/// AC#1: dispose 按 scope 卸载动态插件（撤销 invoker + wasm close）。
#[test]
fn dynamic_adapter_dispose_matches_inproc_scope() {
    let plugin =
        load_wasm_plugin(sample_artifact("dispose"), WasmLoadConfig::default()).expect("load");
    let ctx = Context::new();
    let handle = ctx.plugin(plugin).expect("install");
    ctx.start().expect("start");

    let invoker = {
        let g = ctx.get::<WasmInvoker>().expect("invoker");
        (*g).clone()
    };
    handle.dispose().expect("dispose");
    assert!(!handle.is_alive());
    assert!(ctx.get::<WasmInvoker>().is_none());
    assert!(invoker.explicit_close_count() >= 1);
    assert!(invoker.is_closed());
}

/// AC#2: WASM ABI 不匹配拒绝加载。
#[test]
fn wasm_abi_mismatch_rejects_load() {
    let err = match load_wasm_plugin_with_host_abi(
        sample_artifact("bad-abi"),
        WasmLoadConfig::default(),
        WASM_ABI_VERSION.wrapping_add(3),
    ) {
        Ok(_) => panic!("expected mismatch"),
        Err(e) => e,
    };
    match err {
        Error::WasmAbiMismatch { plugin, host } => {
            assert_eq!(plugin, WASM_ABI_VERSION);
            assert_eq!(host, WASM_ABI_VERSION.wrapping_add(3));
        }
        other => panic!("unexpected: {other:?}"),
    }
}

/// AC#2: native PLUGIN_ABI_VERSION 协商仍生效。
#[test]
fn native_abi_mismatch_still_rejects() {
    let path = example_lib("hello_plugin");
    let err = match load_native_plugin_with_host_abi(&path, PLUGIN_ABI_VERSION.wrapping_add(1)) {
        Ok(_) => panic!("expected mismatch"),
        Err(e) => e,
    };
    assert!(matches!(err, Error::AbiMismatch { .. }));
}

/// AC#3: 进程内 + Wasm 混合；依赖排序（consumer 依赖 WasmInvoker）。
#[test]
fn mixed_inproc_and_wasm_dependency_order() {
    let saw = Rc::new(Cell::new(false));
    let wasm =
        load_wasm_plugin(sample_artifact("dep-src"), WasmLoadConfig::default()).expect("wasm");

    let ctx = Context::new();
    // 故意先装 consumer，再装 wasm：乐观排序应先 build wasm。
    ctx.plugin(NeedsWasmInvoker {
        saw: Rc::clone(&saw),
    })
    .expect("consumer");
    ctx.plugin(wasm).expect("wasm");
    ctx.start().expect("start");

    assert!(saw.get(), "依赖排序后 consumer 应成功 build");
    let out = ctx
        .get::<WasmInvoker>()
        .expect("invoker")
        .call("echo", b"ok")
        .expect("call");
    assert_eq!(out, b"ok");
}

/// AC#3: 进程内 + Native + Wasm；事件与依赖共存。
#[test]
fn mixed_inproc_native_wasm_events() {
    let ping_hits = Rc::new(Cell::new(0));
    let token_saw = Rc::new(Cell::new(false));

    let native = load_native_plugin(example_lib("hello_plugin")).expect("native");
    let wasm = load_wasm_plugin(sample_artifact("mix"), WasmLoadConfig::default()).expect("wasm");

    let ctx = Context::new();
    let hits = Rc::clone(&ping_hits);
    ctx.on(move |_: &Ping| {
        hits.set(hits.get() + 1);
    });

    ctx.plugin(NeedsToken {
        saw: Rc::clone(&token_saw),
    })
    .expect("needs token");
    ctx.plugin(ProviderPlugin).expect("provider");
    ctx.plugin(native).expect("native");
    ctx.plugin(wasm).expect("wasm");
    ctx.start().expect("start");

    assert!(token_saw.get());
    assert!(ctx.get::<NativeInvoker>().is_some());
    assert!(ctx.get::<WasmInvoker>().is_some());

    let greet = ctx
        .get::<NativeInvoker>()
        .expect("native")
        .call("greet", b"mix")
        .expect("greet");
    assert_eq!(String::from_utf8_lossy(&greet), "hello, mix");

    let echo = ctx
        .get::<WasmInvoker>()
        .expect("wasm")
        .call("echo", b"w")
        .expect("echo");
    assert_eq!(echo, b"w");

    ctx.emit(&Ping);
    assert_eq!(ping_hits.get(), 1);
}
