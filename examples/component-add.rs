//! Component Model 宿主加载内置 wasip2 WIT 样例（feature `component`）
//!
//! ```bash
//! cargo run -p plugctx-examples --example component-add --features component
//! ```

use plugctx::{bundled_wit_sample_add_wasm, load_wasm_component};

fn main() {
    println!("=== plugctx-examples component-add ===\n");

    let plugin = load_wasm_component(bundled_wit_sample_add_wasm()).expect("load WIT sample");
    let sum = plugin.call_add(40, 2).expect("add");
    println!("add(40, 2) = {sum}");
    assert_eq!(sum, 42);

    println!("\n=== done ===");
}
