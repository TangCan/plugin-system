//! Plugin trait、句柄、作用域与内部条目表示。
//!
//! 插件条目由上下文侧的 `slotmap::SlotMap` 持有；[`PluginHandle`] 通过稳定
//! [`PluginId`] 定位，删除后旧键失效，避免误指新条目（设计 §7.4 / FR36）。
//!
//! [`PluginScope`] 记录单个插件在 `build` 期间的注册，供后续精确卸载（设计 §3.6）。
//!
//! 与设计 §6.2.1 / §6.2.3 的冻结对照见工作区 `docs/api-freeze.md`。

use std::any::TypeId;

use slotmap::new_key_type;

#[cfg(feature = "async")]
use crate::async_plugin::AsyncPlugin;
use crate::context::Context;
use crate::error::Error;

new_key_type! {
    /// 插件稳定 ID（`slotmap` 键；删除后失效，不会误用到新条目）。
    pub struct PluginId;
}

/// 插件构建期间自动捕获的注册信息（设计 §3.6.1）。
///
/// 仅在 [`Plugin::build`] 执行期间、且当前上下文的作用域栈非空时写入。
/// 根级（非插件 build）的 `provide`/`on`/`effect`/`isolate` **不计入**任何 scope。
#[derive(Clone, Debug, Default)]
pub struct PluginScope {
    /// 本插件 `provide` 的具体类型服务 `TypeId`（按调用顺序）。
    pub provided_services: Vec<TypeId>,
    /// 本插件 `provide_trait` 的 trait 服务 `TypeId`（按调用顺序）。
    pub provided_trait_services: Vec<TypeId>,
    /// 本插件 `on` 登记：`(事件 TypeId, 当时在该事件列表中的下标)`。
    pub registered_events: Vec<(TypeId, usize)>,
    /// 本插件 `on_async` 登记：`(事件 TypeId, 当时在该异步事件列表中的下标)`。
    pub registered_async_events: Vec<(TypeId, usize)>,
    /// 本插件第一个 effect 在全局 `effects` 中的起始下标（push 前的 `len`）。
    pub effects_start: usize,
    /// 本插件登记的 effect 数量。
    pub effects_count: usize,
    /// 本插件第一个子上下文在全局 `children` 中的起始下标（首次 `isolate` 前的 `len`）。
    pub children_start: usize,
    /// 本插件 `isolate` 创建的子上下文数量。
    pub children_count: usize,
}

/// 插件构建契约。
///
/// 启用 `thread-safe` 时附加 `Send + Sync`（FR22）。
#[cfg(not(feature = "thread-safe"))]
pub trait Plugin {
    /// 构建插件：注册服务、监听事件、创建副作用等。
    fn build(&self, ctx: &mut Context) -> Result<(), Error>;

    /// 声明依赖的服务类型。默认无依赖。
    fn dependencies(&self) -> Vec<std::any::TypeId> {
        vec![]
    }
}

/// 插件构建契约（`thread-safe`：`Send + Sync`）。
#[cfg(feature = "thread-safe")]
pub trait Plugin: Send + Sync {
    /// 构建插件：注册服务、监听事件、创建副作用等。
    fn build(&self, ctx: &mut Context) -> Result<(), Error>;

    /// 声明依赖的服务类型。默认无依赖。
    fn dependencies(&self) -> Vec<std::any::TypeId> {
        vec![]
    }
}

/// 允许 `DynamicLoader` 返回的 `Box<dyn Plugin>` 直接传入 [`Context::plugin`]。
impl<T: Plugin + ?Sized> Plugin for Box<T> {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        (**self).build(ctx)
    }

    fn dependencies(&self) -> Vec<std::any::TypeId> {
        (**self).dependencies()
    }
}

/// 用户持有的插件控制器。
#[derive(Clone)]
pub struct PluginHandle {
    pub(crate) ctx: Context,
    pub(crate) plugin_id: PluginId,
}

impl std::fmt::Debug for PluginHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginHandle")
            .field("plugin_id", &self.plugin_id)
            .finish()
    }
}

impl PluginHandle {
    /// 按构建期 [`PluginScope`] 精确卸载本插件，不影响同上下文其他插件（FR15）。
    ///
    /// - 首次成功返回 `Ok(())`，条目从 SlotMap 移除（稳定键失效）。
    /// - 所属 [`crate::Context`] 已完全销毁时返回 [`Error::AlreadyDisposed`]
    ///   （上下文级；优先于插件级错误）。
    /// - 上下文仍存活但条目已不存在（再次 dispose）时返回
    ///   [`Error::PluginAlreadyDisposed`]（不 panic、不误删兄弟插件）。
    /// - 未构建（无 scope）时仅移除条目。
    ///
    /// - 服务「被后续插件或根级覆盖则不误删」按 §5.3 / FR33：仅当前所有者为本插件时移除。
    pub fn dispose(&self) -> Result<(), Error> {
        self.ctx.dispose_plugin(self.plugin_id)
    }

    /// 插件稳定 ID。
    pub fn id(&self) -> PluginId {
        self.plugin_id
    }

    /// 该句柄对应的插件条目是否仍存在于上下文中。
    pub fn is_alive(&self) -> bool {
        self.ctx.contains_plugin(self.plugin_id)
    }

    /// 已成功构建时返回该插件的 [`PluginScope`] 快照；未构建或条目已移除则为 `None`。
    pub fn scope(&self) -> Option<PluginScope> {
        self.ctx.plugin_scope(self.plugin_id)
    }
}

/// 上下文内部持有的插件对象（同步 / 异步分轨存储）。
///
/// 启用 `async` 时，经 [`crate::context::Context::plugin_async`] 安装的条目走
/// `Async` 变体，以便 `start_async` 调用 `build_async`。
pub(crate) enum StoredPlugin {
    Sync(Box<dyn Plugin>),
    #[cfg(feature = "async")]
    Async(Box<dyn AsyncPlugin>),
}

impl StoredPlugin {
    pub(crate) fn as_plugin(&self) -> &dyn Plugin {
        match self {
            Self::Sync(p) => p.as_ref(),
            #[cfg(feature = "async")]
            Self::Async(p) => p.as_ref(),
        }
    }

    pub(crate) fn dependencies(&self) -> Vec<TypeId> {
        self.as_plugin().dependencies()
    }
}

/// 上下文内部插件条目（`plugin` 在 `build` 期间临时取出以避免 RefCell 重入）。
pub(crate) struct PluginEntry {
    pub(crate) plugin: Option<StoredPlugin>,
    pub(crate) built: bool,
    /// 构建成功后保存的作用域记录；未构建或构建失败为 `None`。
    pub(crate) scope: Option<PluginScope>,
}

/// 模块标识，供骨架可达性测试使用。
pub const MODULE_NAME: &str = "plugin";
