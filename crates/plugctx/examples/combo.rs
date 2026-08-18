//! 组合场景示例（Story 5.2 / FR29）
//!
//! 演示核心概念：
//! - **Context**：生命周期入口（`new` → `start` → `dispose`）
//! - **Plugin**：≥2 个插件安装；依赖排序后构建
//! - **服务注入（DI）**：`provide` / `get`
//! - **事件**：`on` / `emit`（含内置 `ReadyEvent`）
//! - **Effect**：setup 立即执行，cleanup 在 `dispose` 时**逆序**执行
//!
//! 运行：
//! ```bash
//! cd plugin-system
//! cargo run -p plugctx --example combo
//! ```

use std::any::TypeId;
use std::cell::RefCell;
use std::rc::Rc;

use plugctx::{Context, Error, Plugin, ReadyEvent};

/// 日志服务（由 LoggerPlugin provide，供 AppPlugin get）。
#[derive(Debug)]
struct Logger {
    name: &'static str,
}

impl Logger {
    fn info(&self, msg: &str) {
        println!("[{}] {msg}", self.name);
    }
}

/// 业务事件：插件组合后可在 start 之后继续 emit。
#[derive(Debug)]
struct PingEvent {
    msg: &'static str,
}

/// Plugin A：提供 Logger，并登记一个 Effect。
struct LoggerPlugin;

impl Plugin for LoggerPlugin {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        println!("→ LoggerPlugin::build — provide(Logger) + effect");
        ctx.provide(Logger {
            name: "combo-logger",
        });

        // Effect：setup 立即跑；cleanup 在 Context::dispose 时逆序执行
        let _ = ctx.effect(|| {
            println!("  effect[A] setup");
            || println!("  effect[A] cleanup")
        });

        Ok(())
    }
}

/// Plugin B：依赖 Logger，监听/触发业务事件，再登记一个 Effect。
struct AppPlugin {
    log: Rc<RefCell<Vec<&'static str>>>,
}

impl Plugin for AppPlugin {
    fn dependencies(&self) -> Vec<TypeId> {
        vec![TypeId::of::<Logger>()]
    }

    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        println!("→ AppPlugin::build — get::<Logger>() + on(PingEvent) + effect");

        {
            // ServiceRef 持有内部借用：必须在调用 on/effect 前 drop
            let logger = ctx
                .get::<Logger>()
                .expect("Logger 应由 LoggerPlugin 先 provide（依赖排序）");
            logger.info("AppPlugin 已注入 Logger");
        }

        let log = Rc::clone(&self.log);
        ctx.on(move |e: &PingEvent| {
            log.borrow_mut().push(e.msg);
            println!("  on(PingEvent): {}", e.msg);
        });

        let _ = ctx.effect(|| {
            println!("  effect[B] setup");
            || println!("  effect[B] cleanup")
        });

        Ok(())
    }
}

fn main() {
    println!("=== plugctx combo example (FR29) ===\n");

    // Context：框架根对象
    let ctx = Context::new();
    let ping_log = Rc::new(RefCell::new(Vec::new()));

    // 监听内置 ReadyEvent（start 成功路径触发）
    ctx.on(|_e: &ReadyEvent| {
        println!("✓ ReadyEvent — Context::start 成功");
    });

    // 故意先装消费者再装提供者，展示依赖排序
    ctx.plugin(AppPlugin {
        log: Rc::clone(&ping_log),
    })
    .expect("install AppPlugin");
    ctx.plugin(LoggerPlugin).expect("install LoggerPlugin");

    println!("\n--- start() ---");
    ctx.start().expect("start");

    println!("\n--- emit(PingEvent) ---");
    ctx.emit(&PingEvent {
        msg: "ping-from-main",
    });
    assert_eq!(ping_log.borrow().as_slice(), &["ping-from-main"]);

    println!("\n--- dispose()（观察 effect 逆序 cleanup：B 然后 A）---");
    ctx.dispose();

    println!("\n=== done ===");
}
