# plugctx 用户指南

面向应用作者：不读 `docs/requirements/` 也能从最小插件走到 native 热插拔和三条场景示例。

Feature 全表见 [`feature-matrix.md`](feature-matrix.md)。发布见 [`publishing.md`](publishing.md)。

## 安装

crates.io 当前版本 **0.1.1**：[`plugctx`](https://crates.io/crates/plugctx) / [`plugctx-derive`](https://crates.io/crates/plugctx-derive)。已上架 README 绑在该版本 `.crate`；改展示文案须升版本（见 [`publishing.md`](publishing.md)）。

```bash
cargo add plugctx
cargo add plugctx-derive   # 可选：#[derive(Plugin)]
```

本仓库内开发请用 workspace path，不必 `cargo add`。

## 最小同步插件

```rust
use plugctx::{Context, Error, Plugin};

struct Hello;

impl Plugin for Hello {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        ctx.provide("hi".to_string());
        Ok(())
    }
}

fn main() {
    let ctx = Context::new();
    ctx.plugin(Hello).unwrap();
    ctx.start().unwrap();
    println!("{}", ctx.get::<String>().unwrap().as_str());
    ctx.dispose();
}
```

流程：`Context::new` → `plugin` → `start` → 业务 → `dispose`。

## Native 热插拔 vs WASM

| 路径 | Feature | 卸载 |
| --- | --- | --- |
| 进程内 `Plugin` | 默认 | `PluginHandle::dispose` 撤销注册 |
| native cdylib | `dynamic-native` | 先撤销注册，再 Drop `Library`（`dlclose`）。热插拔 = **load → use → dispose → load**，无 `reload()`。Windows 若文件仍被锁，先 dispose 或换路径。 |
| WASM | `dynamic-wasm` | 实例显式 `close`/`free`（FR26），不是 `dlclose` |

## 场景示例命令

在仓库根目录：

```bash
# CLI：调用 hello 插件后物理卸载（须先编 cdylib）
cargo build -p hello_plugin
cargo run -p plugctx-examples --example cli-hotplug --features native

# Web：tiny_http；默认自请求一次后退出。常驻见示例文件头注释。
cargo run -p plugctx-examples --example web-service --features web
# 常驻时：
# PLUGCTX_WEB_SELFTEST=0 cargo run -p plugctx-examples --example web-service --features web
# curl -s http://127.0.0.1:3000/

# 游戏：无引擎固定 tick，卸载插件后计数停止
cargo run -p plugctx-examples --example game-loop
```

`plugctx-examples` 为 `publish = false`。不要把 `tiny_http` / `libloading` / 游戏引擎加进 `plugctx` 的默认 features。
