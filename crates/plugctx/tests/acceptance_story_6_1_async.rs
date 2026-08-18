#![cfg(all(feature = "async", feature = "stages", not(feature = "thread-safe")))]

//! Acceptance tests for epic-6 retro item-10 — async + stages `start_async` 阶段顺序与失败路径。
//!
//! 对称 [`acceptance_story_6_1`]（同步 `start`）；需：
//! `cargo test -p plugctx --features "async,stages" --test acceptance_story_6_1_async`

use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use futures::executor::block_on;
use plugctx::{
    AsyncPlugin, Context, DisposeEvent, Error, InitEvent, Plugin, PostStartEvent, PreDisposeEvent,
    ReadyEvent,
};

/// 成功 `start_async`：Init → Ready → PostStart，各恰好一次。
#[test]
fn start_async_emits_init_ready_poststart_in_order() {
    let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    let ctx = Context::new();

    let o = Rc::clone(&order);
    ctx.on(move |_: &InitEvent| o.borrow_mut().push("init"));
    let o = Rc::clone(&order);
    ctx.on(move |_: &ReadyEvent| o.borrow_mut().push("ready"));
    let o = Rc::clone(&order);
    ctx.on(move |_: &PostStartEvent| o.borrow_mut().push("poststart"));

    block_on(async {
        ctx.start_async().await.expect("start_async");
    });
    assert_eq!(
        *order.borrow(),
        vec!["init", "ready", "poststart"],
        "async stage order must match sync: Init → Ready → PostStart"
    );

    let err = block_on(async { ctx.start_async().await.expect_err("second start_async") });
    assert!(matches!(err, Error::AlreadyStarted));
    assert_eq!(
        *order.borrow(),
        vec!["init", "ready", "poststart"],
        "AlreadyStarted must not re-emit stage events"
    );
}

/// 构建失败：Init 已发；Ready/PostStart 不发；未 Started。
#[test]
fn failed_start_async_emits_init_but_not_ready_or_poststart() {
    struct NeedsMissing;
    impl Plugin for NeedsMissing {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            Ok(())
        }
        fn dependencies(&self) -> Vec<TypeId> {
            vec![TypeId::of::<u64>()]
        }
    }

    let init = Rc::new(Cell::new(0u32));
    let ready = Rc::new(Cell::new(0u32));
    let post = Rc::new(Cell::new(0u32));
    let ctx = Context::new();

    let i = Rc::clone(&init);
    ctx.on(move |_: &InitEvent| i.set(i.get() + 1));
    let r = Rc::clone(&ready);
    ctx.on(move |_: &ReadyEvent| r.set(r.get() + 1));
    let p = Rc::clone(&post);
    ctx.on(move |_: &PostStartEvent| p.set(p.get() + 1));

    ctx.plugin(NeedsMissing).expect("install");
    let err = block_on(async { ctx.start_async().await.expect_err("missing dep") });
    assert!(matches!(err, Error::MissingDependency));
    assert_eq!(init.get(), 1, "Init fires before async build");
    assert_eq!(ready.get(), 0);
    assert_eq!(post.get(), 0);
    assert!(!ctx.is_started());
}

/// 异步插件 `build_async` 失败路径同样：Init 已发，Ready/PostStart 不发。
#[test]
fn failed_async_plugin_build_skips_ready_and_poststart() {
    use async_trait::async_trait;

    struct FailingAsync;
    impl Plugin for FailingAsync {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            unreachable!("start_async must use build_async for plugin_async entries")
        }
    }
    #[async_trait(?Send)]
    impl AsyncPlugin for FailingAsync {
        async fn build_async(&self, _ctx: &mut Context) -> Result<(), Error> {
            Err(Error::BuildFailed)
        }
    }

    let init = Rc::new(Cell::new(0u32));
    let ready = Rc::new(Cell::new(0u32));
    let post = Rc::new(Cell::new(0u32));
    let ctx = Context::new();

    let i = Rc::clone(&init);
    ctx.on(move |_: &InitEvent| i.set(i.get() + 1));
    let r = Rc::clone(&ready);
    ctx.on(move |_: &ReadyEvent| r.set(r.get() + 1));
    let p = Rc::clone(&post);
    ctx.on(move |_: &PostStartEvent| p.set(p.get() + 1));

    ctx.plugin_async(FailingAsync).expect("install async");
    let err = block_on(async { ctx.start_async().await.expect_err("build failed") });
    assert!(matches!(err, Error::BuildFailed));
    assert_eq!(init.get(), 1);
    assert_eq!(ready.get(), 0);
    assert_eq!(post.get(), 0);
    assert!(!ctx.is_started());
}

/// dispose 顺序在 async+stages 下仍为 PreDispose → Dispose（与 6.1 对称）。
#[test]
fn dispose_with_async_stages_emits_predispose_before_dispose() {
    let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    let ctx = Context::new();
    let o = Rc::clone(&order);
    ctx.on(move |_: &PreDisposeEvent| o.borrow_mut().push("predispose"));
    let o = Rc::clone(&order);
    ctx.on(move |_: &DisposeEvent| o.borrow_mut().push("dispose"));
    block_on(async {
        ctx.start_async().await.expect("start_async");
    });
    ctx.dispose();
    assert_eq!(*order.borrow(), vec!["predispose", "dispose"]);
}
