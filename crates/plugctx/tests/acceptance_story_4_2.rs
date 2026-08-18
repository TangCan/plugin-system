//! Acceptance tests for story 4.2 — dynamic-native（C ABI + libloading，逻辑卸载）。
//!
//! 需先构建示例插件，再启用 feature：
//! `cargo build -p hello_plugin -p echo_plugin`
//! `cargo test -p plugctx --features dynamic-native --test acceptance_story_4_2`

#![cfg(feature = "dynamic-native")]

use std::path::PathBuf;

use plugctx::{
    load_native_plugin, load_native_plugin_with_host_abi, Context, Error, NativeInvoker,
    PLUGIN_ABI_VERSION,
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

/// AC#1: 符合 ABI 的 cdylib 可加载并安装进 Context，参与 start。
#[test]
fn load_hello_install_and_start() {
    let plugin = load_native_plugin(example_lib("hello_plugin")).expect("load hello");
    assert_eq!(plugin.name(), "hello");

    let ctx = Context::new();
    let handle = ctx.plugin(plugin).expect("install");
    ctx.start().expect("start");
    assert!(handle.is_alive());

    let invoker = ctx
        .get::<NativeInvoker>()
        .expect("native invoker provided in build");
    let out = invoker.call("greet", b"world").expect("greet");
    assert_eq!(String::from_utf8_lossy(&out), "hello, world");
}

/// AC#1: echo 插件同样可走 Context 路径。
#[test]
fn load_echo_and_call_via_context() {
    let plugin = load_native_plugin(example_lib("echo_plugin")).expect("load echo");
    assert_eq!(plugin.name(), "echo");

    let ctx = Context::new();
    let _handle = ctx.plugin(plugin).expect("install");
    ctx.start().expect("start");

    let invoker = ctx.get::<NativeInvoker>().expect("invoker");
    let out = invoker.call("echo", b"ping").expect("echo");
    assert_eq!(out, b"ping");
}

/// AC#2: ABI 不匹配返回明确错误，且不执行 create（通过伪造 host abi 触发）。
#[test]
fn abi_mismatch_rejects_before_create() {
    let path = example_lib("hello_plugin");
    let err = match load_native_plugin_with_host_abi(&path, PLUGIN_ABI_VERSION.wrapping_add(1)) {
        Ok(_) => panic!("expected abi mismatch"),
        Err(e) => e,
    };
    match err {
        Error::AbiMismatch {
            plugin,
            host,
            path: p,
        } => {
            assert_eq!(plugin, PLUGIN_ABI_VERSION);
            assert_eq!(host, PLUGIN_ABI_VERSION.wrapping_add(1));
            assert!(p.exists() || p.to_string_lossy().contains("hello_plugin"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

/// AC#3: dispose = 逻辑卸载（撤销 Context 注册）；不要求 dlclose。
#[test]
fn dispose_is_logical_unload_not_dlclose() {
    let plugin = load_native_plugin(example_lib("hello_plugin")).expect("load");
    let library_kept = plugin.library_mapping_retained();

    let ctx = Context::new();
    let handle = ctx.plugin(plugin).expect("install");
    ctx.start().expect("start");
    assert!(ctx.get::<NativeInvoker>().is_some());

    handle.dispose().expect("dispose");
    assert!(!handle.is_alive());
    assert!(
        ctx.get::<NativeInvoker>().is_none(),
        "逻辑卸载后应撤销插件 build 登记的服务"
    );
    assert!(
        library_kept,
        "NativePlugin 契约：默认保留动态库映射，不以 dlclose 为正确性前提"
    );

    // 同路径再次加载成功（证明先前卸载未依赖 dlclose 正确性）。
    let again = load_native_plugin(example_lib("hello_plugin")).expect("reload");
    assert_eq!(again.name(), "hello");
}
