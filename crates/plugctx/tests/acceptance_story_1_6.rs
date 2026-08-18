#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 1.6 — effect register / cancel / reverse cleanup (ATDD).
//!
//! Red phase: fail to compile or fail assertions until `effect`, `EffectHandle::cancel`,
//! and reverse-order cleanup on `dispose` exist.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use plugctx::{Context, EffectHandle};

/// AC#1: setup runs immediately; cleanup is registered (runs later on dispose).
#[test]
fn effect_setup_runs_immediately_cleanup_on_dispose() {
    let setup_ran = Rc::new(Cell::new(false));
    let cleanup_ran = Rc::new(Cell::new(false));
    let ctx = Context::new();

    let s = Rc::clone(&setup_ran);
    let c = Rc::clone(&cleanup_ran);
    let _handle: EffectHandle = ctx.effect(move || {
        s.set(true);
        move || {
            c.set(true);
        }
    });

    assert!(
        setup_ran.get(),
        "setup must run immediately when effect() is called"
    );
    assert!(!cleanup_ran.get(), "cleanup must not run until dispose");

    ctx.dispose();
    assert!(cleanup_ran.get(), "cleanup must run on dispose");
}

/// AC#2: effects registered A then B clean up as B then A on dispose.
#[test]
fn dispose_runs_cleanups_in_reverse_order() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let ctx = Context::new();

    let o_a = Rc::clone(&order);
    ctx.effect(move || {
        o_a.borrow_mut().push("setup-a");
        let o = Rc::clone(&o_a);
        move || {
            o.borrow_mut().push("cleanup-a");
        }
    });

    let o_b = Rc::clone(&order);
    ctx.effect(move || {
        o_b.borrow_mut().push("setup-b");
        let o = Rc::clone(&o_b);
        move || {
            o.borrow_mut().push("cleanup-b");
        }
    });

    assert_eq!(
        *order.borrow(),
        vec!["setup-a", "setup-b"],
        "setups run immediately in registration order"
    );

    ctx.dispose();
    assert_eq!(
        *order.borrow(),
        vec!["setup-a", "setup-b", "cleanup-b", "cleanup-a"],
        "cleanups must run reverse order B then A"
    );
}

/// AC#3: cancel removes cleanup so dispose does not run it.
#[test]
fn cancel_skips_cleanup_on_dispose() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let ctx = Context::new();

    let l1 = Rc::clone(&log);
    let handle = ctx.effect(move || {
        l1.borrow_mut().push("setup-cancel");
        let l = Rc::clone(&l1);
        move || {
            l.borrow_mut().push("cleanup-cancel");
        }
    });

    let l2 = Rc::clone(&log);
    ctx.effect(move || {
        l2.borrow_mut().push("setup-keep");
        let l = Rc::clone(&l2);
        move || {
            l.borrow_mut().push("cleanup-keep");
        }
    });

    handle.cancel();
    ctx.dispose();

    assert_eq!(
        *log.borrow(),
        vec!["setup-cancel", "setup-keep", "cleanup-keep"],
        "cancelled effect cleanup must not run; other cleanups still run"
    );
}

/// AC#4: second dispose does not re-run cleanups (idempotent).
#[test]
fn dispose_idempotent_cleanups_run_once() {
    let count = Rc::new(Cell::new(0u32));
    let ctx = Context::new();

    let c = Rc::clone(&count);
    ctx.effect(move || {
        move || {
            c.set(c.get() + 1);
        }
    });

    ctx.dispose();
    assert_eq!(count.get(), 1);
    ctx.dispose();
    assert_eq!(count.get(), 1, "second dispose must not re-run cleanups");
}

/// Guardrail: 后登记 cleanup（逆序先跑）可 cancel 先登记者，调用前标志跳过。
#[test]
fn cancel_earlier_effect_from_later_cleanup_during_dispose() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let ctx = Context::new();
    let handle_a_slot = Rc::new(RefCell::new(None::<EffectHandle>));

    let l_a = Rc::clone(&log);
    let handle_a = ctx.effect(move || {
        let l = Rc::clone(&l_a);
        move || {
            l.borrow_mut().push("cleanup-a");
        }
    });
    *handle_a_slot.borrow_mut() = Some(handle_a);

    let l_b = Rc::clone(&log);
    let slot = Rc::clone(&handle_a_slot);
    ctx.effect(move || {
        let l = Rc::clone(&l_b);
        let slot = Rc::clone(&slot);
        move || {
            l.borrow_mut().push("cleanup-b");
            if let Some(h) = slot.borrow_mut().take() {
                h.cancel();
            }
        }
    });

    ctx.dispose();
    assert_eq!(
        *log.borrow(),
        vec!["cleanup-b"],
        "B runs first and cancels A; A cleanup must be skipped"
    );
}

/// Guardrail: cleanup 内 provide / on / effect 不因 RefCell panic。
#[test]
fn cleanup_may_reenter_provide_on_effect_no_panic() {
    #[derive(Debug)]
    struct Flag(bool);

    struct Ping;

    let hit = Rc::new(Cell::new(false));
    let ctx = Context::new();
    let ctx_inner = ctx.clone();
    let h = Rc::clone(&hit);

    ctx.effect(move || {
        move || {
            ctx_inner.provide(Flag(true));
            ctx_inner.on(move |_: &Ping| {});
            // 已 disposed：setup 仍跑，cleanup 不入队
            ctx_inner.effect(move || {
                h.set(true);
                || {}
            });
        }
    });

    ctx.dispose();
    assert!(hit.get(), "effect setup during cleanup must still run");
    assert!(
        ctx.get::<Flag>().map(|f| f.0) == Some(true),
        "provide during cleanup must succeed"
    );
}

/// NFR1: still no async runtime in direct dependencies.
#[test]
fn no_async_runtime_in_direct_dependencies() {
    use std::fs;
    use std::path::PathBuf;

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let text = fs::read_to_string(&manifest).expect("read plugctx Cargo.toml");
    let forbidden = ["tokio", "async-std", "smol"];
    for name in forbidden {
        assert!(
            !dependency_table_mentions(&text, name),
            "plugctx must not depend on async runtime crate `{name}` (NFR1)"
        );
    }
}

fn dependency_table_mentions(cargo_toml: &str, crate_name: &str) -> bool {
    let mut in_deps = false;
    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]"
                || trimmed == "[dev-dependencies]"
                || trimmed == "[build-dependencies]";
            continue;
        }
        if in_deps && trimmed.starts_with(crate_name) {
            let rest = &trimmed[crate_name.len()..];
            if rest.is_empty() || rest.starts_with([' ', '\t', '=', '.']) {
                return true;
            }
        }
    }
    false
}
