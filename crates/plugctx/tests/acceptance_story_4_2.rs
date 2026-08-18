//! Acceptance tests for story 4.2 / 1.1 — dynamic-native（C ABI + libloading，物理卸载）。
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

/// Story 1.1 AC#1/#4: dispose = 先撤销 Context 注册，再 Drop Library（物理卸载）。
#[test]
fn dispose_physically_unloads_library() {
    let plugin = load_native_plugin(example_lib("hello_plugin")).expect("load");

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

    // 同路径再次打开动态库必须成功（映射已释放，不为 ManuallyDrop 泄漏）。
    let again =
        load_native_plugin(example_lib("hello_plugin")).expect("reload after physical unload");
    assert_eq!(again.name(), "hello");
}

/// Story 1.1 AC#2: dispose 前克隆的 NativeInvoker 不可再成功 call（NFR3：Error，非 panic）。
#[test]
fn stale_invoker_call_fails_after_dispose() {
    let plugin = load_native_plugin(example_lib("hello_plugin")).expect("load");
    let ctx = Context::new();
    let handle = ctx.plugin(plugin).expect("install");
    ctx.start().expect("start");

    let invoker = ctx.get::<NativeInvoker>().expect("invoker").clone();
    assert_eq!(
        String::from_utf8_lossy(&invoker.call("greet", b"world").expect("pre-dispose")),
        "hello, world"
    );

    handle.dispose().expect("dispose");

    let err = match invoker.call("greet", b"world") {
        Ok(bytes) => panic!("stale invoker must not succeed, got {bytes:?}"),
        Err(e) => e,
    };
    assert!(
        matches!(
            err,
            Error::NativeCall { .. } | Error::PluginAlreadyDisposed | Error::AlreadyDisposed
        ),
        "expected matchable native/dispose error, got {err:?}"
    );
}

/// Story 1.1：Context 级 dispose 也必须物理卸载（与 PluginHandle::dispose 同一 Drop 路径）。
#[test]
fn context_dispose_physically_unloads_library() {
    let plugin = load_native_plugin(example_lib("hello_plugin")).expect("load");
    let ctx = Context::new();
    let handle = ctx.plugin(plugin).expect("install");
    ctx.start().expect("start");
    let invoker = ctx.get::<NativeInvoker>().expect("invoker").clone();

    ctx.dispose();
    assert!(!handle.is_alive() || ctx.is_disposed());
    let err = invoker
        .call("greet", b"world")
        .expect_err("context dispose must invalidate native invoker");
    assert!(
        matches!(
            err,
            Error::NativeCall { .. } | Error::PluginAlreadyDisposed | Error::AlreadyDisposed
        ),
        "expected matchable error, got {err:?}"
    );
    let again =
        load_native_plugin(example_lib("hello_plugin")).expect("reload after context dispose");
    assert_eq!(again.name(), "hello");
}

/// Story 1.3：用户可见文档不得再写 native「逻辑卸载 ≠ dlclose」。
#[test]
fn user_docs_state_native_physical_unload() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf();
    let files = [
        "README.md",
        "docs/feature-matrix.md",
        "docs/testing.md",
        "CHANGELOG.md",
        "docs/requirements/4. 扩展模块设计.md",
    ];
    for rel in files {
        let text = std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert!(
            !text.contains("逻辑卸载 ≠ `dlclose`") && !text.contains("逻辑卸载 ≠ dlclose"),
            "{rel} still claims native logical-unload ≠ dlclose"
        );
        assert!(
            text.contains("dlclose") || rel == "docs/testing.md",
            "{rel} should mention dlclose for native unload (testing.md may only say 物理卸载)"
        );
    }
    let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
    assert!(
        readme.contains("load → use → dispose → load") || readme.contains("load → dispose → load")
    );
    let wasm_readme = readme.contains("close") || readme.contains("FR26");
    assert!(
        wasm_readme,
        "README must keep WASM instance close semantics"
    );
}

/// Story 1.2 AC#1: dispose hello 后加载另一路径 echo，新行为可用，旧 invoker 不可用。
#[test]
fn reload_different_path_hello_then_echo() {
    let ctx = Context::new();
    let hello = load_native_plugin(example_lib("hello_plugin")).expect("load hello");
    let hello_handle = ctx.plugin(hello).expect("install hello");
    ctx.start().expect("start");
    let old = ctx.get::<NativeInvoker>().expect("hello invoker").clone();
    assert_eq!(
        String::from_utf8_lossy(&old.call("greet", b"x").expect("hello greet")),
        "hello, x"
    );

    hello_handle.dispose().expect("dispose hello");
    assert!(old.call("greet", b"x").is_err());

    let echo = load_native_plugin(example_lib("echo_plugin")).expect("load echo");
    let _echo_handle = ctx.plugin(echo).expect("install echo after start");
    let invoker = ctx.get::<NativeInvoker>().expect("echo invoker");
    assert_eq!(invoker.call("echo", b"ping").expect("echo"), b"ping");
    assert!(
        invoker.call("greet", b"x").is_err(),
        "echo 插件不应再提供 hello 的 greet"
    );
}

/// Story 1.2 AC#2: 同路径覆盖新制品后再 load（Linux/macOS；Windows 文件锁则跳过覆盖）。
#[cfg(not(windows))]
#[test]
fn reload_same_path_after_replacing_artifact() {
    let tmp = std::env::temp_dir().join(format!(
        "plugctx-hotplug-{}-{}.so",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::copy(example_lib("hello_plugin"), &tmp).expect("copy hello to temp");

    let ctx = Context::new();
    let first = load_native_plugin(&tmp).expect("load temp hello");
    assert_eq!(first.name(), "hello");
    let handle = ctx.plugin(first).expect("install");
    ctx.start().expect("start");
    handle.dispose().expect("physical unload");

    std::fs::copy(example_lib("echo_plugin"), &tmp).expect("overwrite temp with echo");
    let second = load_native_plugin(&tmp).expect("load replaced artifact");
    assert_eq!(second.name(), "echo");
    let _h2 = ctx.plugin(second).expect("install echo");
    let invoker = ctx.get::<NativeInvoker>().expect("echo invoker");
    assert_eq!(invoker.call("echo", b"swap").expect("echo"), b"swap");

    let _ = std::fs::remove_file(&tmp);
}
