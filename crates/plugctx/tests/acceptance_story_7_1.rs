//! Acceptance tests for story 7.1 — Extism Pool 封装与有界 checkout（FR43, NFR11, NFR14）。
//!
//! `cargo test -p plugctx --features dynamic-wasm --test acceptance_story_7_1`

#![cfg(feature = "dynamic-wasm")]

use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use plugctx::{bundled_echo_wasm, WasmInstancePool, WasmPoolConfig};

fn pool_with_max(n: usize) -> WasmInstancePool {
    WasmInstancePool::new(bundled_echo_wasm(), WasmPoolConfig { max_instances: n })
        .expect("create wasm instance pool")
}

/// AC#1: max_instances = N 时，并发 checkout ≤ N 均可获得可用实例。
#[test]
fn concurrent_checkout_within_capacity_succeeds() {
    const N: usize = 3;
    let pool = Arc::new(pool_with_max(N));
    let barrier = Arc::new(Barrier::new(N));
    let mut handles = Vec::with_capacity(N);

    for i in 0..N {
        let pool = Arc::clone(&pool);
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier.wait();
            let mut guard = pool
                .checkout(Duration::from_secs(2))
                .expect("checkout result")
                .unwrap_or_else(|| panic!("thread {i}: expected Some within capacity"));
            let out = guard
                .call("echo", format!("t{i}").as_bytes())
                .expect("echo");
            assert_eq!(out, format!("t{i}").into_bytes());
            // 持有 guard 直到所有线程都借到，验证并发占用 ≤ N 仍成功
            barrier.wait();
            drop(guard);
        }));
    }

    for h in handles {
        h.join().expect("worker");
    }
    assert!(
        pool.live_count() <= N,
        "live instances must stay bounded by max_instances"
    );
}

/// AC#2: 池已满时超时 checkout 返回 None，且不得无限新建（count ≤ N）。
#[test]
fn full_pool_checkout_times_out_without_growing() {
    const N: usize = 2;
    let pool = pool_with_max(N);

    let g1 = pool
        .checkout(Duration::from_millis(500))
        .expect("g1")
        .expect("first checkout");
    let g2 = pool
        .checkout(Duration::from_millis(500))
        .expect("g2")
        .expect("second checkout");
    assert_eq!(pool.live_count(), N);
    assert_eq!(pool.max_instances(), N);

    let timed_out = pool
        .checkout(Duration::from_millis(80))
        .expect("timeout checkout should be Ok");
    assert!(
        timed_out.is_none(),
        "full pool must return None on timeout (FR43), got Some"
    );
    assert!(
        pool.live_count() <= N,
        "must not grow beyond max_instances; live={}",
        pool.live_count()
    );

    drop(g1);
    drop(g2);

    // 归还后应可再次借出（7.1 依赖 Extism Drop 归还；reset 细节属 7.2）
    let mut g3 = pool
        .checkout(Duration::from_millis(500))
        .expect("after return")
        .expect("checkout after return must succeed");
    assert_eq!(g3.call("echo", b"again").expect("echo"), b"again");
}

/// AC#3 / NFR14: 默认 features 依赖图不强制拉入 extism。
#[test]
fn default_features_dependency_graph_excludes_extism() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let output = std::process::Command::new("cargo")
        .args([
            "tree",
            "-p",
            "plugctx",
            "-e",
            "normal",
            "--manifest-path",
            &format!("{manifest_dir}/Cargo.toml"),
        ])
        // 不加 --features：验证默认依赖图
        .output()
        .expect("run cargo tree");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.lines().any(|l| l.contains("extism")),
        "default cargo tree must not include extism (NFR14):\n{stdout}"
    );
}
