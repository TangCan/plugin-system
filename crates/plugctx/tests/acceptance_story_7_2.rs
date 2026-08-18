//! Acceptance tests for story 7.2 — 归还 / reset / 显式 destroy（FR44, FR45）。
//!
//! `cargo test -p plugctx --features dynamic-wasm --test acceptance_story_7_2`

#![cfg(feature = "dynamic-wasm")]

use std::time::Duration;

use plugctx::{bundled_echo_wasm, WasmInstancePool, WasmPoolConfig};

fn pool_with_max(n: usize) -> WasmInstancePool {
    WasmInstancePool::new(bundled_echo_wasm(), WasmPoolConfig { max_instances: n })
        .expect("create wasm instance pool")
}

/// AC#1 / FR44: 写入客人状态后 Drop 归还，再次 checkout 不可见前次状态。
#[test]
fn return_resets_guest_state_before_reuse() {
    let pool = pool_with_max(1);

    {
        let mut g = pool
            .checkout(Duration::from_secs(1))
            .expect("checkout")
            .expect("Some");
        g.call("set_state", b"tenant-a-secret").expect("set_state");
        assert_eq!(
            g.call("get_state", b"").expect("get"),
            b"tenant-a-secret",
            "state must be observable within same checkout"
        );
        assert_eq!(pool.live_count(), 1);
    }

    // Drop 归还：live_count 保持（槽位保留）
    assert_eq!(pool.live_count(), 1, "return keeps live slot");

    let mut g2 = pool
        .checkout(Duration::from_millis(500))
        .expect("after return")
        .expect("reuse slot");
    assert_eq!(
        g2.call("get_state", b"").expect("get after reset"),
        b"",
        "FR44: prior guest state must not leak across return"
    );
}

/// AC#2 / FR45: 显式 destroy 不得把实例静默放回；live_count 下降。
#[test]
fn destroy_does_not_return_instance_to_pool() {
    let pool = pool_with_max(1);

    let mut g = pool
        .checkout(Duration::from_secs(1))
        .expect("checkout")
        .expect("Some");
    g.call("set_state", b"poison").expect("set_state");
    let destroyed_id = g.plugin_id();
    assert_eq!(pool.live_count(), 1);

    g.destroy();
    assert_eq!(
        pool.live_count(),
        0,
        "FR45: destroy must free slot (not silent return)"
    );

    let mut g2 = pool
        .checkout(Duration::from_millis(500))
        .expect("checkout after destroy")
        .expect("new instance");
    assert_ne!(
        g2.plugin_id(),
        destroyed_id,
        "destroyed instance must not be checked out again"
    );
    assert_eq!(
        g2.call("get_state", b"").expect("clean"),
        b"",
        "new instance has no poisoned guest state"
    );
    assert_eq!(pool.live_count(), 1);
}

/// AC#3 / FR45: 归还 ≠ 销毁（live_count 语义对照）+ 文档锚点。
#[test]
fn return_is_not_destroy_live_count_semantics() {
    let pool = pool_with_max(2);

    // 路径 A：归还 — live_count 不变
    {
        let g = pool
            .checkout(Duration::from_secs(1))
            .expect("a")
            .expect("Some");
        assert_eq!(pool.live_count(), 1);
        drop(g);
    }
    assert_eq!(
        pool.live_count(),
        1,
        "return keeps instance counted as live/idle"
    );

    // 路径 B：销毁 — live_count 减一
    {
        let g = pool
            .checkout(Duration::from_secs(1))
            .expect("b")
            .expect("Some");
        assert_eq!(pool.live_count(), 1);
        g.destroy();
    }
    assert_eq!(
        pool.live_count(),
        0,
        "destroy decrements; return ≠ destroy (FR45)"
    );

    // 语义锚点：与 PluginHandle::dispose / WasmPlugin::close 对照写在 dynamic_wasm 模块文档。
    // 本测锁定池层可观测差异，防止把「归还」当成「逻辑卸载完成」。
    let _ = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/dynamic_wasm.rs"));
}
