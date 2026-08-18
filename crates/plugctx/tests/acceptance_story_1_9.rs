#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 1.9 — slotmap（或等价）稳定插件 ID（ATDD）。
//!
//! Red phase: 在 SlotMap / 稳定键落地前应编译失败或断言失败。

use std::cell::Cell;
use std::rc::Rc;

use plugctx::{Context, Error, Plugin, PluginId};

struct CountingPlugin {
    builds: Rc<Cell<u32>>,
}

impl Plugin for CountingPlugin {
    fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
        self.builds.set(self.builds.get() + 1);
        Ok(())
    }
}

/// AC#1: 多插件分配稳定 ID；dispose 部分后剩余句柄仍存活。
#[test]
fn remaining_handles_alive_after_partial_dispose() {
    let ctx = Context::new();
    let a = ctx
        .plugin(CountingPlugin {
            builds: Rc::new(Cell::new(0)),
        })
        .expect("install a");
    let b = ctx
        .plugin(CountingPlugin {
            builds: Rc::new(Cell::new(0)),
        })
        .expect("install b");
    let c = ctx
        .plugin(CountingPlugin {
            builds: Rc::new(Cell::new(0)),
        })
        .expect("install c");

    let id_a = a.id();
    let id_b = b.id();
    let id_c = c.id();
    assert_ne!(id_a, id_b);
    assert_ne!(id_b, id_c);
    assert_ne!(id_a, id_c);

    assert!(a.is_alive());
    assert!(b.is_alive());
    assert!(c.is_alive());

    b.dispose().expect("dispose b");

    assert!(a.is_alive(), "A must still resolve after B disposed");
    assert!(c.is_alive(), "C must still resolve after B disposed");
    assert!(!ctx.contains_plugin(id_b), "disposed B key must be invalid");
}

/// AC#1: 已 dispose 的键不得误指后续新安装的插件（世代/稳定键语义）。
#[test]
fn disposed_key_does_not_alias_new_plugin() {
    let ctx = Context::new();
    let a = ctx
        .plugin(CountingPlugin {
            builds: Rc::new(Cell::new(0)),
        })
        .expect("install a");
    let b = ctx
        .plugin(CountingPlugin {
            builds: Rc::new(Cell::new(0)),
        })
        .expect("install b");

    let stale_b = b.id();
    b.dispose().expect("dispose b");
    assert!(!ctx.contains_plugin(stale_b));

    let d = ctx
        .plugin(CountingPlugin {
            builds: Rc::new(Cell::new(0)),
        })
        .expect("install d after b disposed");

    assert!(a.is_alive());
    assert!(d.is_alive());
    assert_ne!(
        stale_b,
        d.id(),
        "new key must differ from stale disposed key"
    );
    assert!(
        !ctx.contains_plugin(stale_b),
        "stale key must not resolve to the new entry"
    );
    assert!(ctx.contains_plugin(d.id()));
}

/// AC#1 / #2: `PluginId` 可复制；`id()` 返回稳定键类型。
#[test]
fn plugin_id_is_stable_copyable_key() {
    let ctx = Context::new();
    let handle = ctx
        .plugin(CountingPlugin {
            builds: Rc::new(Cell::new(0)),
        })
        .expect("install");
    let id: PluginId = handle.id();
    let id_copy = id;
    assert_eq!(id, id_copy);
    assert!(ctx.contains_plugin(id));
    assert!(handle.is_alive());
}

/// AC#2: 选用 slotmap（依赖可达）；构建/start 回归不被破坏。
#[test]
fn slotmap_backed_storage_preserves_start_build() {
    let builds = Rc::new(Cell::new(0));
    let ctx = Context::new();
    let h = ctx
        .plugin(CountingPlugin {
            builds: Rc::clone(&builds),
        })
        .expect("delayed install");
    assert!(h.is_alive());
    assert_eq!(builds.get(), 0);
    ctx.start().expect("start");
    assert_eq!(builds.get(), 1);
    assert!(h.is_alive());
}

/// Automate: 上下文 dispose 后插件键全部失效。
#[test]
fn context_dispose_invalidates_plugin_keys() {
    let ctx = Context::new();
    let h = ctx
        .plugin(CountingPlugin {
            builds: Rc::new(Cell::new(0)),
        })
        .expect("install");
    let id = h.id();
    assert!(ctx.contains_plugin(id));
    ctx.dispose();
    assert!(!ctx.contains_plugin(id));
    assert!(!h.is_alive());
}

/// Automate: 立即安装失败回滚后，已分配键不得残留。
#[test]
fn failed_immediate_install_does_not_leave_stale_key() {
    struct NeedsMissing;

    impl Plugin for NeedsMissing {
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            Ok(())
        }

        fn dependencies(&self) -> Vec<std::any::TypeId> {
            vec![std::any::TypeId::of::<u64>()]
        }
    }

    let ctx = Context::new();
    ctx.start().expect("start empty");
    let err = ctx.plugin(NeedsMissing).expect_err("missing dep");
    assert!(matches!(err, Error::MissingDependency));
    // 无存活句柄；再装一个成功插件，确保 map 未因失败插入残留。
    let ok = ctx
        .plugin(CountingPlugin {
            builds: Rc::new(Cell::new(0)),
        })
        .expect("install after failed immediate");
    assert!(ok.is_alive());
}

/// Automate: Cargo.toml 直接依赖声明 slotmap（FR36 / 设计 §7.4）。
#[test]
fn cargo_toml_declares_slotmap_dependency() {
    use std::fs;
    use std::path::PathBuf;

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let text = fs::read_to_string(&manifest).expect("read Cargo.toml");
    assert!(
        text.contains("slotmap"),
        "plugctx must depend on slotmap for stable plugin IDs"
    );
}
