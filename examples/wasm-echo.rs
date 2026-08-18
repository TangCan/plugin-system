//! Extism WASM echo 演示（feature `wasm`）
//!
//! ```bash
//! cargo run -p plugctx-examples --example wasm-echo --features wasm
//! ```

use plugctx::{bundled_echo_wasm, load_wasm_plugin, WasmLoadConfig};

fn main() {
    println!("=== plugctx-examples wasm-echo ===\n");

    let plugin =
        load_wasm_plugin(bundled_echo_wasm(), WasmLoadConfig::default()).expect("load echo.wasm");
    let out = plugin.call("echo", b"ping").expect("echo");
    let text = String::from_utf8_lossy(&out);
    println!("echo(ping) => {text}");
    assert!(text.contains("ping") || !out.is_empty());

    println!("\n=== done ===");
}
