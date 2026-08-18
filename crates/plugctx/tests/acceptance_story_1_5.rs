#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 1.5 — typed on/emit with reentrancy (ATDD).
//!
//! Red phase: fail to compile or fail assertions until `on`/`emit`,
//! `EventListenerHandle::cancel`, ordered dispatch, and reentrancy-safe emit exist.

use std::cell::RefCell;
use std::rc::Rc;

use plugctx::{Context, EventListenerHandle};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Ping(u32);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Pong(String);

/// AC#1: multiple `on` then `emit` invokes listeners in registration order once each.
#[test]
fn emit_invokes_listeners_in_registration_order() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let ctx = Context::new();

    let o1 = Rc::clone(&order);
    ctx.on(move |e: &Ping| o1.borrow_mut().push(('a', e.0)));
    let o2 = Rc::clone(&order);
    ctx.on(move |e: &Ping| o2.borrow_mut().push(('b', e.0)));
    let o3 = Rc::clone(&order);
    ctx.on(move |e: &Ping| o3.borrow_mut().push(('c', e.0)));

    ctx.emit(&Ping(7));

    assert_eq!(
        *order.borrow(),
        vec![('a', 7), ('b', 7), ('c', 7)],
        "listeners must run once each in registration order"
    );
}

/// AC#2: cancel handle removes listener from subsequent emits.
#[test]
fn cancel_handle_stops_subsequent_emits() {
    let hits = Rc::new(RefCell::new(0u32));
    let ctx = Context::new();

    let h1 = Rc::clone(&hits);
    let handle: EventListenerHandle = ctx.on(move |_: &Ping| {
        *h1.borrow_mut() += 1;
    });
    let h2 = Rc::clone(&hits);
    ctx.on(move |_: &Ping| {
        *h2.borrow_mut() += 10;
    });

    ctx.emit(&Ping(1));
    assert_eq!(*hits.borrow(), 11);

    handle.cancel();
    ctx.emit(&Ping(2));
    assert_eq!(
        *hits.borrow(),
        21,
        "cancelled listener must not run; other listeners still run"
    );
}

/// AC#3: listener may reenter `on` / `emit` / `provide` without RefCell panic.
#[test]
fn emit_reentrancy_on_emit_provide_no_panic() {
    #[derive(Debug)]
    struct Flag(bool);

    let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    let ctx = Context::new();

    let log_outer = Rc::clone(&log);
    let ctx_outer = ctx.clone();
    ctx.on(move |e: &Ping| {
        if e.0 != 1 {
            // Nested same-type emit would only reach here if this listener were
            // re-entered while idle; currently-running listeners are skipped.
            log_outer.borrow_mut().push("outer-nested");
            return;
        }
        log_outer.borrow_mut().push("outer");

        // Reenter: provide service while outer emit is in progress.
        ctx_outer.provide(Flag(true));

        // Reenter: register another listener mid-emit (must not panic).
        let log_late = Rc::clone(&log_outer);
        ctx_outer.on(move |p: &Ping| {
            if p.0 == 2 {
                log_late.borrow_mut().push("late-registered");
            }
        });

        // Reenter: emit a different event type.
        let log_pong = Rc::clone(&log_outer);
        ctx_outer.on(move |p: &Pong| {
            log_pong.borrow_mut().push("pong");
            assert_eq!(p.0, "hi");
        });
        ctx_outer.emit(&Pong("hi".into()));

        // Nested same-type emit — must not BorrowMut-panic.
        // Outer is still running so it is skipped; late-registered runs.
        ctx_outer.emit(&Ping(2));
    });

    ctx.emit(&Ping(1));

    assert!(
        ctx.get::<Flag>().map(|f| f.0) == Some(true),
        "provide during emit must succeed"
    );
    let entries = log.borrow().clone();
    assert_eq!(
        entries,
        vec!["outer", "pong", "late-registered"],
        "reentrant on/emit/provide must run without panic; same-listener skip on nested emit"
    );
}

/// AC#4: emit with no listeners is a successful no-op.
#[test]
fn emit_with_no_listeners_is_noop() {
    let ctx = Context::new();
    ctx.emit(&Ping(0));
    ctx.emit(&Pong("nobody".into()));
}

/// Guardrail: cancel another listener mid-emit; cancelled flag skips remaining snapshot call.
#[test]
fn cancel_sibling_during_emit_skips_rest_of_snapshot() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let ctx = Context::new();

    let log_a = Rc::clone(&log);
    let handle_slot = Rc::new(RefCell::new(None::<EventListenerHandle>));
    let handle_slot_outer = Rc::clone(&handle_slot);

    ctx.on(move |_: &Ping| {
        log_a.borrow_mut().push('a');
        if let Some(h) = handle_slot_outer.borrow_mut().take() {
            h.cancel();
        }
    });

    let log_b = Rc::clone(&log);
    let handle_b = ctx.on(move |_: &Ping| {
        log_b.borrow_mut().push('b');
    });
    *handle_slot.borrow_mut() = Some(handle_b);

    ctx.emit(&Ping(1));
    assert_eq!(
        *log.borrow(),
        vec!['a'],
        "b must be skipped after cancel during same emit snapshot"
    );

    log.borrow_mut().clear();
    ctx.emit(&Ping(2));
    assert_eq!(
        *log.borrow(),
        vec!['a'],
        "b remains cancelled on later emit"
    );
}

/// Guardrail: FR9 — `plugin` during emit must not RefCell-panic.
#[test]
fn plugin_during_emit_no_panic() {
    use plugctx::{Error, Plugin};

    struct Marker;
    impl Plugin for Marker {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            Ok(())
        }
    }

    let installed = Rc::new(RefCell::new(false));
    let ctx = Context::new();
    let flag = Rc::clone(&installed);
    let ctx_inner = ctx.clone();
    ctx.on(move |_: &Ping| {
        ctx_inner.plugin(Marker).expect("plugin during emit");
        *flag.borrow_mut() = true;
    });
    ctx.emit(&Ping(0));
    assert!(*installed.borrow());
}

/// Guardrail: ReadyEvent after start is delivered via real emit path.
#[test]
fn ready_event_delivered_on_start() {
    use plugctx::ReadyEvent;

    let hit = Rc::new(RefCell::new(false));
    let ctx = Context::new();
    let h = Rc::clone(&hit);
    ctx.on(move |_: &ReadyEvent| {
        *h.borrow_mut() = true;
    });
    ctx.start().expect("start empty");
    assert!(*hit.borrow(), "ReadyEvent listener must run on start");
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
