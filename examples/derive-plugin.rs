//! `#[derive(Plugin)]` 演示（`plugctx-derive`）
//!
//! ```bash
//! cargo run -p plugctx-examples --example derive-plugin
//! ```

use std::cell::Cell;
use std::rc::Rc;

use plugctx::{Context, Error, Plugin};
use plugctx_derive::Plugin as PluginDerive;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token(&'static str);

struct Provider;

impl Plugin for Provider {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        println!("→ Provider::build — provide(Token)");
        ctx.provide(Token("hello-derive"));
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
        let token = ctx.get::<Token>().expect("Token");
        println!("→ Consumer::on_build — Token = {:?}", token.0);
        assert_eq!(token.0, "hello-derive");
        self.saw.set(true);
        Ok(())
    }
}

fn main() {
    println!("=== plugctx-examples derive-plugin ===\n");

    let saw = Rc::new(Cell::new(false));
    let ctx = Context::new();
    ctx.plugin(Provider).expect("provider");
    ctx.plugin(Consumer {
        saw: Rc::clone(&saw),
    })
    .expect("consumer");
    ctx.start().expect("start");
    assert!(saw.get());
    ctx.dispose();
    println!("\n=== done ===");
}
