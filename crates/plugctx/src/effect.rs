//! 副作用登记与取消句柄。

use crate::context::Context;
use crate::shared::Flag;

/// 副作用取消句柄；调用 [`cancel`](EffectHandle::cancel) 后，对应 cleanup 在 `dispose` 时不再执行。
pub struct EffectHandle {
    pub(crate) ctx: Context,
    pub(crate) effect_id: usize,
    /// 与存储槽共享；`cancel` 时置位。
    pub(crate) cancelled: Flag,
}

impl EffectHandle {
    /// 取消该副作用的 cleanup；从列表移除且**不**执行清理闭包。句柄按值消费，仅可取消一次。
    pub fn cancel(self) {
        self.cancelled.set(true);
        self.ctx.cancel_effect(self.effect_id);
    }
}

/// 模块标识，供骨架可达性测试使用。
pub const MODULE_NAME: &str = "effect";
