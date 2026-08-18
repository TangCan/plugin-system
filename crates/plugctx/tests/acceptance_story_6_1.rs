#![cfg(all(feature = "stages", not(feature = "thread-safe")))]

//! Acceptance tests for story 6.1 — Custom Stages Init / PostStart / PreDispose（ATDD）。
//!
//! Red phase: 缺 `stages` feature / 类型 / 约定节点 emit 时编译或断言失败。

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use plugctx::{
    Context, DisposeEvent, Error, InitEvent, Plugin, PostStartEvent, PreDisposeEvent, ReadyEvent,
};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("plugin-system root")
        .to_path_buf()
}

fn read_utf8(rel: &str) -> String {
    let path = workspace_root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// AC#2 / NFR5：`stages` 为可选 feature，不得进入 default。
#[test]
fn cargo_toml_stages_is_optional_zero_deps() {
    let cargo = read_utf8("crates/plugctx/Cargo.toml");
    assert!(
        cargo.contains("stages = []") || cargo.contains("stages=[]"),
        "plugctx must declare stages feature with no extra deps"
    );
    assert!(
        cargo.contains("default = []"),
        "default features must remain empty"
    );
    assert!(
        !cargo.contains("default = [\"stages\"]"),
        "stages must not be in default features"
    );
}

/// AC#1：成功 start 顺序 Init → Ready → PostStart，各恰好一次。
#[test]
fn start_emits_init_ready_poststart_in_order() {
    let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    let ctx = Context::new();

    let o = Rc::clone(&order);
    ctx.on(move |_: &InitEvent| o.borrow_mut().push("init"));
    let o = Rc::clone(&order);
    ctx.on(move |_: &ReadyEvent| o.borrow_mut().push("ready"));
    let o = Rc::clone(&order);
    ctx.on(move |_: &PostStartEvent| o.borrow_mut().push("poststart"));

    ctx.start().expect("start");
    assert_eq!(
        *order.borrow(),
        vec!["init", "ready", "poststart"],
        "stage order must be Init → Ready → PostStart"
    );

    let err = ctx.start().expect_err("second start");
    assert!(matches!(err, Error::AlreadyStarted));
    assert_eq!(
        *order.borrow(),
        vec!["init", "ready", "poststart"],
        "AlreadyStarted must not re-emit stage events"
    );
}

/// AC#1：构建失败时 Init 已发；Ready/PostStart 不发；未 Started。
#[test]
fn failed_start_emits_init_but_not_ready_or_poststart() {
    struct NeedsMissing;
    impl Plugin for NeedsMissing {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            Ok(())
        }
        fn dependencies(&self) -> Vec<std::any::TypeId> {
            vec![std::any::TypeId::of::<u64>()]
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
    let err = ctx.start().expect_err("missing dep");
    assert!(matches!(err, Error::MissingDependency));
    assert_eq!(init.get(), 1, "Init fires before build");
    assert_eq!(ready.get(), 0);
    assert_eq!(post.get(), 0);
    assert!(!ctx.is_started());
}

/// AC#1：dispose 顺序 PreDispose → Dispose → effect cleanup。
#[test]
fn dispose_emits_predispose_before_dispose_before_effects() {
    let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    let ctx = Context::new();

    let o = Rc::clone(&order);
    ctx.on(move |_: &PreDisposeEvent| o.borrow_mut().push("predispose"));
    let o = Rc::clone(&order);
    ctx.on(move |_: &DisposeEvent| o.borrow_mut().push("dispose"));
    let o = Rc::clone(&order);
    ctx.effect(move || {
        let o = Rc::clone(&o);
        move || {
            o.borrow_mut().push("effect-cleanup");
        }
    });

    ctx.dispose();
    assert_eq!(
        *order.borrow(),
        vec!["predispose", "dispose", "effect-cleanup"]
    );

    ctx.dispose();
    assert_eq!(
        *order.borrow(),
        vec!["predispose", "dispose", "effect-cleanup"],
        "idempotent dispose must not re-emit"
    );
}

/// AC#1：未 start 直接 dispose 仍触发 PreDispose → Dispose。
#[test]
fn dispose_without_start_still_emits_predispose_and_dispose() {
    let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    let ctx = Context::new();
    let o = Rc::clone(&order);
    ctx.on(move |_: &PreDisposeEvent| o.borrow_mut().push("predispose"));
    let o = Rc::clone(&order);
    ctx.on(move |_: &DisposeEvent| o.borrow_mut().push("dispose"));
    ctx.dispose();
    assert_eq!(*order.borrow(), vec!["predispose", "dispose"]);
}

/// Automate：PreDispose 重入 dispose / on / emit 不 panic；外层仍只触发一次。
#[test]
fn predispose_event_reentrancy_no_panic() {
    let hits = Rc::new(Cell::new(0u32));
    let ctx = Context::new();
    let h = Rc::clone(&hits);
    let ctx_inner = ctx.clone();
    ctx.on(move |_: &PreDisposeEvent| {
        h.set(h.get() + 1);
        ctx_inner.dispose();
        ctx_inner.on(|_: &PreDisposeEvent| {});
        ctx_inner.emit(&PreDisposeEvent);
    });
    ctx.dispose();
    assert_eq!(hits.get(), 1);
    assert!(ctx.is_disposed());
}

/// AC#2：中文文档说明 stages 与缺省行为。
#[test]
fn chinese_docs_describe_stages_and_core_guarantee() {
    let readme = read_utf8("README.md");
    assert!(
        readme.contains("stages") && (readme.contains("InitEvent") || readme.contains("Init")),
        "README must document stages feature"
    );

    let api = read_utf8("docs/requirements/6. API 设计概览.md");
    assert!(
        api.contains("InitEvent") && api.contains("stages"),
        "§6.4.3 must document stages / InitEvent"
    );

    let ext = read_utf8("docs/requirements/4. 扩展模块设计.md");
    assert!(
        ext.contains("4.7") && ext.contains("stages"),
        "§4.7 must name stages feature"
    );
}
