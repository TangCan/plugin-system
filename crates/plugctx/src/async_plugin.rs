//! 异步插件扩展（`async` feature）：[`AsyncPlugin`] 与安装入口。
//!
//! 设计 §4.1：框架本身不绑定具体异步运行时；由调用方提供（tokio / async-std 等）。
//!
//! # 安装约定（相对设计示意的刻意偏离）
//!
//! Rust 无法从 `Box<dyn Plugin>` 可靠探测是否实现了 [`AsyncPlugin`]。
//! 因此异步插件须通过 [`Context::plugin_async`] 安装，以便 `start_async` 调用
//! `build_async`；仅用 [`Context::plugin`] 安装的条目在 `start_async` 中走同步 `build`。

use async_trait::async_trait;

use crate::context::Context;
use crate::error::Error;
use crate::plugin::Plugin;

/// 异步构建扩展：在 `start_async` 路径上替代同步 [`Plugin::build`]。
///
/// 默认使用 `async_trait(?Send)`（单线程 `Rc` Context）。
/// 与 `thread-safe` 组合时改为 `Send` Future（FR22）。
#[cfg(not(feature = "thread-safe"))]
#[async_trait(?Send)]
pub trait AsyncPlugin: Plugin {
    /// 异步构建插件：注册服务、监听事件、创建副作用等。
    async fn build_async(&self, ctx: &mut Context) -> Result<(), Error>;
}

/// 异步构建扩展（`thread-safe`：`Send`）。
#[cfg(feature = "thread-safe")]
#[async_trait]
pub trait AsyncPlugin: Plugin {
    /// 异步构建插件：注册服务、监听事件、创建副作用等。
    async fn build_async(&self, ctx: &mut Context) -> Result<(), Error>;
}

/// 模块标识。
pub const MODULE_NAME: &str = "async_plugin";
