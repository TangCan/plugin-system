//! 核心路径基准（Story 5.7 / FR40）
//!
//! 覆盖：
//! - `get`：服务查找
//! - `emit`：事件 fan-out（有界监听器数量）
//! - `start`：小规模插件依赖构建
//!
//! 运行（在 `plugin-system/`）：
//! ```bash
//! cargo bench -p plugctx --bench core_paths
//! # 保存 / 对比基线：
//! cargo bench -p plugctx --bench core_paths -- --save-baseline main
//! cargo bench -p plugctx --bench core_paths -- --baseline main
//! ```
//!
//! 默认 CI 仅 `cargo bench --no-run` 防腐化，不跑满量迭代。

use std::any::TypeId;
use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use plugctx::{Context, Error, Plugin};

#[derive(Debug, Clone, Copy)]
struct BenchService(u64);

#[derive(Debug)]
struct BenchEvent(u64);

struct ProvidePlugin;

impl Plugin for ProvidePlugin {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        ctx.provide(BenchService(42));
        Ok(())
    }
}

struct DependsPlugin;

impl Plugin for DependsPlugin {
    fn dependencies(&self) -> Vec<TypeId> {
        vec![TypeId::of::<BenchService>()]
    }

    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        let s = ctx
            .get::<BenchService>()
            .expect("BenchService provided by ProvidePlugin");
        black_box(s.0);
        Ok(())
    }
}

/// 轻量插件：无依赖，用于扩大 start 构建规模。
struct LeafPlugin(u8);

impl Plugin for LeafPlugin {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        // 避免空 build 被完全优化掉语义；提供可区分类型较重，改用副作用最小的 no-op + black_box
        black_box(self.0);
        let _ = ctx;
        Ok(())
    }
}

fn setup_started_with_service() -> Context {
    let ctx = Context::new();
    ctx.plugin(ProvidePlugin).expect("install ProvidePlugin");
    ctx.start().expect("start");
    ctx
}

fn bench_core_get(c: &mut Criterion) {
    let ctx = setup_started_with_service();
    c.bench_function("core_get", |b| {
        b.iter(|| {
            let s = ctx.get::<BenchService>().expect("service present");
            black_box(s.0)
        })
    });
    ctx.dispose();
}

fn bench_core_emit(c: &mut Criterion) {
    let ctx = setup_started_with_service();
    // 有界监听器：设计 §8.8 写 100；默认 bench 用 16 以控制本地/可选长跑时长。
    const LISTENERS: usize = 16;
    for _ in 0..LISTENERS {
        ctx.on(|e: &BenchEvent| {
            black_box(e.0);
        });
    }
    c.bench_function("core_emit", |b| {
        b.iter(|| {
            ctx.emit(black_box(&BenchEvent(1)));
        })
    });
    ctx.dispose();
}

fn bench_core_start(c: &mut Criterion) {
    c.bench_function("core_start", |b| {
        b.iter(|| {
            let ctx = Context::new();
            ctx.plugin(ProvidePlugin).expect("ProvidePlugin");
            ctx.plugin(DependsPlugin).expect("DependsPlugin");
            // 小规模：再挂几个无依赖叶子插件
            for i in 0u8..4 {
                ctx.plugin(LeafPlugin(i)).expect("LeafPlugin");
            }
            ctx.start().expect("start");
            ctx.dispose();
            black_box(())
        })
    });
}

criterion_group!(
    core_paths,
    bench_core_get,
    bench_core_emit,
    bench_core_start
);
criterion_main!(core_paths);
