#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 3.1 — AsyncPlugin 与 start_async（ATDD）。
//!
//! 需启用 feature：`cargo test -p plugctx --features async --test acceptance_story_3_1`
//!
//! 使用 `futures::executor::block_on` 驱动，避免测试依赖绑定单一运行时（AC#4 / NFR1）。

use std::cell::RefCell;
use std::rc::Rc;

use async_trait::async_trait;
use futures::executor::block_on;
use plugctx::{AsyncPlugin, Context, Error, Plugin, ReadyEvent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SyncSvc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AsyncSvc;

/// AC#2: 同步插件提供 SyncSvc，异步插件依赖它并提供 AsyncSvc；构建序正确。
#[test]
fn start_async_builds_sync_and_async_in_dependency_order() {
    let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));

    struct SyncProvider {
        order: Rc<RefCell<Vec<&'static str>>>,
    }
    impl Plugin for SyncProvider {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            self.order.borrow_mut().push("sync");
            ctx.provide(SyncSvc);
            Ok(())
        }
    }

    struct AsyncConsumer {
        order: Rc<RefCell<Vec<&'static str>>>,
    }
    impl Plugin for AsyncConsumer {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            Ok(())
        }
        fn dependencies(&self) -> Vec<std::any::TypeId> {
            vec![std::any::TypeId::of::<SyncSvc>()]
        }
    }
    #[async_trait(?Send)]
    impl AsyncPlugin for AsyncConsumer {
        async fn build_async(&self, ctx: &mut Context) -> Result<(), Error> {
            futures::future::ready(()).await;
            self.order.borrow_mut().push("async");
            assert!(
                ctx.get::<SyncSvc>().is_some(),
                "sync dep must be built first"
            );
            ctx.provide(AsyncSvc);
            Ok(())
        }
    }

    let ctx = Context::new();
    ctx.plugin_async(AsyncConsumer {
        order: Rc::clone(&order),
    })
    .expect("install async");
    ctx.plugin(SyncProvider {
        order: Rc::clone(&order),
    })
    .expect("install sync");

    block_on(ctx.start_async()).expect("start_async");

    assert_eq!(order.borrow().as_slice(), &["sync", "async"]);
    assert!(ctx.is_started());
    assert!(ctx.get::<SyncSvc>().is_some());
    assert!(ctx.get::<AsyncSvc>().is_some());
}

/// AC#3: build_async 失败 → BuildFailed，未 Started，无 ReadyEvent。
#[test]
fn start_async_build_failure_does_not_mark_started() {
    struct Boom;
    impl Plugin for Boom {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            Ok(())
        }
    }
    #[async_trait(?Send)]
    impl AsyncPlugin for Boom {
        async fn build_async(&self, _ctx: &mut Context) -> Result<(), Error> {
            Err(Error::BuildFailed)
        }
    }

    let ready = Rc::new(RefCell::new(0u32));
    let ctx = Context::new();
    let ready_c = Rc::clone(&ready);
    ctx.on::<ReadyEvent>(move |_e| {
        *ready_c.borrow_mut() += 1;
    });
    ctx.plugin_async(Boom).expect("install");

    let err = block_on(ctx.start_async()).expect_err("must fail");
    assert_eq!(err, Error::BuildFailed);
    assert!(!ctx.is_started());
    assert_eq!(*ready.borrow(), 0);
}

/// AC#4: 由调用方 executor 驱动；plugctx 不绑定 tokio/async-std。
#[test]
fn start_async_runs_on_caller_executor() {
    struct Tiny;
    impl Plugin for Tiny {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            Ok(())
        }
    }
    #[async_trait(?Send)]
    impl AsyncPlugin for Tiny {
        async fn build_async(&self, _ctx: &mut Context) -> Result<(), Error> {
            Ok(())
        }
    }

    let ctx = Context::new();
    ctx.plugin_async(Tiny).expect("install");
    block_on(ctx.start_async()).expect("start_async on caller executor");
    assert!(ctx.is_started());
}

/// Automate: 同步 `start` 对 Async 条目走 `Plugin::build`（无法 await）。
#[test]
fn sync_start_uses_plugin_build_for_async_entries() {
    let built_sync = Rc::new(RefCell::new(false));
    let built_async = Rc::new(RefCell::new(false));

    struct Dual {
        built_sync: Rc<RefCell<bool>>,
        built_async: Rc<RefCell<bool>>,
    }
    impl Plugin for Dual {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            *self.built_sync.borrow_mut() = true;
            Ok(())
        }
    }
    #[async_trait(?Send)]
    impl AsyncPlugin for Dual {
        async fn build_async(&self, _ctx: &mut Context) -> Result<(), Error> {
            *self.built_async.borrow_mut() = true;
            Ok(())
        }
    }

    let ctx = Context::new();
    ctx.plugin_async(Dual {
        built_sync: Rc::clone(&built_sync),
        built_async: Rc::clone(&built_async),
    })
    .expect("install");
    ctx.start().expect("sync start");
    assert!(*built_sync.borrow(), "sync start must call Plugin::build");
    assert!(
        !*built_async.borrow(),
        "sync start must not call build_async"
    );
}

/// Automate: 重复 start_async → AlreadyStarted。
#[test]
fn start_async_rejects_second_call() {
    struct Tiny;
    impl Plugin for Tiny {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            Ok(())
        }
    }
    #[async_trait(?Send)]
    impl AsyncPlugin for Tiny {
        async fn build_async(&self, _ctx: &mut Context) -> Result<(), Error> {
            Ok(())
        }
    }

    let ctx = Context::new();
    ctx.plugin_async(Tiny).expect("install");
    block_on(ctx.start_async()).expect("first");
    let err = block_on(ctx.start_async()).expect_err("second");
    assert_eq!(err, Error::AlreadyStarted);
}

/// Automate: 仅用 `plugin` 安装即使类型实现了 AsyncPlugin，start_async 仍走 sync build。
#[test]
fn plugin_install_does_not_use_build_async_on_start_async() {
    let via_async = Rc::new(RefCell::new(false));
    let via_sync = Rc::new(RefCell::new(false));

    struct Marked {
        via_async: Rc<RefCell<bool>>,
        via_sync: Rc<RefCell<bool>>,
    }
    impl Plugin for Marked {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            *self.via_sync.borrow_mut() = true;
            Ok(())
        }
    }
    #[async_trait(?Send)]
    impl AsyncPlugin for Marked {
        async fn build_async(&self, _ctx: &mut Context) -> Result<(), Error> {
            *self.via_async.borrow_mut() = true;
            Ok(())
        }
    }

    let ctx = Context::new();
    ctx.plugin(Marked {
        via_async: Rc::clone(&via_async),
        via_sync: Rc::clone(&via_sync),
    })
    .expect("install via plugin");
    block_on(ctx.start_async()).expect("start_async");
    assert!(*via_sync.borrow());
    assert!(!*via_async.borrow());
}
