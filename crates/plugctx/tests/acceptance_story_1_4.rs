#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 1.4 — TypeId DI & dependency ordering errors (ATDD).
//!
//! Red phase: fail to compile or fail assertions until `provide`/`get`/`get_mut`,
//! optimistic dependency-ordered `start`, and `MissingDependency`/`CircularDependency` exist.

use std::any::TypeId;
use std::cell::Cell;
use std::rc::Rc;

use plugctx::{Context, Error, Plugin};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Counter(u32);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token(String);

struct ProviderPlugin;

impl Plugin for ProviderPlugin {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        ctx.provide(Token("ready".into()));
        Ok(())
    }
}

struct ConsumerPlugin {
    saw_token: Rc<Cell<bool>>,
}

impl Plugin for ConsumerPlugin {
    fn dependencies(&self) -> Vec<TypeId> {
        vec![TypeId::of::<Token>()]
    }

    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        let token = ctx
            .get::<Token>()
            .expect("Token must be provided before build");
        assert_eq!(token.0, "ready");
        self.saw_token.set(true);
        Ok(())
    }
}

struct NeedsMissing {
    builds: Rc<Cell<u32>>,
}

impl Plugin for NeedsMissing {
    fn dependencies(&self) -> Vec<TypeId> {
        vec![TypeId::of::<Token>()]
    }

    fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
        self.builds.set(self.builds.get() + 1);
        Ok(())
    }
}

/// Mutual wait: A needs BToken, B needs AToken — neither can build first.
#[derive(Debug)]
struct AToken;
#[derive(Debug)]
struct BToken;

struct PluginNeedsB {
    builds: Rc<Cell<u32>>,
}

impl Plugin for PluginNeedsB {
    fn dependencies(&self) -> Vec<TypeId> {
        vec![TypeId::of::<BToken>()]
    }

    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        self.builds.set(self.builds.get() + 1);
        ctx.provide(AToken);
        Ok(())
    }
}

struct PluginNeedsA {
    builds: Rc<Cell<u32>>,
}

impl Plugin for PluginNeedsA {
    fn dependencies(&self) -> Vec<TypeId> {
        vec![TypeId::of::<AToken>()]
    }

    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        self.builds.set(self.builds.get() + 1);
        ctx.provide(BToken);
        Ok(())
    }
}

/// AC#1: provide then get/get_mut; replace returns old value.
#[test]
fn provide_get_get_mut_and_replace() {
    let ctx = Context::new();
    assert!(ctx.get::<Counter>().is_none());

    assert!(ctx.provide(Counter(1)).is_none());
    assert_eq!(ctx.get::<Counter>().map(|c| c.0), Some(1));

    {
        let mut c = ctx.get_mut::<Counter>().expect("Counter present");
        c.0 = 2;
    }
    assert_eq!(ctx.get::<Counter>().map(|c| c.0), Some(2));

    let old = ctx.provide(Counter(99)).expect("replaced old value");
    assert_eq!(old, Counter(2));
    assert_eq!(ctx.get::<Counter>().map(|c| c.0), Some(99));
}

/// AC#2: missing dependency → MissingDependency; unsatisfied build not called.
#[test]
fn missing_dependency_skips_unsatisfied_build() {
    let builds = Rc::new(Cell::new(0));
    let ctx = Context::new();
    ctx.plugin(NeedsMissing {
        builds: Rc::clone(&builds),
    })
    .expect("delayed install");

    let err = ctx
        .start()
        .expect_err("start must fail on missing dependency");
    assert!(
        matches!(err, Error::MissingDependency),
        "expected MissingDependency, got {err:?}"
    );
    assert_eq!(builds.get(), 0, "build must not run when deps unsatisfied");
    assert!(!ctx.is_started());
}

/// AC#3: mutual dependency cycle → CircularDependency; no infinite loop.
#[test]
fn circular_dependency_returns_error() {
    let builds_a = Rc::new(Cell::new(0));
    let builds_b = Rc::new(Cell::new(0));
    let ctx = Context::new();
    ctx.plugin(PluginNeedsB {
        builds: Rc::clone(&builds_a),
    })
    .expect("install a");
    ctx.plugin(PluginNeedsA {
        builds: Rc::clone(&builds_b),
    })
    .expect("install b");

    let err = ctx
        .start()
        .expect_err("start must fail on circular dependency");
    assert!(
        matches!(err, Error::CircularDependency),
        "expected CircularDependency, got {err:?}"
    );
    assert_eq!(builds_a.get(), 0);
    assert_eq!(builds_b.get(), 0);
    assert!(!ctx.is_started());
}

/// Retro item-2: ≥2 插件各自缺失**不同**依赖 → CircularDependency（无法进展语义，非真环）。
#[test]
fn two_plugins_missing_different_deps_is_circular_stuck_progress() {
    #[derive(Debug)]
    struct MissingAlpha;
    #[derive(Debug)]
    struct MissingBeta;

    struct NeedsAlpha {
        builds: Rc<Cell<u32>>,
    }
    impl Plugin for NeedsAlpha {
        fn dependencies(&self) -> Vec<TypeId> {
            vec![TypeId::of::<MissingAlpha>()]
        }
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            self.builds.set(self.builds.get() + 1);
            Ok(())
        }
    }

    struct NeedsBeta {
        builds: Rc<Cell<u32>>,
    }
    impl Plugin for NeedsBeta {
        fn dependencies(&self) -> Vec<TypeId> {
            vec![TypeId::of::<MissingBeta>()]
        }
        fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
            self.builds.set(self.builds.get() + 1);
            Ok(())
        }
    }

    let builds_a = Rc::new(Cell::new(0));
    let builds_b = Rc::new(Cell::new(0));
    let ctx = Context::new();
    ctx.plugin(NeedsAlpha {
        builds: Rc::clone(&builds_a),
    })
    .expect("install alpha-needer");
    ctx.plugin(NeedsBeta {
        builds: Rc::clone(&builds_b),
    })
    .expect("install beta-needer");

    let err = ctx.start().expect_err("stuck multi-missing must not hang");
    assert!(
        matches!(err, Error::CircularDependency),
        "≥2 plugins each missing different deps → CircularDependency (stuck progress), got {err:?}"
    );
    assert_eq!(builds_a.get(), 0);
    assert_eq!(builds_b.get(), 0);
    assert!(!ctx.is_started());
}

/// AC#4: provider before consumer even if consumer installed first.
#[test]
fn dependency_order_provider_before_consumer() {
    let saw = Rc::new(Cell::new(false));
    let ctx = Context::new();
    // Install consumer first — optimistic sort must still build provider first.
    ctx.plugin(ConsumerPlugin {
        saw_token: Rc::clone(&saw),
    })
    .expect("install consumer");
    ctx.plugin(ProviderPlugin).expect("install provider");

    ctx.start().expect("start with resolvable dependency order");
    assert!(ctx.is_started());
    assert!(saw.get(), "consumer build must see Token from provider");
}

/// Guardrail: root-level `provide` satisfies delayed plugin deps (not only plugin-provided).
#[test]
fn root_provide_satisfies_plugin_dependency() {
    let saw = Rc::new(Cell::new(false));
    let ctx = Context::new();
    ctx.provide(Token("ready".into()));
    ctx.plugin(ConsumerPlugin {
        saw_token: Rc::clone(&saw),
    })
    .expect("install consumer");
    ctx.start()
        .expect("root-provided Token satisfies dependency");
    assert!(saw.get());
}

/// Guardrail: immediate install after start rejects unsatisfied dependencies.
#[test]
fn immediate_install_missing_dependency() {
    let builds = Rc::new(Cell::new(0));
    let ctx = Context::new();
    ctx.start().expect("start empty");
    let err = ctx
        .plugin(NeedsMissing {
            builds: Rc::clone(&builds),
        })
        .expect_err("immediate install must check deps");
    assert!(matches!(err, Error::MissingDependency));
    assert_eq!(builds.get(), 0);
}

/// NFR1: still no async runtime in direct dependencies.
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
