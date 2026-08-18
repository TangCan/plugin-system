#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 1.3 — Plugin trait & delayed/immediate install (ATDD).
//!
//! Red phase: fail to compile or fail assertions until `Plugin`, `Context::plugin`,
//! delayed/immediate `build`, `PluginHandle`, `ReadyEvent`, and `Error::BuildFailed` exist.

use std::cell::Cell;
use std::rc::Rc;

use plugctx::{Context, Error, Plugin, PluginHandle, ReadyEvent};

struct CountingPlugin {
    builds: Rc<Cell<u32>>,
}

impl Plugin for CountingPlugin {
    fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
        self.builds.set(self.builds.get() + 1);
        Ok(())
    }
}

struct FailingPlugin;

impl Plugin for FailingPlugin {
    fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
        Err(Error::BuildFailed)
    }
}

/// AC#1: plugin on unstarted context returns handle and does not call build yet.
#[test]
fn plugin_before_start_is_delayed() {
    let builds = Rc::new(Cell::new(0));
    let ctx = Context::new();
    let handle: PluginHandle = ctx
        .plugin(CountingPlugin {
            builds: Rc::clone(&builds),
        })
        .expect("delayed install should succeed");
    let _ = handle;
    assert_eq!(builds.get(), 0, "build must not run before start");
    assert!(!ctx.is_started());
}

/// AC#2: start builds each delayed plugin exactly once and can emit ReadyEvent.
#[test]
fn start_builds_delayed_plugins_exactly_once() {
    let builds_a = Rc::new(Cell::new(0));
    let builds_b = Rc::new(Cell::new(0));
    let ctx = Context::new();
    ctx.plugin(CountingPlugin {
        builds: Rc::clone(&builds_a),
    })
    .expect("install a");
    ctx.plugin(CountingPlugin {
        builds: Rc::clone(&builds_b),
    })
    .expect("install b");

    ctx.start().expect("start should build delayed plugins");
    assert!(ctx.is_started());
    assert_eq!(builds_a.get(), 1);
    assert_eq!(builds_b.get(), 1);

    // ReadyEvent type must exist for start's emit hook (listeners belong to 1.5).
    let _marker = ReadyEvent;
}

/// AC#3: plugin after start builds immediately.
#[test]
fn plugin_after_start_builds_immediately() {
    let builds = Rc::new(Cell::new(0));
    let ctx = Context::new();
    ctx.start().expect("start empty context");
    ctx.plugin(CountingPlugin {
        builds: Rc::clone(&builds),
    })
    .expect("immediate install");
    assert_eq!(builds.get(), 1);
}

/// AC#3: immediate build failure returns BuildFailed (no success illusion).
#[test]
fn immediate_build_failure_returns_build_failed() {
    let ctx = Context::new();
    ctx.start().expect("start");
    let err = ctx
        .plugin(FailingPlugin)
        .expect_err("immediate build failure must err");
    assert!(
        matches!(err, Error::BuildFailed),
        "expected BuildFailed, got {err:?}"
    );
}

/// AC#4: build failure during start leaves is_started == false.
#[test]
fn start_build_failure_leaves_not_started() {
    let ctx = Context::new();
    ctx.plugin(FailingPlugin)
        .expect("delayed install of failing plugin");
    let err = ctx.start().expect_err("start must fail when build fails");
    assert!(
        matches!(err, Error::BuildFailed),
        "expected BuildFailed, got {err:?}"
    );
    assert!(!ctx.is_started());
}

/// AC#5: plugin after dispose returns AlreadyDisposed.
#[test]
fn plugin_after_dispose_returns_already_disposed() {
    let builds = Rc::new(Cell::new(0));
    let ctx = Context::new();
    ctx.dispose();
    let err = ctx
        .plugin(CountingPlugin {
            builds: Rc::clone(&builds),
        })
        .expect_err("plugin after dispose must fail");
    assert!(matches!(err, Error::AlreadyDisposed));
    assert_eq!(builds.get(), 0);
}

/// AC#6 / NFR1: still no async runtime in direct dependencies.
#[test]
fn no_async_runtime_in_direct_dependencies() {
    use std::fs;
    use std::path::PathBuf;

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let text = fs::read_to_string(&manifest).expect("read plugctx Cargo.toml");
    let forbidden = ["tokio", "async-std", "smol"];
    for name in forbidden {
        assert!(
            !dependency_table_mentions(&text, name),
            "plugctx must not depend on async runtime crate `{name}` (NFR1)"
        );
    }
}

fn dependency_table_mentions(cargo_toml: &str, crate_name: &str) -> bool {
    let mut in_deps = false;
    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]"
                || trimmed == "[dev-dependencies]"
                || trimmed == "[build-dependencies]";
            continue;
        }
        if in_deps && trimmed.starts_with(crate_name) {
            let rest = &trimmed[crate_name.len()..];
            if rest.is_empty() || rest.starts_with([' ', '\t', '=', '.']) {
                return true;
            }
        }
    }
    false
}
