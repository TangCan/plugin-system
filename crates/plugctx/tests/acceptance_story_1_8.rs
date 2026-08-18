#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 1.8 — built-in ReadyEvent / DisposeEvent (ATDD).
//!
//! Red phase: fail to compile or fail assertions until `DisposeEvent` exists,
//! `dispose` emits it before effect cleanups, and ReadyEvent stays once-on-success.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use plugctx::{Context, DisposeEvent, Error, Plugin, ReadyEvent};

/// AC#1: successful start emits ReadyEvent exactly once.
#[test]
fn start_emits_ready_event_exactly_once() {
    let hits = Rc::new(Cell::new(0u32));
    let ctx = Context::new();

    let h = Rc::clone(&hits);
    ctx.on(move |_: &ReadyEvent| {
        h.set(h.get() + 1);
    });

    ctx.start().expect("start empty");
    assert_eq!(
        hits.get(),
        1,
        "ReadyEvent must fire exactly once on success"
    );

    let err = ctx.start().expect_err("second start must fail");
    assert!(matches!(err, Error::AlreadyStarted));
    assert_eq!(
        hits.get(),
        1,
        "failed second start must not emit ReadyEvent again"
    );
}

/// AC#1: failed start does not emit ReadyEvent.
#[test]
fn failed_start_does_not_emit_ready_event() {
    struct NeedsMissing;

    impl Plugin for NeedsMissing {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            Ok(())
        }

        fn dependencies(&self) -> Vec<std::any::TypeId> {
            vec![std::any::TypeId::of::<u64>()]
        }
    }

    let hits = Rc::new(Cell::new(0u32));
    let ctx = Context::new();
    let h = Rc::clone(&hits);
    ctx.on(move |_: &ReadyEvent| {
        h.set(h.get() + 1);
    });

    ctx.plugin(NeedsMissing).expect("install delayed");
    let err = ctx.start().expect_err("missing dependency must fail start");
    assert!(matches!(err, Error::MissingDependency));
    assert_eq!(hits.get(), 0, "ReadyEvent must not fire when start fails");
    assert!(!ctx.is_started());
}

/// AC#2: DisposeEvent fires before reverse effect cleanups.
#[test]
fn dispose_emits_dispose_event_before_effect_cleanups() {
    let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    let ctx = Context::new();

    let o_ev = Rc::clone(&order);
    ctx.on(move |_: &DisposeEvent| {
        o_ev.borrow_mut().push("dispose-event");
    });

    let o_fx = Rc::clone(&order);
    ctx.effect(move || {
        let o = Rc::clone(&o_fx);
        move || {
            o.borrow_mut().push("effect-cleanup");
        }
    });

    ctx.dispose();
    assert_eq!(
        *order.borrow(),
        vec!["dispose-event", "effect-cleanup"],
        "DisposeEvent must run before effect cleanups"
    );
}

/// AC#2: DisposeEvent listeners cloned — reentrant dispose / on / emit do not panic.
#[test]
fn dispose_event_reentrancy_no_panic() {
    let hits = Rc::new(Cell::new(0u32));
    let ctx = Context::new();

    let h = Rc::clone(&hits);
    let ctx_inner = ctx.clone();
    ctx.on(move |_: &DisposeEvent| {
        h.set(h.get() + 1);
        // Re-enter dispose (must be idempotent) and touch event/service APIs.
        ctx_inner.dispose();
        ctx_inner.on(|_: &DisposeEvent| {});
        ctx_inner.emit(&DisposeEvent);
        ctx_inner.provide(42u32);
    });

    ctx.dispose();
    assert_eq!(
        hits.get(),
        1,
        "outer DisposeEvent must run once despite reentrant dispose"
    );
    assert!(ctx.is_disposed());

    // Second dispose is no-op (no second outer fire).
    ctx.dispose();
    assert_eq!(hits.get(), 1);
}

/// AC#2: dispose without start still emits DisposeEvent.
#[test]
fn dispose_without_start_still_emits_dispose_event() {
    let hit = Rc::new(Cell::new(false));
    let ctx = Context::new();
    let h = Rc::clone(&hit);
    ctx.on(move |_: &DisposeEvent| {
        h.set(true);
    });
    ctx.dispose();
    assert!(hit.get(), "Created→Disposing must still emit DisposeEvent");
}

/// AC#2: cascaded child dispose emits its own DisposeEvent.
#[test]
fn cascade_dispose_emits_child_dispose_event() {
    let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    let parent = Context::new();
    let child = parent.isolate().expect("isolate");

    let lp = Rc::clone(&log);
    parent.on(move |_: &DisposeEvent| {
        lp.borrow_mut().push("parent");
    });
    let lc = Rc::clone(&log);
    child.on(move |_: &DisposeEvent| {
        lc.borrow_mut().push("child");
    });

    parent.dispose();
    assert_eq!(
        *log.borrow(),
        vec!["parent", "child"],
        "parent DisposeEvent then child DisposeEvent on cascade"
    );
}

/// AC#3 / NFR: types are public; no async runtime in direct deps.
#[test]
fn lifecycle_event_types_exported_and_no_async_runtime() {
    let _r = ReadyEvent;
    let _d = DisposeEvent;
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    assert!(
        !manifest.contains("tokio")
            && !manifest.contains("async-std")
            && !manifest.contains("smol"),
        "core must stay sync-only"
    );
}

/// Automate: DisposeEvent listeners run in registration order.
#[test]
fn dispose_event_listeners_run_in_registration_order() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let ctx = Context::new();

    let o1 = Rc::clone(&order);
    ctx.on(move |_: &DisposeEvent| o1.borrow_mut().push('a'));
    let o2 = Rc::clone(&order);
    ctx.on(move |_: &DisposeEvent| o2.borrow_mut().push('b'));

    ctx.dispose();
    assert_eq!(*order.borrow(), vec!['a', 'b']);
}

/// Automate: effect registered inside DisposeEvent still runs (disposing ≠ disposed).
#[test]
fn effect_registered_during_dispose_event_still_runs() {
    let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    let ctx = Context::new();

    let o_ev = Rc::clone(&order);
    let ctx_inner = ctx.clone();
    ctx.on(move |_: &DisposeEvent| {
        o_ev.borrow_mut().push("dispose-event");
        let o_fx = Rc::clone(&o_ev);
        ctx_inner.effect(move || {
            let o = Rc::clone(&o_fx);
            move || {
                o.borrow_mut().push("late-effect");
            }
        });
    });

    ctx.dispose();
    assert_eq!(
        *order.borrow(),
        vec!["dispose-event", "late-effect"],
        "effect registered during DisposeEvent must be taken and run"
    );
}

/// Automate: ReadyEvent fires after plugins built (listener can observe provided service).
#[test]
fn ready_event_sees_services_from_built_plugins() {
    struct ProvidesFlag;

    impl Plugin for ProvidesFlag {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(true);
            Ok(())
        }
    }

    let saw = Rc::new(Cell::new(false));
    let ctx = Context::new();
    ctx.plugin(ProvidesFlag).expect("install");
    let s = Rc::clone(&saw);
    let ctx_l = ctx.clone();
    ctx.on(move |_: &ReadyEvent| {
        s.set(ctx_l.get::<bool>().map(|v| *v).unwrap_or(false));
    });
    ctx.start().expect("start");
    assert!(saw.get(), "ReadyEvent must run after plugin build/provide");
}
