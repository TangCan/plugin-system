#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 1.2 — Context lifecycle new / start / dispose (ATDD).
//!
//! Red phase: these tests fail to compile or fail assertions until
//! `Context::new/start/dispose/is_started/is_disposed` and
//! `Error::{AlreadyStarted,AlreadyDisposed}` exist.

use plugctx::{Context, Error};

/// AC#1: fresh context is neither started nor disposed.
#[test]
fn new_context_is_not_started_or_disposed() {
    let ctx = Context::new();
    assert!(!ctx.is_started());
    assert!(!ctx.is_disposed());
}

/// AC#2: start succeeds and flips is_started.
#[test]
fn start_marks_context_as_started() {
    let ctx = Context::new();
    ctx.start().expect("start should succeed on fresh context");
    assert!(ctx.is_started());
    assert!(!ctx.is_disposed());
}

/// AC#3: second start returns AlreadyStarted (no panic).
#[test]
fn start_twice_returns_already_started() {
    let ctx = Context::new();
    ctx.start().expect("first start");
    let err = ctx.start().expect_err("second start must fail");
    assert!(
        matches!(err, Error::AlreadyStarted),
        "expected AlreadyStarted, got {err:?}"
    );
    assert!(ctx.is_started());
}

/// AC#4: dispose sets disposed; second dispose is idempotent.
#[test]
fn dispose_is_idempotent() {
    let ctx = Context::new();
    ctx.dispose();
    assert!(ctx.is_disposed());
    ctx.dispose(); // must not panic
    assert!(ctx.is_disposed());
}

/// AC#4: dispose after start also works and is idempotent.
#[test]
fn dispose_after_start_is_idempotent() {
    let ctx = Context::new();
    ctx.start().expect("start");
    ctx.dispose();
    assert!(ctx.is_disposed());
    ctx.dispose();
    assert!(ctx.is_disposed());
}

/// AC#5: start after dispose returns AlreadyDisposed.
#[test]
fn start_after_dispose_returns_already_disposed() {
    let ctx = Context::new();
    ctx.dispose();
    let err = ctx.start().expect_err("start after dispose must fail");
    assert!(
        matches!(err, Error::AlreadyDisposed),
        "expected AlreadyDisposed, got {err:?}"
    );
}

/// AC#5: start after start+dispose still returns AlreadyDisposed.
#[test]
fn start_after_started_then_dispose_returns_already_disposed() {
    let ctx = Context::new();
    ctx.start().expect("start");
    ctx.dispose();
    let err = ctx.start().expect_err("start after dispose must fail");
    assert!(matches!(err, Error::AlreadyDisposed));
}

/// Guard: Context is a shared handle — clone observes the same lifecycle flags.
#[test]
fn context_clone_shares_lifecycle_state() {
    let ctx = Context::new();
    let clone = ctx.clone();
    ctx.start().expect("start");
    assert!(clone.is_started());
    clone.dispose();
    assert!(ctx.is_disposed());
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
