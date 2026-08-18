#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 2.2 — PluginHandle::dispose 精确卸载（ATDD）。
//!
//! Red phase: 在按 PluginScope 回滚落地前，服务/监听/effect 在「仅删条目」后仍残留，
//! 或二次 dispose / 子上下文 / 根级保留等断言失败。

use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use plugctx::{Context, Error, Plugin};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SvcA(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SvcB(u32);

struct EvA;
struct EvB;

/// AC#1: 仅 dispose A 时，A 的服务/监听/effect 撤销，B 仍可用。
#[test]
fn dispose_a_rolls_back_scope_without_touching_b() {
    let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));

    struct PluginA {
        log: Rc<RefCell<Vec<&'static str>>>,
    }
    impl Plugin for PluginA {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SvcA(1));
            let log = Rc::clone(&self.log);
            let _ = ctx.on::<EvA>(move |_e| {
                log.borrow_mut().push("a-listener");
            });
            let log = Rc::clone(&self.log);
            let _ = ctx.effect(move || {
                let log = Rc::clone(&log);
                move || log.borrow_mut().push("a-cleanup")
            });
            Ok(())
        }
    }

    struct PluginB {
        log: Rc<RefCell<Vec<&'static str>>>,
    }
    impl Plugin for PluginB {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SvcB(2));
            let log = Rc::clone(&self.log);
            let _ = ctx.on::<EvB>(move |_e| {
                log.borrow_mut().push("b-listener");
            });
            let log = Rc::clone(&self.log);
            let _ = ctx.effect(move || {
                let log = Rc::clone(&log);
                move || log.borrow_mut().push("b-cleanup")
            });
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle_a = ctx
        .plugin(PluginA {
            log: Rc::clone(&log),
        })
        .expect("install a");
    let handle_b = ctx
        .plugin(PluginB {
            log: Rc::clone(&log),
        })
        .expect("install b");
    ctx.start().expect("start");

    assert!(ctx.get::<SvcA>().is_some());
    assert!(ctx.get::<SvcB>().is_some());

    handle_a.dispose().expect("dispose a");

    assert!(
        ctx.get::<SvcA>().is_none(),
        "A service must be removed after dispose A"
    );
    assert!(
        ctx.get::<SvcB>().is_some(),
        "B service must remain after dispose A"
    );
    assert!(handle_b.is_alive(), "B handle must stay alive");
    assert!(
        log.borrow().iter().any(|s| *s == "a-cleanup"),
        "A effect cleanup must run on plugin dispose"
    );
    assert!(
        !log.borrow().iter().any(|s| *s == "b-cleanup"),
        "B effect must not run when only A is disposed"
    );

    log.borrow_mut().clear();
    ctx.emit(&EvA);
    ctx.emit(&EvB);
    assert!(
        !log.borrow().iter().any(|s| *s == "a-listener"),
        "A listener must be gone"
    );
    assert!(
        log.borrow().iter().any(|s| *s == "b-listener"),
        "B listener must still fire"
    );

    // B 的 effect 应在 Context::dispose 时仍会执行
    log.borrow_mut().clear();
    ctx.dispose();
    assert!(
        log.borrow().iter().any(|s| *s == "b-cleanup"),
        "B cleanup must still run on context dispose"
    );
}

/// AC#2: 二次 dispose 幂等或 PluginAlreadyDisposed，且不误删 B。
#[test]
fn second_dispose_is_safe_and_does_not_touch_b() {
    struct PluginA;
    impl Plugin for PluginA {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SvcA(1));
            Ok(())
        }
    }
    struct PluginB;
    impl Plugin for PluginB {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SvcB(2));
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle_a = ctx.plugin(PluginA).expect("a");
    let handle_b = ctx.plugin(PluginB).expect("b");
    ctx.start().expect("start");

    handle_a.dispose().expect("first dispose");
    let second = handle_a.dispose();
    assert!(
        second.is_ok() || second == Err(Error::PluginAlreadyDisposed),
        "second dispose must be idempotent or PluginAlreadyDisposed, got {second:?}"
    );

    assert!(
        ctx.get::<SvcB>().is_some(),
        "B must survive double dispose of A"
    );
    assert!(handle_b.is_alive());
}

/// AC#3: 插件 build 中 isolate 的子 Context 随插件 dispose 销毁。
#[test]
fn dispose_plugin_destroys_scoped_child_contexts() {
    let child_disposed = Rc::new(Cell::new(false));
    let child_hold = Rc::new(RefCell::new(None::<Context>));

    struct PluginWithChild {
        flag: Rc<Cell<bool>>,
        hold: Rc<RefCell<Option<Context>>>,
    }
    impl Plugin for PluginWithChild {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            let child = ctx.isolate().expect("isolate");
            let flag = Rc::clone(&self.flag);
            let _ = child.effect(move || {
                let flag = Rc::clone(&flag);
                move || flag.set(true)
            });
            *self.hold.borrow_mut() = Some(child);
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle = ctx
        .plugin(PluginWithChild {
            flag: Rc::clone(&child_disposed),
            hold: Rc::clone(&child_hold),
        })
        .expect("install");
    ctx.start().expect("start");

    assert!(
        child_hold
            .borrow()
            .as_ref()
            .is_some_and(|c| !c.is_disposed()),
        "child must exist after build"
    );

    handle.dispose().expect("dispose plugin");

    assert!(
        child_disposed.get(),
        "scoped child effect cleanup must run when plugin disposes"
    );
    assert!(
        child_hold
            .borrow()
            .as_ref()
            .is_some_and(|c| c.is_disposed()),
        "scoped child context must be disposed"
    );
}

/// AC#4: 根级注册在 dispose 某插件后仍保留。
#[test]
fn root_level_registrations_survive_plugin_dispose() {
    let root_hits = Rc::new(Cell::new(0u32));
    let root_cleanup = Rc::new(Cell::new(false));

    struct EmptyPlugin;
    impl Plugin for EmptyPlugin {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SvcA(9));
            let _ = ctx.on::<EvA>(|_e| {});
            let _ = ctx.effect(|| || {});
            Ok(())
        }
    }

    let ctx = Context::new();
    ctx.provide(SvcB(7));
    let hits = Rc::clone(&root_hits);
    let _ = ctx.on::<EvB>(move |_e| {
        hits.set(hits.get() + 1);
    });
    let flag = Rc::clone(&root_cleanup);
    let _ = ctx.effect(move || {
        let flag = Rc::clone(&flag);
        move || flag.set(true)
    });

    let handle = ctx.plugin(EmptyPlugin).expect("install");
    ctx.start().expect("start");
    handle.dispose().expect("dispose plugin");

    assert_eq!(
        *ctx.get::<SvcB>().expect("root SvcB"),
        SvcB(7),
        "root provide must remain"
    );
    ctx.emit(&EvB);
    assert_eq!(root_hits.get(), 1, "root listener must remain");
    assert!(
        ctx.get::<SvcA>().is_none(),
        "plugin service must be removed"
    );
    assert!(
        !root_cleanup.get(),
        "root effect must not run on plugin dispose"
    );

    ctx.dispose();
    assert!(
        root_cleanup.get(),
        "root effect cleanup must run on context dispose"
    );
}

/// 未构建（延迟安装后未 start）dispose：仅移除条目，不 panic。
#[test]
fn dispose_unbuilt_plugin_removes_entry_only() {
    struct NeverBuilt;
    impl Plugin for NeverBuilt {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            unreachable!("must not build")
        }
    }

    let ctx = Context::new();
    let handle = ctx.plugin(NeverBuilt).expect("install delayed");
    assert!(handle.scope().is_none());
    let id = handle.id();
    handle.dispose().expect("dispose unbuilt");
    assert!(!ctx.contains_plugin(id));
}

/// 守卫：公开 API 暴露 PluginAlreadyDisposed（或二次 dispose 成功幂等）。
#[test]
fn plugin_already_disposed_error_is_distinguishable() {
    // 编译期钉死变体存在；运行时用 Debug/PartialEq
    let err = Error::PluginAlreadyDisposed;
    assert_eq!(err, Error::PluginAlreadyDisposed);
    let _ = TypeId::of::<Error>();
}

/// Automate: cleanup 重入再次 dispose 同一句柄不得 panic，且兄弟插件保留。
#[test]
fn dispose_reentrant_from_effect_cleanup_is_safe() {
    struct PluginA {
        handle_slot: Rc<RefCell<Option<plugctx::PluginHandle>>>,
    }
    impl Plugin for PluginA {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SvcA(1));
            let slot = Rc::clone(&self.handle_slot);
            let _ = ctx.effect(move || {
                let slot = Rc::clone(&slot);
                move || {
                    if let Some(h) = slot.borrow().as_ref() {
                        let _ = h.dispose();
                    }
                }
            });
            Ok(())
        }
    }
    struct PluginB;
    impl Plugin for PluginB {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SvcB(2));
            Ok(())
        }
    }

    let ctx = Context::new();
    let slot = Rc::new(RefCell::new(None));
    let handle_a = ctx
        .plugin(PluginA {
            handle_slot: Rc::clone(&slot),
        })
        .expect("a");
    let handle_b = ctx.plugin(PluginB).expect("b");
    *slot.borrow_mut() = Some(handle_a.clone());
    ctx.start().expect("start");

    handle_a.dispose().expect("outer dispose");
    assert!(
        ctx.get::<SvcB>().is_some(),
        "B must remain after reentrant dispose of A"
    );
    assert!(handle_b.is_alive());
    assert_eq!(
        handle_a.dispose(),
        Err(Error::PluginAlreadyDisposed),
        "outer handle second call still PluginAlreadyDisposed"
    );
}

/// Automate: 同事件类型多监听器时，dispose 前装插件后后装监听仍触发。
#[test]
fn dispose_earlier_listener_keeps_later_same_event_listener() {
    let hits = Rc::new(Cell::new(0u32));

    struct PluginEarly;
    impl Plugin for PluginEarly {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            let _ = ctx.on::<EvA>(|_e| {});
            Ok(())
        }
    }
    struct PluginLate {
        hits: Rc<Cell<u32>>,
    }
    impl Plugin for PluginLate {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            let hits = Rc::clone(&self.hits);
            let _ = ctx.on::<EvA>(move |_e| {
                hits.set(hits.get() + 1);
            });
            Ok(())
        }
    }

    let ctx = Context::new();
    let early = ctx.plugin(PluginEarly).expect("early");
    let _late = ctx
        .plugin(PluginLate {
            hits: Rc::clone(&hits),
        })
        .expect("late");
    ctx.start().expect("start");

    early.dispose().expect("dispose early");
    ctx.emit(&EvA);
    assert_eq!(hits.get(), 1, "later same-event listener must remain");
}

/// Automate: children_start 与 effects_start 类似，根级 isolate 之后插件区间起点正确。
#[test]
fn children_start_accounts_for_prior_root_isolates() {
    struct PluginWithChild;
    impl Plugin for PluginWithChild {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            let _hold = ctx.isolate().expect("isolate");
            Ok(())
        }
    }

    let ctx = Context::new();
    let _root_child = ctx.isolate().expect("isolate");
    let handle = ctx.plugin(PluginWithChild).expect("install");
    ctx.start().expect("start");
    let scope = handle.scope().expect("scope");
    assert_eq!(scope.children_start, 1);
    assert_eq!(scope.children_count, 1);
    handle.dispose().expect("dispose");
}

/// Retro item-5: Context 已 dispose 后 PluginHandle::dispose → AlreadyDisposed（非 PluginAlreadyDisposed）。
#[test]
fn plugin_handle_dispose_after_context_dispose_is_already_disposed() {
    struct Tiny;
    impl Plugin for Tiny {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SvcA(1));
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle = ctx.plugin(Tiny).expect("install");
    ctx.start().expect("start");
    ctx.dispose();
    assert!(ctx.is_disposed());
    let err = handle
        .dispose()
        .expect_err("must fail after context dispose");
    assert!(
        matches!(err, Error::AlreadyDisposed),
        "context-level dispose must surface AlreadyDisposed, got {err:?}"
    );
    assert_ne!(
        err,
        Error::PluginAlreadyDisposed,
        "must distinguish context-destroyed from plugin-already-unloaded"
    );
}

/// Retro item-6: 根级 isolate 已 dispose（死弱引用）后再卸带 children 的插件，子树仍正确销毁。
#[test]
fn dispose_plugin_after_dead_root_isolate_still_disposes_scoped_children() {
    let child_a_flag = Rc::new(Cell::new(false));
    let child_b_flag = Rc::new(Cell::new(false));
    let hold_a = Rc::new(RefCell::new(None::<Context>));
    let hold_b = Rc::new(RefCell::new(None::<Context>));

    struct PluginChild {
        flag: Rc<Cell<bool>>,
        hold: Rc<RefCell<Option<Context>>>,
    }
    impl Plugin for PluginChild {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            let child = ctx.isolate().expect("isolate");
            let flag = Rc::clone(&self.flag);
            let _ = child.effect(move || {
                let flag = Rc::clone(&flag);
                move || flag.set(true)
            });
            *self.hold.borrow_mut() = Some(child);
            Ok(())
        }
    }

    let ctx = Context::new();
    let root_iso = ctx.isolate().expect("root isolate");
    root_iso.dispose();
    assert!(root_iso.is_disposed());

    let handle_a = ctx
        .plugin(PluginChild {
            flag: Rc::clone(&child_a_flag),
            hold: Rc::clone(&hold_a),
        })
        .expect("a");
    let handle_b = ctx
        .plugin(PluginChild {
            flag: Rc::clone(&child_b_flag),
            hold: Rc::clone(&hold_b),
        })
        .expect("b");
    ctx.start().expect("start");

    // 先卸后装插件：暴露全表 retain 与 children_start 下调不一致的回归。
    handle_b.dispose().expect("dispose b");
    assert!(
        child_b_flag.get(),
        "plugin B scoped child must dispose despite prior dead root isolate"
    );
    assert!(
        hold_b.borrow().as_ref().is_some_and(|c| c.is_disposed()),
        "B child context disposed"
    );

    handle_a.dispose().expect("dispose a");
    assert!(
        child_a_flag.get(),
        "plugin A scoped child must still dispose after B unload + dead root slot"
    );
    assert!(
        hold_a.borrow().as_ref().is_some_and(|c| c.is_disposed()),
        "A child context disposed"
    );
}
