#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 2.4 — ContextInterceptor 切入 build/emit（ATDD）。
//!
//! 验证 FR16：`add_interceptor`、build/emit 前后钩子、注册序、失败不调 after、有限重入。

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

use plugctx::{Context, ContextInterceptor, Error, Plugin};

/// 记录钩子调用顺序的测试拦截器。
struct RecordingInterceptor {
    name: &'static str,
    log: Rc<RefCell<Vec<String>>>,
}

impl ContextInterceptor for RecordingInterceptor {
    fn before_plugin_build(&self, _plugin: &dyn Plugin) {
        self.log
            .borrow_mut()
            .push(format!("{}:before_plugin_build", self.name));
    }

    fn after_plugin_build(&self, _plugin: &dyn Plugin) {
        self.log
            .borrow_mut()
            .push(format!("{}:after_plugin_build", self.name));
    }

    fn before_emit(&self, event: &dyn Any) {
        let label = if event.is::<Ping>() { "Ping" } else { "Other" };
        self.log
            .borrow_mut()
            .push(format!("{}:before_emit:{label}", self.name));
    }

    fn after_emit(&self, event: &dyn Any) {
        let label = if event.is::<Ping>() { "Ping" } else { "Other" };
        self.log
            .borrow_mut()
            .push(format!("{}:after_emit:{label}", self.name));
    }
}

struct OkPlugin;
impl Plugin for OkPlugin {
    fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
        Ok(())
    }
}

struct FailPlugin;
impl Plugin for FailPlugin {
    fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
        Err(Error::BuildFailed)
    }
}

#[derive(Clone, Copy)]
struct Ping;

/// AC#1: build 成功时 before → after；失败时仅 before。
#[test]
fn plugin_build_hooks_success_and_failure() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let ctx = Context::new();
    ctx.add_interceptor(RecordingInterceptor {
        name: "A",
        log: Rc::clone(&log),
    });

    ctx.plugin(OkPlugin).expect("install ok");
    ctx.start().expect("start builds ok plugin");
    assert_eq!(
        log.borrow().as_slice(),
        &[
            "A:before_plugin_build".to_string(),
            "A:after_plugin_build".to_string(),
            // start 后 ReadyEvent emit
            "A:before_emit:Other".to_string(),
            "A:after_emit:Other".to_string(),
        ]
    );

    log.borrow_mut().clear();
    let started = Context::new();
    started.add_interceptor(RecordingInterceptor {
        name: "B",
        log: Rc::clone(&log),
    });
    started.start().expect("empty start");
    log.borrow_mut().clear();

    // 已启动后立即构建失败插件
    let err = started.plugin(FailPlugin).expect_err("build must fail");
    assert!(matches!(err, Error::BuildFailed));
    assert_eq!(
        log.borrow().as_slice(),
        &["B:before_plugin_build".to_string()],
        "build 失败时不得调用 after_plugin_build"
    );
}

/// AC#2: emit 前后钩子；监听器仍按注册序。
#[test]
fn emit_hooks_wrap_listeners_in_registration_order() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let ctx = Context::new();
    ctx.add_interceptor(RecordingInterceptor {
        name: "I",
        log: Rc::clone(&log),
    });

    let listener_log = Rc::new(RefCell::new(Vec::new()));
    let l1 = Rc::clone(&listener_log);
    let l2 = Rc::clone(&listener_log);
    ctx.on::<Ping>(move |_| l1.borrow_mut().push("L1"));
    ctx.on::<Ping>(move |_| l2.borrow_mut().push("L2"));

    ctx.emit(&Ping);

    assert_eq!(listener_log.borrow().as_slice(), &["L1", "L2"]);
    assert_eq!(
        log.borrow().as_slice(),
        &[
            "I:before_emit:Ping".to_string(),
            "I:after_emit:Ping".to_string(),
        ]
    );
}

/// AC#3: 多拦截器注册序稳定（before/after 均为 FIFO）。
#[test]
fn multiple_interceptors_stable_registration_order() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let ctx = Context::new();
    ctx.add_interceptor(RecordingInterceptor {
        name: "A",
        log: Rc::clone(&log),
    });
    ctx.add_interceptor(RecordingInterceptor {
        name: "B",
        log: Rc::clone(&log),
    });

    ctx.plugin(OkPlugin).expect("install");
    ctx.start().expect("start");

    let entries: Vec<String> = log
        .borrow()
        .iter()
        .filter(|e| e.contains("plugin_build"))
        .cloned()
        .collect();
    assert_eq!(
        entries,
        vec![
            "A:before_plugin_build".to_string(),
            "B:before_plugin_build".to_string(),
            "A:after_plugin_build".to_string(),
            "B:after_plugin_build".to_string(),
        ]
    );

    log.borrow_mut().clear();
    ctx.emit(&Ping);
    assert_eq!(
        log.borrow().as_slice(),
        &[
            "A:before_emit:Ping".to_string(),
            "B:before_emit:Ping".to_string(),
            "A:after_emit:Ping".to_string(),
            "B:after_emit:Ping".to_string(),
        ]
    );
}

/// AC#4: 钩子内调用 Context API 不 panic。
#[test]
fn interceptor_reentrancy_does_not_panic() {
    struct Reentrant;
    impl ContextInterceptor for Reentrant {
        fn before_emit(&self, event: &dyn Any) {
            if event.is::<Ping>() {
                // 钩子内 provide / on 不应因 RefCell 冲突 panic
            }
        }
    }

    let ctx = Context::new();
    let ctx_for_hook = ctx.clone();
    struct ProvideOnBefore {
        ctx: Context,
    }
    impl ContextInterceptor for ProvideOnBefore {
        fn before_emit(&self, event: &dyn Any) {
            if event.is::<Ping>() {
                self.ctx.provide(42u32);
                let _ = self.ctx.on::<Ping>(|_| {});
            }
        }
        fn after_plugin_build(&self, _plugin: &dyn Plugin) {
            self.ctx.provide(7u8);
        }
    }

    ctx.add_interceptor(ProvideOnBefore { ctx: ctx_for_hook });
    ctx.add_interceptor(Reentrant);

    ctx.plugin(OkPlugin).expect("install");
    ctx.start().expect("start");
    assert_eq!(*ctx.get::<u8>().expect("from after_plugin_build"), 7);

    ctx.emit(&Ping);
    assert_eq!(*ctx.get::<u32>().expect("from before_emit"), 42);
}

/// 子上下文不继承父拦截器。
#[test]
fn child_context_does_not_inherit_interceptors() {
    let parent_log = Rc::new(RefCell::new(Vec::new()));
    let parent = Context::new();
    parent.add_interceptor(RecordingInterceptor {
        name: "P",
        log: Rc::clone(&parent_log),
    });

    let child = parent.isolate().expect("isolate");
    child.emit(&Ping);
    assert!(parent_log.borrow().is_empty(), "子 emit 不得触发父拦截器");

    let child_log = Rc::new(RefCell::new(Vec::new()));
    child.add_interceptor(RecordingInterceptor {
        name: "C",
        log: Rc::clone(&child_log),
    });
    child.emit(&Ping);
    assert_eq!(
        child_log.borrow().as_slice(),
        &[
            "C:before_emit:Ping".to_string(),
            "C:after_emit:Ping".to_string(),
        ]
    );
    assert!(parent_log.borrow().is_empty());
}

/// 无监听器时 emit 仍环绕 before/after。
#[test]
fn emit_without_listeners_still_runs_interceptors() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let ctx = Context::new();
    ctx.add_interceptor(RecordingInterceptor {
        name: "I",
        log: Rc::clone(&log),
    });
    ctx.emit(&Ping);
    assert_eq!(
        log.borrow().as_slice(),
        &[
            "I:before_emit:Ping".to_string(),
            "I:after_emit:Ping".to_string(),
        ]
    );
}

/// Automate: dispose 清空拦截器，之后 emit 不再触发旧钩子。
#[test]
fn context_dispose_clears_interceptors() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let ctx = Context::new();
    ctx.add_interceptor(RecordingInterceptor {
        name: "I",
        log: Rc::clone(&log),
    });
    ctx.dispose();
    log.borrow_mut().clear();
    // DisposeEvent 已在 dispose 内 emit；清空后再 emit 不得再记入
    ctx.emit(&Ping);
    assert!(log.borrow().is_empty(), "dispose 后拦截器应已清空");
}

/// Automate: before_emit 内嵌套 emit 不 panic，且内外钩子均完整。
#[test]
fn nested_emit_from_before_hook_completes() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let ctx = Context::new();
    let ctx_hook = ctx.clone();
    let log_hook = Rc::clone(&log);

    struct NestEmit {
        ctx: Context,
        log: Rc<RefCell<Vec<String>>>,
        armed: RefCell<bool>,
    }
    impl ContextInterceptor for NestEmit {
        fn before_emit(&self, event: &dyn Any) {
            if event.is::<Ping>() && !*self.armed.borrow() {
                *self.armed.borrow_mut() = true;
                self.log.borrow_mut().push("outer_before".into());
                self.ctx.emit(&Pong);
            } else if event.is::<Pong>() {
                self.log.borrow_mut().push("inner_before".into());
            }
        }
        fn after_emit(&self, event: &dyn Any) {
            if event.is::<Pong>() {
                self.log.borrow_mut().push("inner_after".into());
            } else if event.is::<Ping>() {
                self.log.borrow_mut().push("outer_after".into());
            }
        }
    }

    #[derive(Clone, Copy)]
    struct Pong;

    ctx.add_interceptor(NestEmit {
        ctx: ctx_hook,
        log: log_hook,
        armed: RefCell::new(false),
    });
    ctx.emit(&Ping);
    assert_eq!(
        log.borrow().as_slice(),
        &[
            "outer_before".to_string(),
            "inner_before".to_string(),
            "inner_after".to_string(),
            "outer_after".to_string(),
        ]
    );
}

/// Automate: after_plugin_build 内 provide 仍计入当前 PluginScope。
#[test]
fn after_plugin_build_provide_is_scoped() {
    struct Marker;
    struct Scoped;
    impl Plugin for Scoped {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            Ok(())
        }
    }

    struct AfterProvide {
        ctx: Context,
    }
    impl ContextInterceptor for AfterProvide {
        fn after_plugin_build(&self, _plugin: &dyn Plugin) {
            self.ctx.provide(Marker);
        }
    }

    let ctx = Context::new();
    ctx.add_interceptor(AfterProvide { ctx: ctx.clone() });
    let handle = ctx.plugin(Scoped).expect("install");
    ctx.start().expect("start");

    let scope = handle.scope().expect("built");
    assert!(
        scope.provided_services.contains(&TypeId::of::<Marker>()),
        "after_plugin_build 内 provide 应记入 scope"
    );
    handle.dispose().expect("dispose");
    assert!(ctx.get::<Marker>().is_none());
}
