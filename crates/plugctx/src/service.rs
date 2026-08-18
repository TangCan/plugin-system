//! 服务注册：以 `TypeId` 为键的依赖注入。
//!
//! 本模块文档说明 DI 约定；注册表实际存放在 [`crate::Context`] 内部。
//!
//! - `provide` / `get` / `get_mut`：具体类型服务；`provide` 仅写本级。
//! - `provide_trait` / `get_trait`：trait 对象服务；键为 `TypeId::of::<dyn Trait>()`，
//!   存于独立的 `trait_services` 表，与具体类型表隔离。
//! - `get` / `get_mut` / `get_trait`：本级未命中时沿 `isolate` 父链查找。
//! - 子级再次 `provide` / `provide_trait` 同键时覆盖本级视图，不污染父级。
//! - 插件 `build` 期间的注册会记入 [`crate::PluginScope`]，供精确卸载。

/// 模块标识，供骨架可达性测试使用。
pub const MODULE_NAME: &str = "service";
