//! Story 5.1 验收：derive 生成 dependencies / on_build 委托（FR27）。

use std::any::TypeId;
use std::cell::Cell;
use std::rc::Rc;

use plugctx::{Context, Error, Plugin};
use plugctx_derive::Plugin as PluginDerive;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token(String);

struct Provider;

impl Plugin for Provider {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        ctx.provide(Token("ok".into()));
        Ok(())
    }
}

#[derive(PluginDerive)]
#[plugin(depends(Token))]
struct Consumer {
    saw: Rc<Cell<bool>>,
}

impl Consumer {
    fn on_build(&self, ctx: &mut Context) -> Result<(), Error> {
        let token = ctx.get::<Token>().expect("Token 应在 build 前可用");
        assert_eq!(token.0, "ok");
        self.saw.set(true);
        Ok(())
    }
}

#[derive(PluginDerive)]
struct EmptyDepends {
    ran: Rc<Cell<bool>>,
}

impl EmptyDepends {
    fn on_build(&self, _ctx: &mut Context) -> Result<(), Error> {
        self.ran.set(true);
        Ok(())
    }
}

#[test]
fn derive_plugin_respects_depends_ordering() {
    let saw = Rc::new(Cell::new(false));
    let ctx = Context::new();
    ctx.plugin(Provider).expect("install provider");
    ctx.plugin(Consumer {
        saw: Rc::clone(&saw),
    })
    .expect("install consumer");
    ctx.start().expect("依赖应可满足");
    assert!(saw.get());
}

#[test]
fn derive_plugin_missing_depend_errors() {
    let saw = Rc::new(Cell::new(false));
    let ctx = Context::new();
    ctx.plugin(Consumer {
        saw: Rc::clone(&saw),
    })
    .expect("install consumer");
    let err = ctx.start().expect_err("缺少 Token 应失败");
    assert!(
        matches!(err, Error::MissingDependency),
        "expected MissingDependency, got {err:?}"
    );
    assert!(!saw.get());
}

#[test]
fn derive_plugin_empty_depends_builds() {
    let ran = Rc::new(Cell::new(false));
    let ctx = Context::new();
    ctx.plugin(EmptyDepends {
        ran: Rc::clone(&ran),
    })
    .expect("install empty");
    ctx.start().expect("无 depends 应可构建");
    assert!(ran.get());
}

#[test]
fn derive_dependencies_lists_declared_types() {
    let p = Consumer {
        saw: Rc::new(Cell::new(false)),
    };
    assert_eq!(p.dependencies(), vec![TypeId::of::<Token>()]);
}

#[derive(Debug)]
struct Alpha;
#[derive(Debug)]
struct Beta;

#[derive(PluginDerive)]
#[plugin(depends(Alpha, Beta))]
struct MultiDepends;

impl MultiDepends {
    fn on_build(&self, _ctx: &mut Context) -> Result<(), Error> {
        Ok(())
    }
}

#[test]
fn derive_multi_depends_lists_both_types() {
    let p = MultiDepends;
    assert_eq!(
        p.dependencies(),
        vec![TypeId::of::<Alpha>(), TypeId::of::<Beta>()]
    );
}

#[test]
fn core_crate_does_not_depend_on_derive() {
    let manifest = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../plugctx/Cargo.toml"
    ));
    assert!(
        !manifest.contains("plugctx-derive"),
        "plugctx 核心不得依赖 plugctx-derive（FR27）"
    );
}
