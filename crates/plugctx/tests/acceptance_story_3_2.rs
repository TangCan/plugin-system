#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 3.2 — emit_parallel 宿主侧并行 fan-out（ATDD）。
//!
//! 需启用 feature：`cargo test -p plugctx --features parallel --test acceptance_story_3_2`
//!
//! 使用 `futures::executor::block_on` 驱动，避免绑定单一运行时（同 3.1 / NFR1）。

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context as TaskContext, Poll};

use futures::executor::block_on;
use plugctx::Context;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParallelEvent(u32);

#[derive(Clone, Debug, PartialEq, Eq)]
struct SyncOnlyEvent(&'static str);

/// 单次让出，使 `join_all` 能轮询其它 Future（宿主侧并发）。
fn yield_once() -> impl Future<Output = ()> {
    YieldOnce { yielded: false }
}

struct YieldOnce {
    yielded: bool,
}

impl Future for YieldOnce {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<()> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

/// AC#2: 多个异步监听器经 `emit_parallel` 全部完成；宿主侧 `join_all` 可重叠。
#[test]
fn emit_parallel_runs_async_listeners_concurrently_to_completion() {
    let in_flight = Rc::new(Cell::new(0u32));
    let max_in_flight = Rc::new(Cell::new(0u32));
    let done = Rc::new(Cell::new(0u32));

    let ctx = Context::new();

    for _ in 0..3 {
        let in_flight = Rc::clone(&in_flight);
        let max_in_flight = Rc::clone(&max_in_flight);
        let done = Rc::clone(&done);
        ctx.on_async(move |_e: ParallelEvent| {
            let in_flight = Rc::clone(&in_flight);
            let max_in_flight = Rc::clone(&max_in_flight);
            let done = Rc::clone(&done);
            async move {
                let n = in_flight.get() + 1;
                in_flight.set(n);
                if n > max_in_flight.get() {
                    max_in_flight.set(n);
                }
                yield_once().await;
                in_flight.set(in_flight.get() - 1);
                done.set(done.get() + 1);
            }
        });
    }

    block_on(async {
        ctx.emit_parallel(&ParallelEvent(1))
            .await
            .expect("emit_parallel");
    });

    assert_eq!(done.get(), 3, "all async listeners must complete");
    assert!(
        max_in_flight.get() >= 2,
        "host-side fan-out should overlap (max in-flight >= 2), got {}",
        max_in_flight.get()
    );
}

/// AC#3: 同步 `emit` 仍按注册序串行；`on_async` 监听器不被同步 emit 调用。
#[test]
fn sync_emit_still_runs_in_registration_order() {
    let order = Rc::new(RefCell::new(Vec::<u8>::new()));
    let ctx = Context::new();

    for i in 0u8..3 {
        let order = Rc::clone(&order);
        ctx.on::<SyncOnlyEvent>(move |_e| {
            order.borrow_mut().push(i);
        });
    }

    let async_hits = Rc::new(Cell::new(0u32));
    let hits = Rc::clone(&async_hits);
    ctx.on_async(move |_e: SyncOnlyEvent| {
        let hits = Rc::clone(&hits);
        async move {
            hits.set(hits.get() + 1);
        }
    });

    ctx.emit(&SyncOnlyEvent("x"));

    assert_eq!(order.borrow().as_slice(), &[0, 1, 2]);
    assert_eq!(
        async_hits.get(),
        0,
        "sync emit must not invoke on_async listeners"
    );
}

/// AC#2: cancel 后 emit_parallel 跳过该异步监听器。
#[test]
fn emit_parallel_skips_cancelled_async_listener() {
    let hits = Rc::new(Cell::new(0u32));
    let ctx = Context::new();

    let h1 = {
        let hits = Rc::clone(&hits);
        ctx.on_async(move |_e: ParallelEvent| {
            let hits = Rc::clone(&hits);
            async move {
                hits.set(hits.get() + 1);
            }
        })
    };
    {
        let hits = Rc::clone(&hits);
        ctx.on_async(move |_e: ParallelEvent| {
            let hits = Rc::clone(&hits);
            async move {
                hits.set(hits.get() + 10);
            }
        });
    }
    h1.cancel();

    block_on(async {
        ctx.emit_parallel(&ParallelEvent(7)).await.expect("ok");
    });

    assert_eq!(hits.get(), 10);
}

/// AC#2: 无异步监听器时 emit_parallel 为 Ok no-op；同步监听器不被调用。
#[test]
fn emit_parallel_noop_without_async_listeners() {
    let ctx = Context::new();
    ctx.on::<ParallelEvent>(|_e| {
        panic!("sync listener must not run on emit_parallel");
    });
    block_on(async {
        ctx.emit_parallel(&ParallelEvent(0)).await.expect("noop ok");
    });
}

/// AC#1 / NFR5: parallel API 仅在 feature 启用时可用（本文件 required-features=["parallel"]）。
#[test]
fn parallel_feature_exposes_emit_parallel_api() {
    let ctx = Context::new();
    block_on(async {
        let _ = ctx.emit_parallel(&ParallelEvent(42)).await;
    });
}

/// Automate: emit_parallel 整次 fan-out 前后调用拦截器（与同步 emit 一致）。
#[test]
fn emit_parallel_invokes_interceptors_around_fanout() {
    use plugctx::ContextInterceptor;
    use std::any::Any;

    let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));

    struct LogInterceptor {
        log: Rc<RefCell<Vec<&'static str>>>,
    }
    impl ContextInterceptor for LogInterceptor {
        fn before_emit(&self, _event: &dyn Any) {
            self.log.borrow_mut().push("before");
        }
        fn after_emit(&self, _event: &dyn Any) {
            self.log.borrow_mut().push("after");
        }
    }

    let ctx = Context::new();
    ctx.add_interceptor(LogInterceptor {
        log: Rc::clone(&log),
    });
    let log2 = Rc::clone(&log);
    ctx.on_async(move |_e: ParallelEvent| {
        let log2 = Rc::clone(&log2);
        async move {
            log2.borrow_mut().push("listener");
        }
    });

    block_on(async {
        ctx.emit_parallel(&ParallelEvent(1)).await.expect("ok");
    });

    assert_eq!(log.borrow().as_slice(), &["before", "listener", "after"]);
}

/// Automate: 同步 on 与 on_async 分轨——emit_parallel 不调用同步监听器。
#[test]
fn emit_parallel_does_not_invoke_sync_on_listeners() {
    let sync_hits = Rc::new(Cell::new(0u32));
    let async_hits = Rc::new(Cell::new(0u32));
    let ctx = Context::new();

    let s = Rc::clone(&sync_hits);
    ctx.on::<ParallelEvent>(move |_e| {
        s.set(s.get() + 1);
    });
    let a = Rc::clone(&async_hits);
    ctx.on_async(move |_e: ParallelEvent| {
        let a = Rc::clone(&a);
        async move {
            a.set(a.get() + 1);
        }
    });

    block_on(async {
        ctx.emit_parallel(&ParallelEvent(9)).await.expect("ok");
    });

    assert_eq!(sync_hits.get(), 0);
    assert_eq!(async_hits.get(), 1);
}

/// Retro item-9: 插件 build 内 on_async 后 PluginHandle::dispose，emit_parallel 不再触发；
/// 同事件多插件交错下标 fixup 后兄弟监听仍正确。
#[test]
fn dispose_plugin_removes_on_async_and_fixups_interleaved_indices() {
    use plugctx::{Error, Plugin};

    let hits = Rc::new(Cell::new(0u32));

    struct PluginEarly {
        hits: Rc<Cell<u32>>,
    }
    impl Plugin for PluginEarly {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            let hits = Rc::clone(&self.hits);
            let _ = ctx.on_async(move |_e: ParallelEvent| {
                let hits = Rc::clone(&hits);
                async move {
                    hits.set(hits.get() + 1);
                }
            });
            Ok(())
        }
    }

    struct PluginLate {
        hits: Rc<Cell<u32>>,
    }
    impl Plugin for PluginLate {
        fn build(&self, ctx: &mut Context) -> Result<(), Error> {
            let hits = Rc::clone(&self.hits);
            let _ = ctx.on_async(move |_e: ParallelEvent| {
                let hits = Rc::clone(&hits);
                async move {
                    hits.set(hits.get() + 10);
                }
            });
            Ok(())
        }
    }

    let ctx = Context::new();
    let early = ctx
        .plugin(PluginEarly {
            hits: Rc::clone(&hits),
        })
        .expect("early");
    let _late = ctx
        .plugin(PluginLate {
            hits: Rc::clone(&hits),
        })
        .expect("late");
    ctx.start().expect("start");

    early.dispose().expect("dispose early plugin");

    block_on(async {
        ctx.emit_parallel(&ParallelEvent(1)).await.expect("ok");
    });

    assert_eq!(
        hits.get(),
        10,
        "disposed plugin on_async must not fire; interleaved late listener (+10) must remain"
    );
}
