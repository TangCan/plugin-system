//! `plugctx` — 最小同步插件框架核心 crate。
//!
//! # 生命周期要点
//!
//! - [`Context::new`] 创建未启动上下文；[`Context::start`] 按依赖序构建插件并触发 [`ReadyEvent`]。
//! - [`Context::dispose`] 触发 [`DisposeEvent`]、逆序执行 effect cleanup，并级联销毁子上下文（幂等）。
//! - 已 dispose 后 [`Context::isolate`] → [`Error::AlreadyDisposed`]。
//! - 启用 feature `stages` 时额外触发 Init/PostStart/PreDispose 事件（设计 §4.7）。
//! - 销毁窗口内 `provide`/`on`/`effect` 边界见工作区 `docs/dispose-registration-window.md`。
//!
//! # 常见错误（§6.2.5 核心已冻结）
//!
//! - 重复 `start` → [`Error::AlreadyStarted`]；销毁后写操作 → [`Error::AlreadyDisposed`]
//! - 依赖未满足 → [`Error::MissingDependency`]；成环/无法进展 → [`Error::CircularDependency`]
//! - 插件 `build` 失败 → [`Error::BuildFailed`]；句柄二次卸载 → [`Error::PluginAlreadyDisposed`]
//! - 预留：[`Error::ServiceNotFound`]（`get` 仍返回 [`Option`]）
//!
//! 完整变体与相对设计偏差见 [`Error`] 与工作区 `docs/api-freeze.md`。
//!
//! # Feature 矩阵
//!
//! 默认同步内核无需 feature。可选：`async`、`parallel`、`thread-safe`、`dynamic-native`、
//! `dynamic-wasm`（含 Extism 单实例加载与有界 `WasmInstancePool` checkout）、
//! `dynamic-wasm-component`（`wasmtime::component`；与 Extism 经 `PluginBackend` 分路径共存、分制品）、
//! `tracing`（诊断 span 门面；不引入订阅端）、`stages`（扩展生命周期事件）。
//! 发布切片（0.1.0 / 0.2.0）与设计对齐：工作区 `CHANGELOG.md`、`docs/feature-matrix.md`；
//! Component Model 版本钉死见 `docs/component-model-versions.md`；另见 `README.md` 与 `docs/testing.md`。
//! crates.io：`plugctx` / `plugctx-derive` `0.1.1` 已上架（0.1.0 仍保留）。元数据与 `publish = false` 边界见工作区 `docs/publishing.md`（FR51）。

#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(feature = "async")]
pub mod async_plugin;
/// 稳定 C ABI 布局（`dynamic-native`；与脚手架 `plugin-api` 同源）。
#[cfg(feature = "dynamic-native")]
pub mod c_abi;
pub mod context;
#[cfg(any(
    feature = "dynamic-native",
    feature = "dynamic-wasm",
    feature = "dynamic-wasm-component"
))]
pub mod dynamic;
#[cfg(feature = "dynamic-native")]
pub mod dynamic_native;
#[cfg(feature = "dynamic-wasm")]
pub mod dynamic_wasm;
#[cfg(feature = "dynamic-wasm-component")]
pub mod dynamic_wasm_component;
pub mod effect;
pub mod error;
pub mod event;
pub mod interceptor;
pub mod plugin;
pub mod service;
pub(crate) mod shared;

#[cfg(feature = "async")]
pub use async_plugin::AsyncPlugin;
pub use context::Context;
#[cfg(feature = "dynamic-wasm-component")]
pub use dynamic::ComponentLoader;
#[cfg(feature = "dynamic-native")]
pub use dynamic::DylibLoader;
#[cfg(feature = "dynamic-wasm")]
pub use dynamic::WasmLoader;
#[cfg(any(
    feature = "dynamic-native",
    feature = "dynamic-wasm",
    feature = "dynamic-wasm-component"
))]
pub use dynamic::{DynamicLoader, DynamicSource};
#[cfg(any(feature = "dynamic-wasm", feature = "dynamic-wasm-component"))]
pub use dynamic::{PluginBackend, PluginBackendKind};
#[cfg(feature = "dynamic-native")]
pub use dynamic_native::{
    load_native_plugin, load_native_plugin_with_host_abi, NativeInvoker, NativePlugin,
    PLUGIN_ABI_VERSION,
};
#[cfg(feature = "dynamic-wasm")]
pub use dynamic_wasm::{
    bundled_echo_wasm, load_wasm_plugin, load_wasm_plugin_with_host_abi, wasm_artifact_with_meta,
    WasmCheckoutGuard, WasmInstancePool, WasmInvoker, WasmLoadConfig, WasmPlugin, WasmPoolConfig,
    WASM_ABI_VERSION,
};
#[cfg(feature = "dynamic-wasm-component")]
pub use dynamic_wasm_component::{
    bundled_component_add_wat, bundled_wit_sample_add_wasm, load_wasm_component, ComponentInvoker,
    ComponentPlugin,
};
pub use effect::EffectHandle;
pub use error::Error;
pub use event::{DisposeEvent, EventListenerHandle, ReadyEvent};
#[cfg(feature = "stages")]
pub use event::{InitEvent, PostStartEvent, PreDisposeEvent};
pub use interceptor::ContextInterceptor;
pub use plugin::{Plugin, PluginHandle, PluginId, PluginScope};
pub use shared::{ServiceMut, ServiceRef};

/// Crate 标识，供冒烟测试与诊断使用。
pub const CRATE_NAME: &str = "plugctx";
