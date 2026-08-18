#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 5.2 — runnable combo example (ATDD).
//!
//! FR29: 可运行示例演示插件组合、服务注入、事件与 dispose。
//! Red phase: `combo_example_source_exists` fails until `examples/combo.rs` lands.

use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use plugctx::{Context, Error, Plugin, ReadyEvent};

/// AC#1/#3: example source must exist for `cargo run -p plugctx --example combo`.
#[test]
fn combo_example_source_exists() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/combo.rs");
    assert!(
        path.is_file(),
        "missing examples/combo.rs — FR29 requires a runnable combo example (story 5.2)"
    );
}

/// Automate 护栏：示例注释须指向 Context / Plugin / Effect 核心概念（AC#2）。
#[test]
fn combo_example_documents_core_concepts() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/combo.rs"),
    )
    .expect("read examples/combo.rs");
    for needle in ["Context", "Plugin", "Effect", "provide", "dispose"] {
        assert!(
            src.contains(needle),
            "examples/combo.rs should mention `{needle}` for FR29 teachability"
        );
    }
}

/// Automate 护栏：工作区 README 须给出 `cargo run --example combo`（AC#2/#3）。
#[test]
fn readme_documents_combo_example_run() {
    let readme = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../README.md"),
    )
    .expect("read plugin-system/README.md");
    assert!(
        readme.contains("cargo run -p plugctx --example combo"),
        "README must document how to run the combo example"
    );
}

#[derive(Debug)]
struct Logger {
    name: &'static str,
}

#[derive(Debug)]
struct PingEvent {
    msg: &'static str,
}

struct LoggerPlugin {
    builds: Rc<Cell<u32>>,
}

impl Plugin for LoggerPlugin {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        self.builds.set(self.builds.get() + 1);
        ctx.provide(Logger { name: "stdout" });
        Ok(())
    }
}

struct AppPlugin {
    builds: Rc<Cell<u32>>,
    got_logger: Rc<Cell<bool>>,
}

impl Plugin for AppPlugin {
    fn dependencies(&self) -> Vec<TypeId> {
        vec![TypeId::of::<Logger>()]
    }

    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        self.builds.set(self.builds.get() + 1);
        let logger = ctx.get::<Logger>().expect("Logger must be provided");
        assert_eq!(logger.name, "stdout");
        self.got_logger.set(true);
        Ok(())
    }
}

/// AC#1: install ≥2 plugins; provide/get via dependency ordering.
#[test]
fn combo_two_plugins_provide_get() {
    let logger_builds = Rc::new(Cell::new(0));
    let app_builds = Rc::new(Cell::new(0));
    let got_logger = Rc::new(Cell::new(false));

    let ctx = Context::new();
    // Install consumer before provider — start must still order by dependencies.
    ctx.plugin(AppPlugin {
        builds: Rc::clone(&app_builds),
        got_logger: Rc::clone(&got_logger),
    })
    .expect("install AppPlugin");
    ctx.plugin(LoggerPlugin {
        builds: Rc::clone(&logger_builds),
    })
    .expect("install LoggerPlugin");

    ctx.start().expect("start combo");
    assert_eq!(logger_builds.get(), 1);
    assert_eq!(app_builds.get(), 1);
    assert!(got_logger.get());
}

/// AC#1: on/emit typed events (plus ReadyEvent hook presence).
#[test]
fn combo_on_emit_custom_event() {
    let pings = Rc::new(Cell::new(0));
    let ready = Rc::new(Cell::new(0));
    let ctx = Context::new();

    let p = Rc::clone(&pings);
    ctx.on(move |e: &PingEvent| {
        assert_eq!(e.msg, "hello");
        p.set(p.get() + 1);
    });

    let r = Rc::clone(&ready);
    ctx.on(move |_e: &ReadyEvent| {
        r.set(r.get() + 1);
    });

    ctx.plugin(LoggerPlugin {
        builds: Rc::new(Cell::new(0)),
    })
    .expect("install");
    ctx.plugin(AppPlugin {
        builds: Rc::new(Cell::new(0)),
        got_logger: Rc::new(Cell::new(false)),
    })
    .expect("install");

    ctx.start().expect("start");
    assert_eq!(ready.get(), 1, "start should emit ReadyEvent");

    ctx.emit(&PingEvent { msg: "hello" });
    assert_eq!(pings.get(), 1);
}

/// AC#1: effect cleanup runs on dispose, reverse registration order.
#[test]
fn combo_effect_cleanup_on_dispose_reverse() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let ctx = Context::new();

    let o1 = Rc::clone(&order);
    ctx.effect(move || {
        o1.borrow_mut().push("setup-a");
        let o = Rc::clone(&o1);
        move || {
            o.borrow_mut().push("cleanup-a");
        }
    });

    let o2 = Rc::clone(&order);
    ctx.effect(move || {
        o2.borrow_mut().push("setup-b");
        let o = Rc::clone(&o2);
        move || {
            o.borrow_mut().push("cleanup-b");
        }
    });

    ctx.start().expect("start");
    ctx.dispose();

    assert_eq!(
        *order.borrow(),
        vec!["setup-a", "setup-b", "cleanup-b", "cleanup-a"]
    );
}

/// AC#1: start then dispose lifecycle for multi-plugin context.
#[test]
fn combo_start_dispose_lifecycle() {
    let ctx = Context::new();
    ctx.plugin(LoggerPlugin {
        builds: Rc::new(Cell::new(0)),
    })
    .unwrap();
    ctx.plugin(AppPlugin {
        builds: Rc::new(Cell::new(0)),
        got_logger: Rc::new(Cell::new(false)),
    })
    .unwrap();

    assert!(!ctx.is_started());
    ctx.start().expect("start");
    assert!(ctx.is_started());
    ctx.dispose();
    assert!(ctx.is_disposed());
}
