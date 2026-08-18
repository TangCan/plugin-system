#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 2.1 — PluginScope 构建期自动记录（ATDD）。
//!
//! Red phase: 在 PluginScope / 作用域栈落地前应编译失败或断言失败。

use std::any::TypeId;
use std::cell::RefCell;
use std::rc::Rc;

use plugctx::{Context, Error, Plugin, PluginScope};

#[derive(Clone, Copy)]
struct SvcA;

#[derive(Clone, Copy)]
struct SvcB;

struct Ping;

/// AC#1: build 内 provide / on / effect / isolate 写入对应 PluginScope。
#[test]
fn build_records_provide_on_effect_isolate_in_scope() {
    struct RecordingPlugin;

    impl Plugin for RecordingPlugin {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SvcA);
            let _ = ctx.on::<Ping>(|_e| {});
            let _ = ctx.effect(|| || {});
            let _child = ctx.isolate().expect("isolate");
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle = ctx.plugin(RecordingPlugin).expect("install");
    assert!(
        handle.scope().is_none(),
        "delayed install must not have scope before build"
    );
    ctx.start().expect("start");

    let scope = handle.scope().expect("built plugin must expose scope");
    assert!(
        scope.provided_services.contains(&TypeId::of::<SvcA>()),
        "provide must record service TypeId"
    );
    assert!(
        scope
            .registered_events
            .iter()
            .any(|(tid, _)| *tid == TypeId::of::<Ping>()),
        "on must record event registration"
    );
    assert!(
        scope.effects_count >= 1,
        "effect must increment effects_count"
    );
    assert!(
        scope.children_count >= 1,
        "isolate must increment children_count"
    );
}

/// AC#2: 根级 provide/on/effect 合法且不计入任何插件 scope。
#[test]
fn root_level_registrations_not_in_plugin_scope() {
    struct EmptyPlugin;

    impl Plugin for EmptyPlugin {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            Ok(())
        }
    }

    let ctx = Context::new();
    ctx.provide(SvcA);
    let _ = ctx.on::<Ping>(|_e| {});
    let _ = ctx.effect(|| || {});

    let handle = ctx.plugin(EmptyPlugin).expect("install");
    ctx.start().expect("start");

    let scope = handle.scope().expect("empty build still yields scope");
    assert!(
        scope.provided_services.is_empty(),
        "root provide must not appear in plugin scope"
    );
    assert!(
        scope.registered_events.is_empty(),
        "root on must not appear in plugin scope"
    );
    assert_eq!(
        scope.effects_count, 0,
        "root effect must not appear in plugin scope"
    );
    // 根级注册仍可用
    assert!(ctx.get::<SvcA>().is_some());
}

/// AC#3: 嵌套 build 时内外层 scope 不串扰。
#[test]
fn nested_plugin_build_scopes_do_not_cross() {
    struct Inner;

    impl Plugin for Inner {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SvcB);
            let _ = ctx.on::<Ping>(|_e| {});
            Ok(())
        }
    }

    struct Outer {
        inner: Rc<RefCell<Option<plugctx::PluginHandle>>>,
    }

    impl Plugin for Outer {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SvcA);
            let handle = ctx.plugin(Inner).expect("nested immediate install");
            *self.inner.borrow_mut() = Some(handle);
            Ok(())
        }
    }

    let inner_slot = Rc::new(RefCell::new(None));
    let ctx = Context::new();
    ctx.start()
        .expect("start empty so nested plugin builds immediately");
    let outer = ctx
        .plugin(Outer {
            inner: Rc::clone(&inner_slot),
        })
        .expect("install outer");

    let outer_scope = outer.scope().expect("outer scope");
    let inner = inner_slot
        .borrow_mut()
        .take()
        .expect("inner handle captured");
    let inner_scope = inner.scope().expect("inner scope");

    assert!(
        outer_scope
            .provided_services
            .contains(&TypeId::of::<SvcA>()),
        "outer must record SvcA"
    );
    assert!(
        !outer_scope
            .provided_services
            .contains(&TypeId::of::<SvcB>()),
        "outer must not record inner SvcB"
    );
    assert!(
        inner_scope
            .provided_services
            .contains(&TypeId::of::<SvcB>()),
        "inner must record SvcB"
    );
    assert!(
        !inner_scope
            .provided_services
            .contains(&TypeId::of::<SvcA>()),
        "inner must not record outer SvcA"
    );
    assert!(
        !outer_scope
            .registered_events
            .iter()
            .any(|(tid, _)| *tid == TypeId::of::<Ping>()),
        "outer must not inherit inner event regs"
    );
    assert!(
        inner_scope
            .registered_events
            .iter()
            .any(|(tid, _)| *tid == TypeId::of::<Ping>()),
        "inner must record its on::<Ping>"
    );
}

/// 立即安装路径同样写入 scope。
#[test]
fn immediate_install_also_records_scope() {
    struct P;

    impl Plugin for P {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SvcA);
            Ok(())
        }
    }

    let ctx = Context::new();
    ctx.start().expect("start");
    let h = ctx.plugin(P).expect("immediate");
    let scope = h.scope().expect("scope after immediate build");
    assert!(scope.provided_services.contains(&TypeId::of::<SvcA>()));
}

/// PluginScope 公开字段形状（effects_start + count）可供后续卸载使用。
#[test]
fn plugin_scope_exposes_effects_interval_fields() {
    let _s = PluginScope {
        provided_services: vec![],
        provided_trait_services: vec![],
        registered_events: vec![],
        registered_async_events: vec![],
        effects_start: 0,
        effects_count: 0,
        children_start: 0,
        children_count: 0,
    };
}

/// Automate: 根级已有 effect 时，插件 effects_start 为全局连续区间起点。
#[test]
fn effects_start_accounts_for_prior_root_effects() {
    struct P;

    impl Plugin for P {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            let _ = ctx.effect(|| || {});
            let _ = ctx.effect(|| || {});
            Ok(())
        }
    }

    let ctx = Context::new();
    let _ = ctx.effect(|| || {}); // 根级，不计 scope
    let h = ctx.plugin(P).expect("install");
    ctx.start().expect("start");
    let scope = h.scope().expect("scope");
    assert_eq!(scope.effects_start, 1);
    assert_eq!(scope.effects_count, 2);
}

/// Automate: 构建失败时不保留 scope，且作用域栈可继续服务后续插件。
#[test]
fn failed_build_leaves_no_scope_and_stack_reusable() {
    struct Boom;

    impl Plugin for Boom {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SvcA);
            Err(Error::BuildFailed)
        }
    }

    struct OkPlugin;

    impl Plugin for OkPlugin {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SvcB);
            Ok(())
        }
    }

    let ctx = Context::new();
    ctx.start().expect("start empty");
    let err = ctx.plugin(Boom).expect_err("build must fail");
    assert!(matches!(err, Error::BuildFailed));

    let ok = ctx.plugin(OkPlugin).expect("install after failed");
    let scope = ok.scope().expect("ok plugin scope");
    assert!(scope.provided_services.contains(&TypeId::of::<SvcB>()));
    assert!(!scope.provided_services.contains(&TypeId::of::<SvcA>()));
}

/// Automate: `Context::plugin_scope` 与 `PluginHandle::scope` 一致。
#[test]
fn context_plugin_scope_matches_handle() {
    struct P;

    impl Plugin for P {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide(SvcA);
            Ok(())
        }
    }

    let ctx = Context::new();
    let h = ctx.plugin(P).expect("install");
    ctx.start().expect("start");
    let via_handle = h.scope().expect("handle scope");
    let via_ctx = ctx.plugin_scope(h.id()).expect("ctx scope");
    assert_eq!(via_handle.provided_services, via_ctx.provided_services);
}
