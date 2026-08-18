//! 未实现 `Plugin` 的类型不得安装进 Context（编译期拒绝）。
use plugctx::Context;

struct NotAPlugin;

fn main() {
    let ctx = Context::new();
    let _ = ctx.plugin(NotAPlugin);
}
