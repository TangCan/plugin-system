//! 统一错误类型（设计 §6.2.5 核心变体已冻结）。
//!
//! 用户指南：见工作区 `README.md`「生命周期与常见错误」与 `docs/api-freeze.md`。
//!
//! # 核心冻结变体（§6.2.5）
//!
//! | 变体 | 典型触发 |
//! |------|----------|
//! | [`AlreadyStarted`](Error::AlreadyStarted) | 对已 `start` 的上下文再次 `start` |
//! | [`AlreadyDisposed`](Error::AlreadyDisposed) | 销毁后再 `start` / `plugin` / `isolate`；Context 已毁后的 `PluginHandle::dispose` |
//! | [`MissingDependency`](Error::MissingDependency) | 插件 `dependencies` 声明的服务未被 `provide` |
//! | [`CircularDependency`](Error::CircularDependency) | 依赖成环，或乐观构建 ≥2 插件无法进展（卡住语义） |
//! | [`ServiceNotFound`](Error::ServiceNotFound) | 期望服务必存在的集成场景（当前 `get` 仍返回 `Option`） |
//! | [`PluginAlreadyDisposed`](Error::PluginAlreadyDisposed) | 上下文仍存活时插件句柄已卸载后再 `dispose` |
//! | [`BuildFailed`](Error::BuildFailed) | 插件 `build` 返回失败 |
//!
//! # 相对设计伪代码的偏差
//!
//! - 核心变体多为**单元变体**（无 `TypeId` 载荷），以保持 `Clone + PartialEq + Eq`，兼容既有验收断言。
//! - [`BuildFailed`](Error::BuildFailed) 不包裹 `Box<dyn std::error::Error>`（同因）。
//! - [`ServiceNotFound`](Error::ServiceNotFound) 预留稳定名；`Context::get` / `get_trait` **不**改为 `Result`。
//! - `Native*` / `Wasm*` / [`AbiMismatch`](Error::AbiMismatch) 等为扩展路径错误，超出 §6.2.5 最小列表但属于 FR35「等」字范围。

use std::path::PathBuf;

/// 框架可恢复错误（非 panic）。
///
/// 核心七变体对照设计 §6.2.5；完整公开 API 冻结说明见工作区 `docs/api-freeze.md`。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// 上下文已经启动，拒绝重复 `start`。
    #[error("上下文已经启动")]
    AlreadyStarted,
    /// 上下文已经销毁，拒绝写操作（如 `start` / `plugin` / `isolate`），
    /// 以及所属上下文已毁时的 [`crate::PluginHandle::dispose`]。
    ///
    /// 与 [`PluginAlreadyDisposed`](Error::PluginAlreadyDisposed) 的区别：本变体表示
    /// **Context 级**已销毁；后者表示 Context 仍存活但该插件条目已卸载。
    #[error("上下文已经销毁")]
    AlreadyDisposed,
    /// 插件 `build` 失败。
    #[error("插件构建失败")]
    BuildFailed,
    /// 插件声明的服务依赖未被提供（乐观构建剩余 1 个未满足插件）。
    ///
    /// 排查：确认上游插件已 `provide` 对应类型，或调整安装顺序 / `dependencies()`。
    #[error("缺少服务依赖")]
    MissingDependency,
    /// 插件依赖形成环，**或**乐观构建一轮后仍有 ≥2 个未构建插件同时无法进展
    ///（「卡住」语义，不一定是真环）。
    ///
    /// 典型场景：两个插件各自缺失**不同**服务、谁也无法先提供对方依赖——调用方会看到
    /// 本变体而非两个独立的 [`MissingDependency`](Error::MissingDependency)
    ///（后者仅在恰好剩余 1 个未满足插件时返回）。
    ///
    /// 排查：打破 `dependencies` 环；为卡住的插件补齐上游 `provide`；避免互相等待对方服务。
    #[error("检测到循环依赖或无法进展的依赖等待")]
    CircularDependency,
    /// 期望的服务未找到。
    ///
    /// 预留给「服务必须存在」的集成校验；当前 [`crate::Context::get`] /
    /// [`crate::Context::get_trait`] 仍返回 [`Option`]（见 `docs/api-freeze.md`）。
    #[error("服务未找到")]
    ServiceNotFound,
    /// 插件句柄已卸载，拒绝再次按作用域回滚。
    #[error("插件已经卸载")]
    PluginAlreadyDisposed,
    /// 动态库打开失败（`dynamic-native`）。
    #[error("加载动态库 `{path}` 失败: {message}", path = path.display())]
    NativeLoad { path: PathBuf, message: String },
    /// 缺少 `plugin_entry` 符号（`dynamic-native`）。
    #[error("动态库 `{path}` 缺少符号 `plugin_entry`: {message}", path = path.display())]
    NativeSymbol { path: PathBuf, message: String },
    /// ABI 版本不匹配；未执行 `create`/`init`（`dynamic-native` / FR24）。
    #[error("ABI 版本不匹配（`{path}`）: 插件={plugin}, 宿主={host}", path = path.display())]
    AbiMismatch {
        path: PathBuf,
        plugin: u32,
        host: u32,
    },
    /// 插件 `init` 失败。
    #[error("插件 `{name}` 初始化失败（status={status}）")]
    NativeInit { name: String, status: i32 },
    /// 插件 `call` 失败。
    #[error("插件 `{name}` 调用 `{op}` 失败（status={status}）")]
    NativeCall {
        name: String,
        op: String,
        status: i32,
    },
    /// 插件名非 UTF-8 或空指针。
    #[error("插件返回了无效名称")]
    NativeBadName,
    /// WASM 制品加载失败（`dynamic-wasm`）。
    #[error("加载 WASM 插件失败: {message}")]
    WasmLoad { message: String },
    /// WASM ABI 版本不匹配；未实例化（`dynamic-wasm` / FR24, NFR6）。
    #[error("WASM ABI 版本不匹配: 插件={plugin}, 宿主={host}")]
    WasmAbiMismatch { plugin: u32, host: u32 },
    /// WASM 实例调用失败（`dynamic-wasm`）。
    #[error("WASM 插件 `{name}` 调用 `{op}` 失败: {message}")]
    WasmCall {
        name: String,
        op: String,
        message: String,
    },
    /// WASM 实例已显式关闭（`dynamic-wasm` / FR26）。
    #[error("WASM 插件 `{name}` 实例已关闭")]
    WasmClosed { name: String },
}

/// 模块标识，供骨架可达性测试使用。
pub const MODULE_NAME: &str = "error";
