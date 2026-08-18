//! CLI 场景：加载 native 插件、调用、物理卸载后再 call 失败。
//!
//! ```bash
//! cargo build -p hello_plugin
//! cargo run -p plugctx-examples --example cli-hotplug --features native
//! ```

use std::path::PathBuf;

use plugctx::{load_native_plugin, Context, NativeInvoker};

fn example_lib(name: &str) -> PathBuf {
    let mut candidates = Vec::new();
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(dir));
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target"));
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

fn main() {
    let plugin = load_native_plugin(example_lib("hello_plugin")).expect("load hello_plugin");
    let ctx = Context::new();
    let handle = ctx.plugin(plugin).expect("install");
    ctx.start().expect("start");

    let invoker = ctx.get::<NativeInvoker>().expect("NativeInvoker").clone();
    let out = invoker.call("greet", b"cli").expect("greet");
    println!("{}", String::from_utf8_lossy(&out));

    handle.dispose().expect("physical unload");
    match invoker.call("greet", b"cli") {
        Err(e) => println!("after dispose: {e}"),
        Ok(bytes) => panic!("stale invoker must not succeed: {bytes:?}"),
    }
}
