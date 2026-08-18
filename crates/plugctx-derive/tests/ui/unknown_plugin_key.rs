use plugctx_derive::Plugin;

#[derive(Plugin)]
#[plugin(inject(Foo))]
struct BadAttr;

impl BadAttr {
    fn on_build(&self, _ctx: &mut plugctx::Context) -> Result<(), plugctx::Error> {
        Ok(())
    }
}

fn main() {}
