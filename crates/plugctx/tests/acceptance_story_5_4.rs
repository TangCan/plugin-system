#![cfg(all(feature = "tracing", not(feature = "thread-safe")))]

//! Acceptance tests for story 5.4 — tracing 诊断 feature（ATDD）。
//!
//! 启用 `tracing` 时关键路径可观测 span；Cargo 声明为可选门面且不含订阅端（FR37 / NFR1）。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use plugctx::Context;
use tracing::span::{Attributes, Id};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context as LayerCtx, Layer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::Registry;

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

#[derive(Clone, Default)]
struct SpanRecorder {
    names: Arc<Mutex<Vec<String>>>,
}

impl<S> Layer<S> for SpanRecorder
where
    S: Subscriber,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, _id: &Id, _ctx: LayerCtx<'_, S>) {
        self.names
            .lock()
            .expect("span recorder lock")
            .push(attrs.metadata().name().to_string());
    }

    fn on_event(&self, event: &Event<'_>, _ctx: LayerCtx<'_, S>) {
        // 事件名通常为 "event"；保留以便将来扩展断言。
        let _ = event;
    }
}

fn capture_spans(f: impl FnOnce()) -> Vec<String> {
    let recorder = SpanRecorder::default();
    let names = recorder.names.clone();
    let subscriber = Registry::default().with(recorder);
    tracing::subscriber::with_default(subscriber, f);
    let captured = names.lock().expect("span recorder lock").clone();
    captured
}

/// AC#2: `tracing` 为可选门面；正常依赖不含 tracing-subscriber。
#[test]
fn cargo_toml_tracing_is_optional_facade() {
    let cargo = read_utf8("crates/plugctx/Cargo.toml");
    assert!(
        cargo.contains("tracing = [\"dep:tracing\"]"),
        "plugctx must declare optional tracing feature"
    );
    assert!(
        cargo.contains("tracing = { workspace = true, optional = true }"),
        "tracing must be optional dependency"
    );
    assert!(
        !cargo.contains("default = [\"tracing\"]") && cargo.contains("default = []"),
        "tracing must not be in default features"
    );
    // 订阅端不得出现在 [dependencies] 正常依赖表中（注释除外）。
    let deps_body = cargo
        .split("[dependencies]")
        .nth(1)
        .and_then(|s| s.split('[').next())
        .expect("[dependencies] section");
    let deps_decl_lines: Vec<&str> = deps_body
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect();
    assert!(
        !deps_decl_lines
            .iter()
            .any(|l| l.contains("tracing-subscriber")),
        "tracing-subscriber must not be a normal dependency (NFR1)"
    );
    assert!(
        cargo.contains("tracing-subscriber = { workspace = true }"),
        "dev-dependencies should include tracing-subscriber for acceptance capture"
    );
}

/// AC#2: README 说明 tracing 门面、非默认、无强制订阅端。
#[test]
fn readme_documents_tracing_feature() {
    let readme = read_utf8("README.md");
    assert!(
        readme.contains("`tracing`") || readme.contains("| `tracing`"),
        "README Feature 矩阵 must mention tracing"
    );
    assert!(
        readme.contains("门面") || readme.contains("tracing-subscriber"),
        "README must explain facade / subscriber boundary"
    );
    assert!(
        readme.contains("非默认") || readme.contains("--features tracing"),
        "README must document opt-in tracing"
    );
}

/// AC#1: start 发出 plugctx.start span。
#[test]
fn start_emits_structured_span() {
    let names = capture_spans(|| {
        let ctx = Context::new();
        ctx.start().expect("start");
        ctx.dispose();
    });
    assert!(
        names.iter().any(|n| n == "plugctx.start"),
        "expected plugctx.start span, got {names:?}"
    );
}

/// AC#1: emit 发出 plugctx.emit span。
#[test]
fn emit_emits_structured_span() {
    #[derive(Clone, Copy)]
    struct Probe;
    let names = capture_spans(|| {
        let ctx = Context::new();
        ctx.start().expect("start");
        ctx.emit(&Probe);
        ctx.dispose();
    });
    assert!(
        names.iter().any(|n| n == "plugctx.emit"),
        "expected plugctx.emit span, got {names:?}"
    );
}

/// AC#1: dispose 发出 plugctx.dispose span。
#[test]
fn dispose_emits_structured_span() {
    let names = capture_spans(|| {
        let ctx = Context::new();
        ctx.start().expect("start");
        ctx.dispose();
    });
    assert!(
        names.iter().any(|n| n == "plugctx.dispose"),
        "expected plugctx.dispose span, got {names:?}"
    );
}

/// AC#1: 构建单个插件时发出 plugctx.build_plugin span。
#[test]
fn build_plugin_emits_structured_span() {
    use plugctx::Plugin;

    struct EmptyPlugin;
    impl Plugin for EmptyPlugin {
        fn build(&self, _ctx: &mut Context) -> Result<(), plugctx::Error> {
            Ok(())
        }
    }

    let names = capture_spans(|| {
        let ctx = Context::new();
        ctx.plugin(EmptyPlugin).expect("install");
        ctx.start().expect("start");
        ctx.dispose();
    });
    assert!(
        names.iter().any(|n| n == "plugctx.build_plugin"),
        "expected plugctx.build_plugin span, got {names:?}"
    );
}

/// AC#1: start → emit → dispose 组合可观测。
#[test]
fn start_emit_dispose_spans_observable() {
    #[derive(Clone, Copy)]
    struct Probe;
    let names = capture_spans(|| {
        let ctx = Context::new();
        ctx.start().expect("start");
        ctx.emit(&Probe);
        ctx.dispose();
    });
    for need in ["plugctx.start", "plugctx.emit", "plugctx.dispose"] {
        assert!(
            names.iter().any(|n| n == need),
            "missing span `{need}` in {names:?}"
        );
    }
}
