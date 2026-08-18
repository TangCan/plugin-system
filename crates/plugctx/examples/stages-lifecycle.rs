//! stages 生命周期事件示例（feature `stages`）
//!
//! 顺序：Init → Ready → PostStart；dispose：PreDispose → Dispose。
//!
//! ```bash
//! cargo run -p plugctx --example stages-lifecycle --features stages
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use plugctx::{Context, DisposeEvent, InitEvent, PostStartEvent, PreDisposeEvent, ReadyEvent};

fn main() {
    println!("=== plugctx stages-lifecycle example ===\n");

    let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    let ctx = Context::new();

    {
        let log = Rc::clone(&log);
        ctx.on(move |_: &InitEvent| {
            log.borrow_mut().push("init");
            println!("• InitEvent");
        });
    }
    {
        let log = Rc::clone(&log);
        ctx.on(move |_: &ReadyEvent| {
            log.borrow_mut().push("ready");
            println!("• ReadyEvent");
        });
    }
    {
        let log = Rc::clone(&log);
        ctx.on(move |_: &PostStartEvent| {
            log.borrow_mut().push("post-start");
            println!("• PostStartEvent");
        });
    }
    {
        let log = Rc::clone(&log);
        ctx.on(move |_: &PreDisposeEvent| {
            log.borrow_mut().push("pre-dispose");
            println!("• PreDisposeEvent");
        });
    }
    {
        let log = Rc::clone(&log);
        ctx.on(move |_: &DisposeEvent| {
            log.borrow_mut().push("dispose");
            println!("• DisposeEvent");
        });
    }

    println!("--- start() ---");
    ctx.start().expect("start");

    println!("\n--- dispose() ---");
    ctx.dispose();

    assert_eq!(
        log.borrow().as_slice(),
        &["init", "ready", "post-start", "pre-dispose", "dispose"]
    );
    println!("\n=== done ===");
}
