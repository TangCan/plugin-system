//! 类型化事件与监听器取消句柄。

use std::any::TypeId;

use crate::context::Context;
use crate::shared::Flag;

/// 内核保证的就绪生命周期事件：`start()` 成功构建全部插件后恰好触发一次。
///
/// 扩展阶段事件（`InitEvent` / `PostStartEvent`）需启用 Cargo feature `stages`（设计 §4.7 / FR32）。
#[derive(Debug, Clone, Copy, Default)]
pub struct ReadyEvent;

/// 内核保证的销毁生命周期事件：`dispose()` 在 effects 逆序清理之前触发。
///
/// 通过现有 `emit` 路径分发（监听器列表先克隆再调用），监听器内重入不 panic。
/// 扩展阶段事件（`PreDisposeEvent`）需启用 Cargo feature `stages`（设计 §4.7 / FR32）。
#[derive(Debug, Clone, Copy, Default)]
pub struct DisposeEvent;

/// 扩展生命周期：开始构建插件**之前**触发（feature `stages`）。
///
/// 顺序：`InitEvent` → 构建 → [`ReadyEvent`] → [`PostStartEvent`]。
/// 构建失败时仍可能已发出本事件；此时不进入 Started，且不发 Ready/PostStart。
#[cfg(feature = "stages")]
#[derive(Debug, Clone, Copy, Default)]
pub struct InitEvent;

/// 扩展生命周期：[`ReadyEvent`] **之后**触发（feature `stages`）。
#[cfg(feature = "stages")]
#[derive(Debug, Clone, Copy, Default)]
pub struct PostStartEvent;

/// 扩展生命周期：[`DisposeEvent`] **之前**触发（feature `stages`）。
///
/// 顺序：`PreDisposeEvent` → [`DisposeEvent`] → effects 逆序 cleanup → 级联子上下文。
#[cfg(feature = "stages")]
#[derive(Debug, Clone, Copy, Default)]
pub struct PreDisposeEvent;

/// 事件监听器取消句柄；调用 [`cancel`](EventListenerHandle::cancel) 后后续 `emit` 不再触发该监听器。
pub struct EventListenerHandle {
    pub(crate) ctx: Context,
    pub(crate) type_id: TypeId,
    pub(crate) listener_id: usize,
    /// 与存储槽共享；`cancel` 时置位，即使本轮 emit 已克隆列表也可跳过。
    pub(crate) cancelled: Flag,
}

impl EventListenerHandle {
    /// 取消监听器；之后的 `emit` 不再调用。句柄按值消费，仅可取消一次。
    pub fn cancel(self) {
        self.cancelled.set(true);
        self.ctx
            .cancel_event_listener(self.type_id, self.listener_id);
    }
}

/// 模块标识，供骨架可达性测试使用。
pub const MODULE_NAME: &str = "event";
