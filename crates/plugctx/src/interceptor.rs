//! ContextInterceptor：插件 build / 事件 emit 前后的同步横切钩子（FR16）。
//!
//! # 对象安全约定
//!
//! 设计伪代码曾使用 `before_emit<E>` / `after_emit<E>` 泛型方法；为实现
//! `Box`/`Rc<dyn ContextInterceptor>`，emit 钩子改为 `&dyn Any`。调用方可用
//! `downcast_ref` / `is` 识别具体事件类型。此为相对设计文档的**刻意偏离**。
//!
//! # 调用约定
//!
//! - 多拦截器：`before_*` / `after_*` 均按**注册序**（FIFO）调用。
//! - 插件 `build`：**成功**后才调用 `after_plugin_build`；失败仅调用过 `before_plugin_build`。
//! - 子上下文不继承父级拦截器。
//! - 钩子内允许有限重入（`provide` / `on` / `emit` 等）；框架在调用前快照列表并释放借用。

use std::any::Any;

use crate::plugin::Plugin;

/// 上下文拦截器：在插件构建与事件触发前后插入横切逻辑。
///
/// 所有方法默认空实现；按需覆盖。方法应为同步、不 panic。
/// 启用 `thread-safe` 时附加 `Send + Sync`（FR22）。
#[cfg(not(feature = "thread-safe"))]
pub trait ContextInterceptor {
    /// 单个插件 `build` 即将执行前调用。
    fn before_plugin_build(&self, _plugin: &dyn Plugin) {}

    /// 单个插件 `build` **成功**后调用；失败路径不调用。
    fn after_plugin_build(&self, _plugin: &dyn Plugin) {}

    /// `emit` 分发监听器之前调用；`event` 为类型擦除的事件引用。
    fn before_emit(&self, _event: &dyn Any) {}

    /// `emit` 分发监听器之后调用（无监听器时仍会调用）。
    fn after_emit(&self, _event: &dyn Any) {}
}

/// 上下文拦截器（`thread-safe`：`Send + Sync`）。
#[cfg(feature = "thread-safe")]
pub trait ContextInterceptor: Send + Sync {
    /// 单个插件 `build` 即将执行前调用。
    fn before_plugin_build(&self, _plugin: &dyn Plugin) {}

    /// 单个插件 `build` **成功**后调用；失败路径不调用。
    fn after_plugin_build(&self, _plugin: &dyn Plugin) {}

    /// `emit` 分发监听器之前调用；`event` 为类型擦除的事件引用。
    fn before_emit(&self, _event: &dyn Any) {}

    /// `emit` 分发监听器之后调用（无监听器时仍会调用）。
    fn after_emit(&self, _event: &dyn Any) {}
}
