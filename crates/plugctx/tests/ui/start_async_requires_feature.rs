//! 默认 features 下不得调用 `start_async`（需 `async` feature）。
use plugctx::Context;

fn main() {
    let ctx = Context::new();
    let _ = ctx.start_async();
}
