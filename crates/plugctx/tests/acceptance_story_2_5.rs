#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 2.5 — §5.3 卸载回滚细则（覆盖/索引/区间）（ATDD）。
//!
//! Red phase: 当前 dispose 按 TypeId 无条件 remove，A→B 覆盖后 dispose A 会误删 B 的服务。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use plugctx::{Context, Error, Plugin};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SharedSvc(u32);

struct Ev;

trait Greeter: 'static {
    fn greet(&self) -> &str;
}

struct Hello;
impl Greeter for Hello {
    fn greet(&self) -> &str {
        "hello"
    }
}

struct Hi;
impl Greeter for Hi {
    fn greet(&self) -> &str {
        "hi"
    }
}

/// AC#1: A provide T → B provide T → dispose A 后当前 T（B）仍保留。
#[test]
fn dispose_a_does_not_remove_service_overwritten_by_b() {
    struct PluginA;
    impl Plugin for PluginA {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SharedSvc(1));
            Ok(())
        }
    }

    struct PluginB;
    impl Plugin for PluginB {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SharedSvc(2));
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle_a = ctx.plugin(PluginA).expect("install a");
    let handle_b = ctx.plugin(PluginB).expect("install b");
    ctx.start().expect("start");

    assert_eq!(*ctx.get::<SharedSvc>().expect("shared"), SharedSvc(2));

    handle_a.dispose().expect("dispose a");

    assert_eq!(
        *ctx.get::<SharedSvc>().expect("B's service must remain"),
        SharedSvc(2),
        "disposing A must not remove T overwritten by B"
    );
    assert!(handle_b.is_alive());

    handle_b.dispose().expect("dispose b");
    assert!(
        ctx.get::<SharedSvc>().is_none(),
        "disposing B (current owner) must remove T"
    );
}

/// AC#1 对称: provide_trait 覆盖后 dispose 先提供者不得误删。
#[test]
fn dispose_a_does_not_remove_trait_service_overwritten_by_b() {
    struct PluginA;
    impl Plugin for PluginA {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide_trait::<dyn Greeter>(Box::new(Hello));
            Ok(())
        }
    }

    struct PluginB;
    impl Plugin for PluginB {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide_trait::<dyn Greeter>(Box::new(Hi));
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle_a = ctx.plugin(PluginA).expect("install a");
    let handle_b = ctx.plugin(PluginB).expect("install b");
    ctx.start().expect("start");

    assert_eq!(
        ctx.get_trait::<dyn Greeter>().expect("greeter").greet(),
        "hi"
    );

    handle_a.dispose().expect("dispose a");
    assert_eq!(
        ctx.get_trait::<dyn Greeter>()
            .expect("B greeter must remain")
            .greet(),
        "hi"
    );

    handle_b.dispose().expect("dispose b");
    assert!(ctx.get_trait::<dyn Greeter>().is_none());
}

/// AC#1 变体: A provide → 根级覆盖 → dispose A 不得误删根级值。
#[test]
fn dispose_a_does_not_remove_root_overwritten_service() {
    struct PluginA;
    impl Plugin for PluginA {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SharedSvc(1));
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle_a = ctx.plugin(PluginA).expect("install a");
    ctx.start().expect("start");
    assert_eq!(*ctx.get::<SharedSvc>().expect("a"), SharedSvc(1));

    ctx.provide(SharedSvc(99));
    handle_a.dispose().expect("dispose a");

    assert_eq!(
        *ctx.get::<SharedSvc>().expect("root overwrite"),
        SharedSvc(99),
        "root-level overwrite must survive dispose A"
    );
}

/// AC#2: 多监听按索引规则移除；仅 A 的监听消失，B 仍触发。
#[test]
fn dispose_a_removes_only_a_listeners_by_index_rules() {
    let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));

    struct PluginA {
        log: Rc<RefCell<Vec<&'static str>>>,
    }
    impl Plugin for PluginA {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            let log = Rc::clone(&self.log);
            let _ = ctx.on::<Ev>(move |_e| {
                log.borrow_mut().push("a1");
            });
            let log = Rc::clone(&self.log);
            let _ = ctx.on::<Ev>(move |_e| {
                log.borrow_mut().push("a2");
            });
            Ok(())
        }
    }

    struct PluginB {
        log: Rc<RefCell<Vec<&'static str>>>,
    }
    impl Plugin for PluginB {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            let log = Rc::clone(&self.log);
            let _ = ctx.on::<Ev>(move |_e| {
                log.borrow_mut().push("b");
            });
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle_a = ctx
        .plugin(PluginA {
            log: Rc::clone(&log),
        })
        .expect("a");
    let _handle_b = ctx
        .plugin(PluginB {
            log: Rc::clone(&log),
        })
        .expect("b");
    ctx.start().expect("start");

    ctx.emit(&Ev);
    assert_eq!(*log.borrow(), vec!["a1", "a2", "b"]);

    log.borrow_mut().clear();
    handle_a.dispose().expect("dispose a");
    ctx.emit(&Ev);
    assert_eq!(
        *log.borrow(),
        vec!["b"],
        "only B listener must remain after dispose A"
    );
}

/// AC#2: 多 effect 连续区间；dispose A 逆序仅跑 A 的 cleanup。
#[test]
fn dispose_a_runs_only_a_effects_in_reverse_range() {
    let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));

    struct PluginA {
        log: Rc<RefCell<Vec<&'static str>>>,
    }
    impl Plugin for PluginA {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            let log = Rc::clone(&self.log);
            let _ = ctx.effect(move || {
                let log = Rc::clone(&log);
                move || log.borrow_mut().push("a-e1")
            });
            let log = Rc::clone(&self.log);
            let _ = ctx.effect(move || {
                let log = Rc::clone(&log);
                move || log.borrow_mut().push("a-e2")
            });
            Ok(())
        }
    }

    struct PluginB {
        log: Rc<RefCell<Vec<&'static str>>>,
    }
    impl Plugin for PluginB {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            let log = Rc::clone(&self.log);
            let _ = ctx.effect(move || {
                let log = Rc::clone(&log);
                move || log.borrow_mut().push("b-e1")
            });
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle_a = ctx
        .plugin(PluginA {
            log: Rc::clone(&log),
        })
        .expect("a");
    let _handle_b = ctx
        .plugin(PluginB {
            log: Rc::clone(&log),
        })
        .expect("b");
    ctx.start().expect("start");

    handle_a.dispose().expect("dispose a");
    assert_eq!(
        *log.borrow(),
        vec!["a-e2", "a-e1"],
        "A effects must run in reverse; B must not run"
    );
}

/// AC#3: dispose A 销毁其 isolate 子上下文，不影响兄弟插件。
#[test]
fn dispose_a_destroys_child_without_affecting_sibling() {
    let child_flag = Rc::new(Cell::new(false));
    let child_hold = Rc::new(RefCell::new(None::<Context>));
    let sibling_alive = Rc::new(Cell::new(true));

    struct PluginA {
        flag: Rc<Cell<bool>>,
        hold: Rc<RefCell<Option<Context>>>,
    }
    impl Plugin for PluginA {
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

    struct PluginB {
        alive: Rc<Cell<bool>>,
    }
    impl Plugin for PluginB {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SharedSvc(42));
            let alive = Rc::clone(&self.alive);
            let _ = ctx.effect(move || {
                let alive = Rc::clone(&alive);
                move || alive.set(false)
            });
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle_a = ctx
        .plugin(PluginA {
            flag: Rc::clone(&child_flag),
            hold: Rc::clone(&child_hold),
        })
        .expect("a");
    let handle_b = ctx
        .plugin(PluginB {
            alive: Rc::clone(&sibling_alive),
        })
        .expect("b");
    ctx.start().expect("start");

    assert!(
        child_hold
            .borrow()
            .as_ref()
            .is_some_and(|c| !c.is_disposed()),
        "child must exist after build"
    );

    handle_a.dispose().expect("dispose a");
    assert!(child_flag.get(), "A's child must be disposed");
    assert!(
        child_hold
            .borrow()
            .as_ref()
            .is_some_and(|c| c.is_disposed()),
        "scoped child context must be disposed"
    );
    assert!(handle_b.is_alive());
    assert_eq!(*ctx.get::<SharedSvc>().expect("b svc"), SharedSvc(42));
    assert!(
        sibling_alive.get(),
        "B effect must not run when only A is disposed"
    );
}

/// Automate 护栏: A 多次 provide 同 T 后被 B 覆盖；dispose A 仍保留 B。
#[test]
fn repeated_provide_then_overwrite_survives_dispose_a() {
    struct PluginA;
    impl Plugin for PluginA {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SharedSvc(1));
            ctx.provide(SharedSvc(11));
            Ok(())
        }
    }

    struct PluginB;
    impl Plugin for PluginB {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SharedSvc(22));
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle_a = ctx.plugin(PluginA).expect("a");
    let handle_b = ctx.plugin(PluginB).expect("b");
    ctx.start().expect("start");

    handle_a.dispose().expect("dispose a");
    assert_eq!(*ctx.get::<SharedSvc>().expect("b"), SharedSvc(22));
    handle_b.dispose().expect("dispose b");
    assert!(ctx.get::<SharedSvc>().is_none());
}

/// Automate 护栏: 先 dispose 覆盖方 B，再 dispose A — A 不再拥有当前值故无副作用。
#[test]
fn dispose_overwriter_then_original_is_noop_for_service() {
    struct PluginA;
    impl Plugin for PluginA {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SharedSvc(1));
            Ok(())
        }
    }

    struct PluginB;
    impl Plugin for PluginB {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SharedSvc(2));
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle_a = ctx.plugin(PluginA).expect("a");
    let handle_b = ctx.plugin(PluginB).expect("b");
    ctx.start().expect("start");

    handle_b.dispose().expect("dispose b");
    assert!(
        ctx.get::<SharedSvc>().is_none(),
        "current owner B dispose must remove T"
    );
    handle_a.dispose().expect("dispose a after b");
    assert!(ctx.get::<SharedSvc>().is_none());
}

/// Automate 护栏: A/B 交错同事件监听；dispose A 后仅 B 触发，顺序稳定。
#[test]
fn interleaved_same_event_listeners_survive_partial_dispose() {
    let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));

    struct PluginA {
        log: Rc<RefCell<Vec<&'static str>>>,
    }
    impl Plugin for PluginA {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            let log = Rc::clone(&self.log);
            let _ = ctx.on::<Ev>(move |_e| {
                log.borrow_mut().push("a");
            });
            Ok(())
        }
    }

    struct PluginB {
        log: Rc<RefCell<Vec<&'static str>>>,
    }
    impl Plugin for PluginB {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            let log = Rc::clone(&self.log);
            let _ = ctx.on::<Ev>(move |_e| {
                log.borrow_mut().push("b1");
            });
            let log = Rc::clone(&self.log);
            let _ = ctx.on::<Ev>(move |_e| {
                log.borrow_mut().push("b2");
            });
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle_a = ctx
        .plugin(PluginA {
            log: Rc::clone(&log),
        })
        .expect("a");
    let _handle_b = ctx
        .plugin(PluginB {
            log: Rc::clone(&log),
        })
        .expect("b");
    ctx.start().expect("start");

    handle_a.dispose().expect("dispose a");
    ctx.emit(&Ev);
    assert_eq!(*log.borrow(), vec!["b1", "b2"]);
}
