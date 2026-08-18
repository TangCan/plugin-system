//! 无引擎 tick 循环：插件在 tick 中改状态，卸载后不再改。
//!
//! ```bash
//! cargo run -p plugctx-examples --example game-loop
//! ```

use std::cell::Cell;
use std::rc::Rc;

use plugctx::{Context, Error, Plugin};

struct Tick;

struct Bumper {
    hits: Rc<Cell<u32>>,
}

impl Plugin for Bumper {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        let hits = Rc::clone(&self.hits);
        ctx.on(move |_: &Tick| {
            hits.set(hits.get() + 1);
        });
        let _ = ctx.effect(|| {
            println!("bumper effect setup");
            || println!("bumper effect cleanup")
        });
        Ok(())
    }
}

fn main() {
    let hits = Rc::new(Cell::new(0));
    let ctx = Context::new();
    let handle = ctx
        .plugin(Bumper {
            hits: Rc::clone(&hits),
        })
        .expect("install");
    ctx.start().expect("start");

    for i in 1..=3 {
        ctx.emit(&Tick);
        println!("tick {i} hits={}", hits.get());
    }
    assert_eq!(hits.get(), 3);

    handle.dispose().expect("unload bumper");
    ctx.emit(&Tick);
    ctx.emit(&Tick);
    println!("after unload hits={} (must stay 3)", hits.get());
    assert_eq!(hits.get(), 3);

    ctx.dispose();
}
