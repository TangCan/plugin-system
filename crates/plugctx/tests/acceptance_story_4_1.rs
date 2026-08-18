//! Acceptance tests for story 4.1 — thread-safe Context 存储（ATDD）。
//!
//! 需启用 feature：`cargo test -p plugctx --features thread-safe --test acceptance_story_4_1`

use std::sync::{Arc, Barrier, Mutex};
use std::thread;

use plugctx::{Context, Plugin, PluginHandle};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Counter(u32);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Ping(u32);

struct CounterPlugin;

impl Plugin for CounterPlugin {
    fn build(&self, ctx: &mut Context) -> Result<(), plugctx::Error> {
        ctx.provide(Counter(0));
        Ok(())
    }
}

/// AC#1: thread-safe 下 Context 为 Send + Sync，可跨线程共享。
#[test]
fn context_is_send_sync_when_thread_safe() {
    fn assert_send_sync<T: Send + Sync>(_: &T) {}
    let ctx = Context::new();
    assert_send_sync(&ctx);
}

/// AC#1: 跨线程 provide / get 无数据竞争。
#[test]
fn cross_thread_provide_and_get() {
    let ctx = Context::new();
    ctx.provide(Counter(1));

    let ctx2 = ctx.clone();
    let handle = thread::spawn(move || {
        assert_eq!(*ctx2.get::<Counter>().expect("counter"), Counter(1));
        ctx2.provide(Counter(2));
        assert_eq!(*ctx2.get::<Counter>().expect("counter"), Counter(2));
    });
    handle.join().expect("worker");
    assert_eq!(*ctx.get::<Counter>().expect("counter"), Counter(2));
}

/// AC#3: 多线程并发 emit；监听器全部执行完毕且无 panic。
#[test]
fn concurrent_emit_from_multiple_threads() {
    let ctx = Context::new();
    let hits = Arc::new(Mutex::new(0u32));
    let hits_reg = Arc::clone(&hits);
    ctx.on(move |_e: &Ping| {
        let mut g = hits_reg.lock().expect("hits");
        *g += 1;
    });

    let barrier = Arc::new(Barrier::new(4));
    let mut joins = Vec::new();
    for i in 0..4 {
        let ctx = ctx.clone();
        let barrier = Arc::clone(&barrier);
        joins.push(thread::spawn(move || {
            barrier.wait();
            ctx.emit(&Ping(i));
        }));
    }
    for j in joins {
        j.join().expect("emitter");
    }
    assert_eq!(*hits.lock().expect("hits"), 4);
}

/// AC#1: 插件安装与 start 可在线程间传递 Context。
#[test]
fn plugin_start_across_threads() {
    let ctx = Context::new();
    let ctx_worker = ctx.clone();
    let handle: PluginHandle =
        thread::spawn(move || ctx_worker.plugin(CounterPlugin).expect("install"))
            .join()
            .expect("install thread");

    ctx.start().expect("start");
    assert!(handle.is_alive());
    assert_eq!(*ctx.get::<Counter>().expect("counter"), Counter(0));
}

/// AC#3 护栏: 持有 get guard 时文档约定勿再写；此处验证释放后可写。
#[test]
fn get_guard_released_allows_provide() {
    let ctx = Context::new();
    ctx.provide(Counter(1));
    {
        let r = ctx.get::<Counter>().expect("r");
        assert_eq!(*r, Counter(1));
    }
    ctx.provide(Counter(9));
    assert_eq!(*ctx.get::<Counter>().expect("r"), Counter(9));
}

/// Automate 护栏: 默认同步语义在 thread-safe 下仍可用（单线程路径）。
#[test]
fn thread_safe_start_emit_ready_still_works() {
    let ctx = Context::new();
    let hits = Arc::new(Mutex::new(0u32));
    let hits2 = Arc::clone(&hits);
    ctx.on(move |_e: &plugctx::ReadyEvent| {
        *hits2.lock().expect("hits") += 1;
    });
    ctx.plugin(CounterPlugin).expect("install");
    ctx.start().expect("start");
    assert_eq!(*hits.lock().expect("hits"), 1);
    assert_eq!(*ctx.get::<Counter>().expect("c"), Counter(0));
}
