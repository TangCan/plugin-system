#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 5.5 — proptest 属性测试（随机操作序列）（ATDD / FR38）。
//!
//! 红相：缺少 `proptest` dev-dep 或文档未声明属性测试交付时失败；
//! 绿相：随机 install/unload/emit/start/dispose 序列下不变量成立（NFR3 / NFR9）。

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

use plugctx::{Context, Error, Plugin, PluginHandle};
use proptest::prelude::*;
use proptest::test_runner::Config as ProptestConfig;

#[derive(Debug, Clone)]
struct Ping;

/// 可安装的简单插件：登记 effect（计数 cleanup）与可选重入监听。
struct SeqPlugin {
    tag: u8,
    cleanups: Rc<Cell<u32>>,
    setups: Rc<Cell<u32>>,
    /// 全序列共享：限制重入深度，避免多监听器互相 emit 导致栈溢出。
    emit_depth: Rc<Cell<u32>>,
    reentrant: bool,
}

impl Plugin for SeqPlugin {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        self.setups.set(self.setups.get() + 1);
        let cleanups = Rc::clone(&self.cleanups);
        let _ = ctx.effect(move || {
            let cleanups = Rc::clone(&cleanups);
            move || {
                cleanups.set(cleanups.get() + 1);
            }
        });

        if self.reentrant {
            let nested = ctx.clone();
            let depth = Rc::clone(&self.emit_depth);
            let _ = ctx.on(move |_: &Ping| {
                // 重入 emit：不得因借用冲突 panic（NFR9 / FR38）。
                if depth.get() >= 1 {
                    return;
                }
                depth.set(1);
                nested.emit(&Ping);
                depth.set(0);
            });
        } else {
            let _ = ctx.on(move |_: &Ping| {
                // 普通监听：存在即可；dispose 后由终态不变量覆盖。
            });
        }

        let _ = self.tag;
        Ok(())
    }
}

#[derive(Debug, Clone)]
enum Op {
    /// 安装普通插件（带 effect）。
    Install,
    /// 安装带重入监听的插件。
    InstallReentrant,
    /// 按索引卸载仍存活的插件。
    Unload(proptest::sample::Index),
    /// 触发事件（可能重入）。
    Emit,
    /// 尝试 start（合法/非法状态均不得 panic）。
    Start,
    /// 销毁整个上下文。
    DisposeContext,
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => Just(Op::Install),
        1 => Just(Op::InstallReentrant),
        3 => any::<proptest::sample::Index>().prop_map(Op::Unload),
        3 => Just(Op::Emit),
        2 => Just(Op::Start),
        1 => Just(Op::DisposeContext),
    ]
}

fn run_sequence(ops: &[Op]) {
    let setups = Rc::new(Cell::new(0u32));
    let cleanups = Rc::new(Cell::new(0u32));
    let emit_depth = Rc::new(Cell::new(0u32));
    let ctx = Context::new();
    let mut handles: Vec<PluginHandle> = Vec::new();
    let mut next_tag: u8 = 0;
    let mut context_disposed = false;

    for op in ops {
        if context_disposed {
            // dispose 后：状态合法；操作须返回明确错误或不 panic，不以 panic 作控制流。
            match op {
                Op::Install | Op::InstallReentrant => {
                    let p = SeqPlugin {
                        tag: next_tag,
                        cleanups: Rc::clone(&cleanups),
                        setups: Rc::clone(&setups),
                        emit_depth: Rc::clone(&emit_depth),
                        reentrant: matches!(op, Op::InstallReentrant),
                    };
                    next_tag = next_tag.wrapping_add(1);
                    let err = ctx.plugin(p).expect_err("install after dispose");
                    assert!(
                        matches!(err, Error::AlreadyDisposed),
                        "expected AlreadyDisposed, got {err:?}"
                    );
                }
                Op::Unload(_) => {
                    for h in &handles {
                        let _ = h.dispose(); // 不 panic
                        assert!(!h.is_alive());
                    }
                }
                Op::Emit => {
                    ctx.emit(&Ping); // 不 panic
                }
                Op::Start => {
                    let err = ctx.start().expect_err("start after dispose");
                    assert!(matches!(err, Error::AlreadyDisposed));
                }
                Op::DisposeContext => {
                    ctx.dispose(); // 幂等
                }
            }
            assert!(ctx.is_disposed());
            continue;
        }

        match op {
            Op::Install | Op::InstallReentrant => {
                let reentrant = matches!(op, Op::InstallReentrant);
                let p = SeqPlugin {
                    tag: next_tag,
                    cleanups: Rc::clone(&cleanups),
                    setups: Rc::clone(&setups),
                    emit_depth: Rc::clone(&emit_depth),
                    reentrant,
                };
                next_tag = next_tag.wrapping_add(1);
                match ctx.plugin(p) {
                    Ok(h) => handles.push(h),
                    Err(e) => panic!("unexpected install error: {e:?}"),
                }
            }
            Op::Unload(idx) => {
                let alive: Vec<usize> = handles
                    .iter()
                    .enumerate()
                    .filter(|(_, h)| h.is_alive())
                    .map(|(i, _)| i)
                    .collect();
                if alive.is_empty() {
                    continue;
                }
                let i = alive[idx.index(alive.len())];
                match handles[i].dispose() {
                    Ok(()) => {}
                    Err(Error::PluginAlreadyDisposed) => {}
                    Err(e) => panic!("unexpected unload error: {e:?}"),
                }
                assert!(!handles[i].is_alive());
            }
            Op::Emit => {
                ctx.emit(&Ping);
            }
            Op::Start => match ctx.start() {
                Ok(()) => {
                    assert!(ctx.is_started());
                    assert!(!ctx.is_disposed());
                }
                Err(Error::AlreadyStarted) => {
                    assert!(ctx.is_started());
                }
                Err(Error::AlreadyDisposed) => {
                    assert!(ctx.is_disposed());
                    context_disposed = true;
                }
                Err(e) => panic!("unexpected start error: {e:?}"),
            },
            Op::DisposeContext => {
                ctx.dispose();
                context_disposed = true;
                assert!(ctx.is_disposed());
            }
        }

        // 进行中不变量：started/disposed 互斥语义（disposed 后 started 标志可保留历史值，但 is_disposed 优先）。
        if ctx.is_disposed() {
            context_disposed = true;
        }
    }

    if !ctx.is_disposed() {
        ctx.dispose();
    }

    // —— 终态不变量（FR38 / NFR3 / NFR9）——
    assert!(ctx.is_disposed(), "context must be disposed after sequence");
    for h in &handles {
        assert!(
            !h.is_alive(),
            "no dangling plugin registration after dispose"
        );
    }

    let setup_n = setups.get();
    let cleanup_n = cleanups.get();
    assert_eq!(
        cleanup_n, setup_n,
        "each built plugin effect cleanup must run exactly once (setups={setup_n}, cleanups={cleanup_n})"
    );

    // dispose 后 emit 不得 panic；无悬挂监听副作用要求至少不崩溃。
    ctx.emit(&Ping);
    ctx.dispose();
}

/// AC#1: 随机操作序列下 dispose 后无悬挂注册、状态合法、effect 恰一次、重入不 panic。
#[test]
fn prop_random_install_unload_emit_invariants() {
    let mut config = ProptestConfig::with_cases(64);
    // CI 友好：限制复杂度，避免默认超时。
    config.max_shrink_iters = 10_000;

    let mut runner = proptest::test_runner::TestRunner::new(config);
    runner
        .run(&proptest::collection::vec(op_strategy(), 1..=24), |ops| {
            run_sequence(&ops);
            Ok(())
        })
        .unwrap();
}

/// AC#1 护栏：重复 dispose / 重入 emit 的确定性小用例（缩小反例时的基线）。
#[test]
fn deterministic_reentrant_emit_then_dispose_no_panic() {
    let setups = Rc::new(Cell::new(0u32));
    let cleanups = Rc::new(Cell::new(0u32));
    let ctx = Context::new();
    let h = ctx
        .plugin(SeqPlugin {
            tag: 0,
            cleanups: Rc::clone(&cleanups),
            setups: Rc::clone(&setups),
            emit_depth: Rc::new(Cell::new(0)),
            reentrant: true,
        })
        .expect("install");
    ctx.start().expect("start");
    ctx.emit(&Ping);
    h.dispose().expect("unload");
    assert!(!h.is_alive());
    ctx.emit(&Ping);
    ctx.dispose();
    assert!(ctx.is_disposed());
    assert_eq!(cleanups.get(), setups.get());
    assert_eq!(cleanups.get(), 1);
}

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

/// 文档：属性测试已交付并给出命令（NFR8）。
#[test]
fn testing_doc_documents_proptest_story_5_5() {
    let doc = read_utf8("docs/testing.md");
    for needle in ["proptest", "acceptance_story_5_5", "属性", "随机", "不变量"] {
        assert!(
            doc.contains(needle),
            "docs/testing.md must mention `{needle}` for Story 5.5"
        );
    }
    assert!(
        !doc.contains("Story **5.5**（本故事不阻塞）"),
        "docs/testing.md must mark Story 5.5 as delivered, not blocked"
    );
}

/// Cargo：proptest 仅为 plugctx 的 dev-dependency。
#[test]
fn cargo_toml_proptest_is_dev_dependency_only() {
    let cargo = read_utf8("crates/plugctx/Cargo.toml");
    assert!(
        cargo.contains("proptest"),
        "plugctx Cargo.toml must declare proptest"
    );
    // 粗护栏：出现在 [dev-dependencies] 段落后、下一 [ 段前。
    let dev = cargo
        .split("[dev-dependencies]")
        .nth(1)
        .expect("[dev-dependencies] section");
    let before_next = dev.split("\n[").next().unwrap_or(dev);
    assert!(
        before_next.contains("proptest"),
        "proptest must be under [dev-dependencies]"
    );

    let ws = read_utf8("Cargo.toml");
    assert!(
        ws.contains("proptest"),
        "workspace Cargo.toml must declare proptest"
    );
}

/// Automate：多插件先精确卸载再 Context::dispose，cleanup 仍恰一次且无悬挂。
#[test]
fn unload_all_plugins_then_dispose_cleanup_once() {
    let setups = Rc::new(Cell::new(0u32));
    let cleanups = Rc::new(Cell::new(0u32));
    let emit_depth = Rc::new(Cell::new(0u32));
    let ctx = Context::new();

    let mut handles = Vec::new();
    for tag in 0..5u8 {
        let h = ctx
            .plugin(SeqPlugin {
                tag,
                cleanups: Rc::clone(&cleanups),
                setups: Rc::clone(&setups),
                emit_depth: Rc::clone(&emit_depth),
                reentrant: tag == 2,
            })
            .expect("install");
        handles.push(h);
    }
    ctx.start().expect("start");
    assert_eq!(setups.get(), 5);

    ctx.emit(&Ping);

    for h in &handles {
        h.dispose().expect("unload");
        assert!(!h.is_alive());
    }
    assert_eq!(cleanups.get(), 5, "plugin dispose must run each cleanup");

    ctx.emit(&Ping); // 无悬挂监听副作用 / 不 panic
    ctx.dispose();
    assert!(ctx.is_disposed());
    assert_eq!(
        cleanups.get(),
        setups.get(),
        "context dispose must not double-run already cleaned effects"
    );
}

/// Automate：README 回归门禁提及属性测试入口。
#[test]
fn readme_mentions_proptest_acceptance() {
    let readme = read_utf8("README.md");
    assert!(
        readme.contains("acceptance_story_5_5") && readme.contains("proptest"),
        "README must point to Story 5.5 proptest acceptance"
    );
}
