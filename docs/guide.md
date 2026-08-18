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
| native cdylib | `dynamic-native` | 先撤销注册，再 Drop `Library`（`dlclose` / `FreeLibrary`）。热插拔 = **load → use → dispose → load**，无 `reload()`。 |
| WASM | `dynamic-wasm` | 实例显式 `close`/`free`（FR26），不是 `dlclose` |
| WASM Component | `dynamic-wasm-component` | Drop Store（FR49），同样不是 native `dlclose` |

### Native 卸载限度（FR4）

`dispose` 成功、甚至 `dlclose` / `FreeLibrary` 返回成功，**不等于**一定能覆盖插件文件或映射已从进程消失：

1. **文件锁：** `FreeLibrary` / `dlclose` 成功 ≠ 文件一定可覆盖。Windows 上其它 `LoadLibrary`、PIN、依赖模块仍可能锁住 `.dll`；须先 dispose 或换路径再写文件。
2. **macOS TLS：** 带非平凡 TLS 析构的 dylib 在 macOS 上可能被标成**永不卸载**；`dlclose` 成功也不表示 unmap。
3. **sound 卸载：** 须无残留引用、出站函数指针、库内线程。否则即使 OS 报告卸载成功，再 load 也不安全。

不要把 `hot-lib-reloader` 一类文件监视工具当成生产公开 API。


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
