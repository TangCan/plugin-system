#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 1.1 — plugctx core crate skeleton (ATDD).
//!
//! Red phase: these tests fail to compile or fail assertions until
//! `plugctx` exposes the required module placeholders and stays free of
//! async runtime dependencies.

use std::fs;
use std::path::PathBuf;

/// AC#1: core module placeholders must be publicly reachable.
#[test]
fn core_modules_are_publicly_reachable() {
    // Touch each module path so missing `pub mod` fails at compile time.
    let _ = plugctx::context::MODULE_NAME;
    let _ = plugctx::plugin::MODULE_NAME;
    let _ = plugctx::service::MODULE_NAME;
    let _ = plugctx::event::MODULE_NAME;
    let _ = plugctx::effect::MODULE_NAME;
    let _ = plugctx::error::MODULE_NAME;
}

/// AC#1: crate smoke — package builds and tests run.
#[test]
fn smoke_crate_links() {
    assert_eq!(plugctx::CRATE_NAME, "plugctx");
}

/// AC#2: workspace README documents coexistence with legacy crates.
#[test]
fn readme_documents_coexistence_with_legacy_scaffold() {
    let readme = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../README.md");
    let text = fs::read_to_string(&readme).expect("read plugin-system README.md");
    assert!(
        text.contains("plugctx") && text.contains("plugin-api") && text.contains("plugin-host"),
        "README must mention plugctx alongside plugin-api/plugin-host"
    );
    assert!(
        text.contains("并存") || text.to_ascii_lowercase().contains("coexist"),
        "README must state coexistence/migration relationship"
    );
}

/// AC#3 / NFR1: 默认路径不得强制异步运行时；`async-trait`/`futures` 仅允许为 optional。
#[test]
fn no_async_runtime_in_direct_dependencies() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let text = fs::read_to_string(&manifest).expect("read plugctx Cargo.toml");
    let runtimes = ["tokio", "async-std", "smol"];
    for name in runtimes {
        assert!(
            !dependency_table_mentions(&text, name),
            "plugctx must not depend on async runtime crate `{name}` (NFR1)"
        );
    }
    // Story 3.1: async-trait / futures 可作为 optional 依赖，但不得非 optional 强制拉入。
    for name in ["async-trait", "futures"] {
        if let Some(line) = dependency_line(&text, "[dependencies]", name) {
            assert!(
                line.contains("optional") || line.contains("dep:"),
                "`{name}` in [dependencies] must be optional (feature-gated): {line}"
            );
        }
    }
}

fn dependency_table_mentions(cargo_toml: &str, crate_name: &str) -> bool {
    dependency_line(cargo_toml, "[dependencies]", crate_name).is_some()
        || dependency_line(cargo_toml, "[dev-dependencies]", crate_name).is_some()
        || dependency_line(cargo_toml, "[build-dependencies]", crate_name).is_some()
}

fn dependency_line<'a>(cargo_toml: &'a str, table: &str, crate_name: &str) -> Option<&'a str> {
    let mut in_table = false;
    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_table = trimmed == table;
            continue;
        }
        if in_table && trimmed.starts_with(crate_name) {
            let rest = &trimmed[crate_name.len()..];
            if rest.is_empty() || rest.starts_with([' ', '\t', '=', '.']) {
                return Some(trimmed);
            }
        }
    }
    None
}
