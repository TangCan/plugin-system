#![cfg(not(feature = "thread-safe"))]

//! Acceptance tests for story 1.7 — isolate inherit / isolate / cascade dispose (ATDD).
//!
//! Red phase: fail to compile or fail assertions until `isolate`, parent service
//! inheritance, child provide isolation, and cascade dispose exist.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use plugctx::Context;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token(String);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Counter(u32);

/// AC#1: child get reads parent-provided service.
#[test]
fn isolate_child_can_get_parent_service() {
    let parent = Context::new();
    parent.provide(Token("from-parent".into()));

    let child = parent.isolate().expect("isolate");
    assert_eq!(
        child.get::<Token>().map(|t| t.0.clone()),
        Some("from-parent".into()),
        "child must inherit parent service via get"
    );
}

/// AC#2: child provide same type does not pollute parent.
#[test]
fn child_provide_does_not_pollute_parent() {
    let parent = Context::new();
    parent.provide(Token("parent".into()));
    let child = parent.isolate().expect("isolate");

    child.provide(Token("child".into()));

    assert_eq!(
        child.get::<Token>().map(|t| t.0.clone()),
        Some("child".into()),
        "child must see its own override"
    );
    assert_eq!(
        parent.get::<Token>().map(|t| t.0.clone()),
        Some("parent".into()),
        "parent must keep original value"
    );
}

/// AC#3: parent dispose cascades to child and grandchild; effects cleaned per scope.
#[test]
fn parent_dispose_cascades_to_descendants_and_runs_effects() {
    let log = Rc::new(RefCell::new(Vec::new()));

    let parent = Context::new();
    let child = parent.isolate().expect("isolate");
    let grand = child.isolate().expect("isolate");

    let lp = Rc::clone(&log);
    parent.effect(move || {
        let l = Rc::clone(&lp);
        move || {
            l.borrow_mut().push("cleanup-parent");
        }
    });
    let lc = Rc::clone(&log);
    child.effect(move || {
        let l = Rc::clone(&lc);
        move || {
            l.borrow_mut().push("cleanup-child");
        }
    });
    let lg = Rc::clone(&log);
    grand.effect(move || {
        let l = Rc::clone(&lg);
        move || {
            l.borrow_mut().push("cleanup-grand");
        }
    });

    parent.dispose();

    assert!(parent.is_disposed());
    assert!(child.is_disposed());
    assert!(grand.is_disposed());

    let order = log.borrow().clone();
    assert!(
        order.contains(&"cleanup-parent"),
        "parent effect must run: {order:?}"
    );
    assert!(
        order.contains(&"cleanup-child"),
        "child effect must run via cascade: {order:?}"
    );
    assert!(
        order.contains(&"cleanup-grand"),
        "grandchild effect must run via cascade: {order:?}"
    );
    // Parent cleanup runs before children are disposed (design: effects then children).
    let pi = order.iter().position(|s| *s == "cleanup-parent").unwrap();
    let ci = order.iter().position(|s| *s == "cleanup-child").unwrap();
    let gi = order.iter().position(|s| *s == "cleanup-grand").unwrap();
    assert!(
        pi < ci && pi < gi,
        "parent cleanup before child/grand: {order:?}"
    );
}

/// AC#4: disposing child alone leaves parent usable.
#[test]
fn child_dispose_alone_leaves_parent_intact() {
    let parent = Context::new();
    parent.provide(Counter(7));
    let child = parent.isolate().expect("isolate");
    child.provide(Counter(1));

    let lc = Rc::new(Cell::new(false));
    let flag = Rc::clone(&lc);
    child.effect(move || {
        move || {
            flag.set(true);
        }
    });

    child.dispose();

    assert!(child.is_disposed());
    assert!(!parent.is_disposed());
    assert_eq!(
        parent.get::<Counter>().map(|c| c.0),
        Some(7),
        "parent service must remain after child dispose"
    );
    assert!(lc.get(), "child effect cleanup must still run");

    // Parent can still isolate / provide after child dispose.
    parent.provide(Token("still-ok".into()));
    assert_eq!(
        parent.get::<Token>().map(|t| t.0.clone()),
        Some("still-ok".into())
    );
}

/// Guardrail: grandchild can read root service through chain.
#[test]
fn grandchild_inherits_through_chain() {
    let root = Context::new();
    root.provide(Token("root".into()));
    let mid = root.isolate().expect("isolate");
    let leaf = mid.isolate().expect("isolate");
    assert_eq!(
        leaf.get::<Token>().map(|t| t.0.clone()),
        Some("root".into())
    );
}

/// Guardrail: child get_mut 命中父服务时修改的是父级同一实例。
#[test]
fn child_get_mut_inherits_and_mutates_parent_instance() {
    let parent = Context::new();
    parent.provide(Counter(1));
    let child = parent.isolate().expect("isolate");

    {
        let mut c = child.get_mut::<Counter>().expect("inherited Counter");
        c.0 = 42;
    }

    assert_eq!(parent.get::<Counter>().map(|c| c.0), Some(42));
    assert_eq!(child.get::<Counter>().map(|c| c.0), Some(42));
}

/// Guardrail: 中间层 provide 遮蔽根服务；根不受影响。
#[test]
fn mid_override_shadows_for_leaf_not_root() {
    let root = Context::new();
    root.provide(Token("root".into()));
    let mid = root.isolate().expect("isolate");
    mid.provide(Token("mid".into()));
    let leaf = mid.isolate().expect("isolate");

    assert_eq!(leaf.get::<Token>().map(|t| t.0.clone()), Some("mid".into()));
    assert_eq!(
        root.get::<Token>().map(|t| t.0.clone()),
        Some("root".into())
    );
}

/// Guardrail: 父 dispose 幂等；已级联销毁的子再次 dispose 安全。
#[test]
fn cascade_dispose_idempotent() {
    let parent = Context::new();
    let child = parent.isolate().expect("isolate");
    parent.dispose();
    assert!(child.is_disposed());
    parent.dispose();
    child.dispose();
    assert!(parent.is_disposed());
    assert!(child.is_disposed());
}

/// Retro item-1: 父已 dispose 后 isolate 必须 AlreadyDisposed，不得成功登记孤儿子树。
#[test]
fn isolate_after_parent_dispose_returns_already_disposed() {
    let parent = Context::new();
    parent.dispose();
    assert!(parent.is_disposed());
    let err = match parent.isolate() {
        Ok(_) => panic!("isolate on disposed parent must fail"),
        Err(e) => e,
    };
    assert!(
        matches!(err, plugctx::Error::AlreadyDisposed),
        "expected AlreadyDisposed, got {err:?}"
    );
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
