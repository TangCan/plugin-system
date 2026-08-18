//! AsyncPlugin + start_async 示例（feature `async`）
//!
//! 异步插件须用 [`Context::plugin_async`] 安装（仅 `plugin` 不会走 `build_async`）。
//!
//! ```bash
//! cargo run -p plugctx --example async-start --features async
//! ```

use std::any::TypeId;
use std::cell::RefCell;
use std::rc::Rc;

use async_trait::async_trait;
use futures::executor::block_on;
use plugctx::{AsyncPlugin, Context, Error, Plugin, ReadyEvent};

#[derive(Debug)]
struct Config {
    name: &'static str,
}

struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        println!("→ ConfigPlugin::build — provide(Config)");
        ctx.provide(Config { name: "async-demo" });
        Ok(())
    }
}

struct BootPlugin {
    order: Rc<RefCell<Vec<&'static str>>>,
}

impl Plugin for BootPlugin {
    fn dependencies(&self) -> Vec<TypeId> {
        vec![TypeId::of::<Config>()]
    }

    fn build(&self, _ctx: &mut Context) -> Result<(), Error> {
        Ok(())
    }
}

#[async_trait(?Send)]
impl AsyncPlugin for BootPlugin {
    async fn build_async(&self, ctx: &mut Context) -> Result<(), Error> {
        let name = {
            let cfg = ctx.get::<Config>().expect("Config 应由 ConfigPlugin 提供");
            cfg.name
        };
        self.order.borrow_mut().push("async-boot");
        println!("→ BootPlugin::build_async — Config.name = {name}");
        Ok(())
    }
}

fn main() {
    println!("=== plugctx async-start example ===\n");

    let order = Rc::new(RefCell::new(Vec::new()));
    let ctx = Context::new();
    ctx.on(|_: &ReadyEvent| println!("✓ ReadyEvent"));

    ctx.plugin_async(BootPlugin {
        order: Rc::clone(&order),
    })
    .expect("install BootPlugin");
    ctx.plugin(ConfigPlugin).expect("install ConfigPlugin");

    println!("--- start_async() ---");
    block_on(async { ctx.start_async().await.expect("start_async") });

    assert_eq!(order.borrow().as_slice(), &["async-boot"]);
    ctx.dispose();
    println!("\n=== done ===");
}
