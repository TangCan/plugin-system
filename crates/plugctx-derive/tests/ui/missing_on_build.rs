use plugctx_derive::Plugin;

#[derive(Plugin)]
struct NoBuild;

fn main() {
    let _ = NoBuild;
}
