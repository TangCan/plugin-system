#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 2.3 — trait 对象服务 provide_trait / get_trait（ATDD）。
//!
//! Red phase: `provide_trait` / `get_trait` 未落地前编译失败；落地后验证 FR17。

use std::any::TypeId;
use std::cell::Cell;
use std::rc::Rc;

use plugctx::{Context, Error, Plugin};

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

trait Counter: 'static {
    fn value(&self) -> u32;
}

struct CounterImpl(u32);
impl Counter for CounterImpl {
    fn value(&self) -> u32 {
        self.0
    }
}

/// AC#1: provide_trait 后 get_trait 返回可用引用。
#[test]
fn provide_trait_then_get_trait_returns_usable_ref() {
    let ctx = Context::new();
    assert!(ctx.provide_trait::<dyn Greeter>(Box::new(Hello)).is_none());

    let g = ctx
        .get_trait::<dyn Greeter>()
        .expect("trait service present");
    assert_eq!(g.greet(), "hello");
}

/// AC#2: 再次 provide_trait 同一 trait 返回被替换的旧 Box。
#[test]
fn provide_trait_replace_returns_old_box() {
    let ctx = Context::new();
    ctx.provide_trait::<dyn Greeter>(Box::new(Hello));

    let old = ctx
        .provide_trait::<dyn Greeter>(Box::new(Hi))
        .expect("must return previous Box");
    assert_eq!(old.greet(), "hello");

    let g = ctx.get_trait::<dyn Greeter>().expect("replaced");
    assert_eq!(g.greet(), "hi");
}

/// AC#3: 子 Context 可继承读到父 trait 服务；子级覆盖不污染父级。
#[test]
fn child_inherits_trait_service_and_override_is_local() {
    let parent = Context::new();
    parent.provide_trait::<dyn Greeter>(Box::new(Hello));

    let child = parent.isolate().expect("isolate");
    {
        let g = child
            .get_trait::<dyn Greeter>()
            .expect("child inherits parent trait service");
        assert_eq!(g.greet(), "hello");
    }

    child.provide_trait::<dyn Greeter>(Box::new(Hi));
    {
        let g = child.get_trait::<dyn Greeter>().expect("child override");
        assert_eq!(g.greet(), "hi");
    }
    {
        let g = parent.get_trait::<dyn Greeter>().expect("parent unchanged");
        assert_eq!(g.greet(), "hello");
    }
}

/// AC#4: 插件 build 中 provide_trait，dispose 后从 scope 移除。
#[test]
fn plugin_dispose_removes_scoped_trait_service() {
    struct Provider;
    impl Plugin for Provider {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide_trait::<dyn Counter>(Box::new(CounterImpl(42)));
            Ok(())
        }
    }

    struct Sibling;
    impl Plugin for Sibling {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide_trait::<dyn Greeter>(Box::new(Hello));
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle = ctx.plugin(Provider).expect("install provider");
    let sibling = ctx.plugin(Sibling).expect("install sibling");
    ctx.start().expect("start");

    assert_eq!(ctx.get_trait::<dyn Counter>().expect("present").value(), 42);
    assert!(ctx.get_trait::<dyn Greeter>().is_some());

    let scope = handle.scope().expect("built scope");
    assert!(
        scope
            .provided_trait_services
            .contains(&TypeId::of::<dyn Counter>()),
        "scope must record trait TypeId"
    );

    handle.dispose().expect("dispose provider");

    assert!(
        ctx.get_trait::<dyn Counter>().is_none(),
        "trait service must be removed with plugin scope"
    );
    assert!(
        ctx.get_trait::<dyn Greeter>().is_some(),
        "sibling trait service must remain"
    );
    assert!(sibling.is_alive());
}

/// 根级 provide_trait 不计入任何插件 scope；插件 dispose 不移除根级 trait 服务。
#[test]
fn root_level_trait_service_survives_plugin_dispose() {
    struct Empty;
    impl Plugin for Empty {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            Ok(())
        }
    }

    let ctx = Context::new();
    ctx.provide_trait::<dyn Greeter>(Box::new(Hello));
    let handle = ctx.plugin(Empty).expect("install");
    ctx.start().expect("start");

    handle.dispose().expect("dispose empty");
    assert_eq!(
        ctx.get_trait::<dyn Greeter>().expect("root kept").greet(),
        "hello"
    );
}

/// 具体类型 services 与 trait_services 表隔离：同名无关 TypeId 互不干扰。
#[test]
fn concrete_provide_does_not_satisfy_get_trait() {
    let ctx = Context::new();
    // 具体类型服务不能被 get_trait 取到
    ctx.provide(CounterImpl(7));
    assert!(ctx.get::<CounterImpl>().is_some());
    assert!(
        ctx.get_trait::<dyn Counter>().is_none(),
        "concrete provide must not populate trait_services"
    );

    ctx.provide_trait::<dyn Counter>(Box::new(CounterImpl(9)));
    assert_eq!(ctx.get_trait::<dyn Counter>().unwrap().value(), 9);
    // 具体类型条目仍在
    assert_eq!(ctx.get::<CounterImpl>().unwrap().0, 7);
}

/// get_trait 未命中返回 None；不 panic。
#[test]
fn get_trait_missing_returns_none() {
    let ctx = Context::new();
    assert!(ctx.get_trait::<dyn Greeter>().is_none());
}

/// 插件 scope 记账：build 内 provide_trait 写入 provided_trait_services。
#[test]
fn plugin_scope_records_provide_trait_type_ids() {
    let recorded = Rc::new(Cell::new(false));

    struct Recorder {
        recorded: Rc<Cell<bool>>,
    }
    impl Plugin for Recorder {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide_trait::<dyn Greeter>(Box::new(Hello));
            self.recorded.set(true);
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle = ctx
        .plugin(Recorder {
            recorded: Rc::clone(&recorded),
        })
        .expect("install");
    ctx.start().expect("start");
    assert!(recorded.get());

    let scope = handle.scope().expect("scope");
    assert_eq!(
        scope.provided_trait_services,
        vec![TypeId::of::<dyn Greeter>()]
    );
}

/// Automate: Context::dispose 清空 trait_services；幂等。
#[test]
fn context_dispose_clears_trait_services() {
    let ctx = Context::new();
    ctx.provide_trait::<dyn Greeter>(Box::new(Hello));
    assert!(ctx.get_trait::<dyn Greeter>().is_some());
    ctx.dispose();
    assert!(ctx.get_trait::<dyn Greeter>().is_none());
    ctx.dispose(); // 幂等
}

/// Automate: 孙上下文可沿父链继承 trait 服务。
#[test]
fn grandchild_inherits_trait_service_via_parent_chain() {
    let root = Context::new();
    root.provide_trait::<dyn Counter>(Box::new(CounterImpl(3)));
    let mid = root.isolate().expect("isolate");
    let leaf = mid.isolate().expect("isolate");
    assert_eq!(leaf.get_trait::<dyn Counter>().expect("inherit").value(), 3);
}

/// Automate: 同一插件多次 provide_trait 同 trait 时 scope 多次记账，dispose 后仍清空。
#[test]
fn repeated_provide_trait_in_build_still_removed_on_dispose() {
    struct DoubleProvide;
    impl Plugin for DoubleProvide {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            ctx.provide_trait::<dyn Greeter>(Box::new(Hello));
            ctx.provide_trait::<dyn Greeter>(Box::new(Hi));
            Ok(())
        }
    }

    let ctx = Context::new();
    let handle = ctx.plugin(DoubleProvide).expect("install");
    ctx.start().expect("start");

    let scope = handle.scope().expect("scope");
    assert_eq!(
        scope.provided_trait_services,
        vec![TypeId::of::<dyn Greeter>(), TypeId::of::<dyn Greeter>()]
    );
    assert_eq!(ctx.get_trait::<dyn Greeter>().unwrap().greet(), "hi");

    handle.dispose().expect("dispose");
    assert!(ctx.get_trait::<dyn Greeter>().is_none());
}
