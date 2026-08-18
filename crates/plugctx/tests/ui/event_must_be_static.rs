//! 事件类型须满足 `'static`；带生命周期的事件不得注册到 `on`。
use plugctx::Context;

struct Borrowed<'a>(&'a str);

fn register_non_static<'a>(ctx: &Context, s: &'a str) {
    let _ = ctx.on(move |ev: &Borrowed<'a>| {
        let _ = (ev.0, s);
    });
}

fn main() {
    let ctx = Context::new();
    let owned = String::from("nope");
    register_non_static(&ctx, &owned);
}
