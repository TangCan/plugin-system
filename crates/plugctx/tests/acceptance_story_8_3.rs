//! Acceptance tests for story 8.3 — 一 Store 一实例销毁与 ATDD（FR49）。
//!
//! ```bash
//! cargo test -p plugctx --features dynamic-wasm-component --test acceptance_story_8_3
//! ```

#![cfg(feature = "dynamic-wasm-component")]

use plugctx::{bundled_component_add_wat, load_wasm_component, ComponentInvoker, Context, Error};

/// AC#1 / AC#2：未 Drop 时 Store 存活、可调用；Drop 后探针递增且再次调用失败。
#[test]
fn before_drop_usable_after_drop_unusable() {
    let plugin = load_wasm_component(bundled_component_add_wat().as_bytes()).expect("load");
    assert_eq!(plugin.store_drop_count(), 0, "未 Drop 时探针为 0");
    assert_eq!(plugin.call_add(40, 2).expect("before drop"), 42);

    plugin.close();

    assert!(
        plugin.store_drop_count() >= 1,
        "Drop Store 后探针须可观测，got {}",
        plugin.store_drop_count()
    );
    assert!(plugin.is_closed());
    assert!(
        matches!(plugin.call_add(1, 1), Err(Error::WasmClosed { .. })),
        "Drop 后再次调用须失败"
    );
}

/// AC#1：Context dispose Effect 路径同样 Drop Store（与显式 close 对齐）。
#[test]
fn context_dispose_drops_store() {
    let plugin = load_wasm_component(bundled_component_add_wat().as_bytes()).expect("load");
    let ctx = Context::new();
    let handle = ctx.plugin(plugin).expect("install");
    ctx.start().expect("start");

    // Clone 出句柄后再 dispose，避免 ServiceRef 借住 Context。
    let invoker = ctx.get::<ComponentInvoker>().expect("invoker").clone();
    assert_eq!(invoker.store_drop_count(), 0);
    assert_eq!(invoker.call_add(7, 8).expect("before dispose"), 15);

    handle.dispose().expect("dispose");

    assert!(ctx.get::<ComponentInvoker>().is_none());
    assert!(
        invoker.store_drop_count() >= 1,
        "dispose Effect 须 Drop Store，count={}",
        invoker.store_drop_count()
    );
    assert!(matches!(
        invoker.call_add(0, 0),
        Err(Error::WasmClosed { .. })
    ));
}

/// AC#2：close 幂等 — 仅一次 Store Drop。
#[test]
fn store_drop_is_idempotent() {
    let plugin = load_wasm_component(bundled_component_add_wat().as_bytes()).expect("load");
    plugin.close();
    plugin.close();
    plugin.close();
    assert_eq!(plugin.store_drop_count(), 1);
}
