//! 统一动态加载入口（FR34 / FR48 / 设计 §6.2）。
//!
//! `DynamicLoader` 供 native / Extism WASM / Component Model 适配器共享同一宿主契约；
//! 具体加载仍委托 [`crate::dynamic_native`] / [`crate::dynamic_wasm`] /
//! [`crate::dynamic_wasm_component`]，本模块不做二次实现。
//!
//! # 双后端（FR48）
//!
//! [`PluginBackend`] 在启用对应 feature 时统一 Extism 与 Component 路径。
//! **客人制品必须分路径编译**：Extism PDK `.wasm` 与 Component Model 组件**二进制不兼容**，
//! 不得假设同一 `.wasm` 可两吃；需要共享逻辑协议时另建适配层。

use std::path::Path;

use crate::error::Error;
use crate::plugin::Plugin;

/// 动态插件来源：路径或原始字节（epic AC：路径/字节）。
#[derive(Debug, Clone, Copy)]
pub enum DynamicSource<'a> {
    /// 文件系统路径（native `.so` / `.dylib` / `.dll`，或 WASM / 组件制品文件）。
    Path(&'a Path),
    /// 内存中的制品字节（主要用于 WASM / 组件 / 嵌入式载荷）。
    Bytes(&'a [u8]),
}

/// 统一动态插件加载契约（与 Interceptor / AsyncPlugin 同级扩展 API）。
pub trait DynamicLoader {
    /// 加载外部插件，返回可经 [`crate::Context::plugin`] 安装的适配器。
    ///
    /// 失败时返回可诊断 [`Error`]；**不得**半初始化调用方的 [`crate::Context`]。
    fn load(&self, source: DynamicSource<'_>) -> Result<Box<dyn Plugin>, Error>;
}

/// WASM / 组件后端种类（FR48）。
///
/// 仅当对应 Cargo feature 启用时变体才存在；用于按插件选择后端，而非混用制品。
#[cfg(any(feature = "dynamic-wasm", feature = "dynamic-wasm-component"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginBackendKind {
    /// Extism PDK 路径（feature `dynamic-wasm`）。
    #[cfg(feature = "dynamic-wasm")]
    Extism,
    /// `wasmtime::component` 路径（feature `dynamic-wasm-component`）。
    #[cfg(feature = "dynamic-wasm-component")]
    Component,
}

/// 统一后端入口：按后端加载**对应**制品 → [`Box<dyn Plugin>`]（FR48）。
///
/// # 分制品
///
/// - [`PluginBackend::Extism`] 只接受 Extism PDK（或本仓库 `echo.wasm` 类）制品。
/// - [`PluginBackend::Component`] 只接受 Component Model 组件（或 WAT）制品。
/// - **禁止**把同一 `.wasm` 字节同时交给两个后端并期望成功。
#[cfg(any(feature = "dynamic-wasm", feature = "dynamic-wasm-component"))]
#[derive(Debug, Clone)]
pub enum PluginBackend {
    /// Extism 动态加载器。
    #[cfg(feature = "dynamic-wasm")]
    Extism(WasmLoader),
    /// Component Model 动态加载器。
    #[cfg(feature = "dynamic-wasm-component")]
    Component(ComponentLoader),
}

#[cfg(any(feature = "dynamic-wasm", feature = "dynamic-wasm-component"))]
impl PluginBackend {
    /// 后端种类。
    pub fn kind(&self) -> PluginBackendKind {
        match self {
            #[cfg(feature = "dynamic-wasm")]
            Self::Extism(_) => PluginBackendKind::Extism,
            #[cfg(feature = "dynamic-wasm-component")]
            Self::Component(_) => PluginBackendKind::Component,
        }
    }

    /// 按所选后端加载制品。
    pub fn load(&self, source: DynamicSource<'_>) -> Result<Box<dyn Plugin>, Error> {
        DynamicLoader::load(self, source)
    }
}

#[cfg(any(feature = "dynamic-wasm", feature = "dynamic-wasm-component"))]
impl DynamicLoader for PluginBackend {
    fn load(&self, source: DynamicSource<'_>) -> Result<Box<dyn Plugin>, Error> {
        match self {
            #[cfg(feature = "dynamic-wasm")]
            Self::Extism(loader) => loader.load(source),
            #[cfg(feature = "dynamic-wasm-component")]
            Self::Component(loader) => loader.load(source),
        }
    }
}

/// 原生动态库加载器（`dynamic-native`）：委托 [`crate::load_native_plugin`]。
#[cfg(feature = "dynamic-native")]
#[derive(Debug, Default, Clone, Copy)]
pub struct DylibLoader;

#[cfg(feature = "dynamic-native")]
impl DynamicLoader for DylibLoader {
    fn load(&self, source: DynamicSource<'_>) -> Result<Box<dyn Plugin>, Error> {
        match source {
            DynamicSource::Path(path) => {
                let plugin = crate::dynamic_native::load_native_plugin(path)?;
                Ok(Box::new(plugin))
            }
            DynamicSource::Bytes(_) => Err(Error::NativeLoad {
                path: Path::new("<bytes>").to_path_buf(),
                message: "DylibLoader 仅支持 DynamicSource::Path，不支持内存字节".into(),
            }),
        }
    }
}

/// WASM 动态加载器（`dynamic-wasm`）：委托 [`crate::load_wasm_plugin`]。
#[cfg(feature = "dynamic-wasm")]
#[derive(Debug, Clone, Default)]
pub struct WasmLoader {
    /// 传给 [`crate::load_wasm_plugin`] 的配置。
    pub config: crate::dynamic_wasm::WasmLoadConfig,
}

#[cfg(feature = "dynamic-wasm")]
impl WasmLoader {
    pub fn new(config: crate::dynamic_wasm::WasmLoadConfig) -> Self {
        Self { config }
    }
}

#[cfg(feature = "dynamic-wasm")]
impl DynamicLoader for WasmLoader {
    fn load(&self, source: DynamicSource<'_>) -> Result<Box<dyn Plugin>, Error> {
        let bytes: Vec<u8> = match source {
            DynamicSource::Bytes(b) => b.to_vec(),
            DynamicSource::Path(path) => std::fs::read(path).map_err(|e| Error::WasmLoad {
                message: format!("读取 WASM 制品 `{}` 失败: {e}", path.display()),
            })?,
        };
        let plugin = crate::dynamic_wasm::load_wasm_plugin(bytes, self.config.clone())?;
        Ok(Box::new(plugin))
    }
}

/// Component Model 动态加载器（`dynamic-wasm-component`）：委托 [`crate::load_wasm_component`]。
#[cfg(feature = "dynamic-wasm-component")]
#[derive(Debug, Default, Clone, Copy)]
pub struct ComponentLoader;

#[cfg(feature = "dynamic-wasm-component")]
impl DynamicLoader for ComponentLoader {
    fn load(&self, source: DynamicSource<'_>) -> Result<Box<dyn Plugin>, Error> {
        let bytes: Vec<u8> = match source {
            DynamicSource::Bytes(b) => b.to_vec(),
            DynamicSource::Path(path) => std::fs::read(path).map_err(|e| Error::WasmLoad {
                message: format!("读取组件制品 `{}` 失败: {e}", path.display()),
            })?,
        };
        let plugin = crate::dynamic_wasm_component::load_wasm_component(bytes)?;
        Ok(Box::new(plugin))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "dynamic-native")]
    #[test]
    fn dylib_loader_rejects_bytes() {
        let err = match DylibLoader.load(DynamicSource::Bytes(b"x")) {
            Ok(_) => panic!("expected rejection"),
            Err(e) => e,
        };
        assert!(matches!(err, Error::NativeLoad { .. }));
    }

    #[cfg(feature = "dynamic-wasm")]
    #[test]
    fn wasm_loader_rejects_empty_bytes() {
        let err = match WasmLoader::default().load(DynamicSource::Bytes(&[])) {
            Ok(_) => panic!("expected empty artifact error"),
            Err(e) => e,
        };
        assert!(matches!(err, Error::WasmLoad { .. }));
    }

    #[cfg(feature = "dynamic-wasm")]
    #[test]
    fn wasm_loader_missing_path_errors() {
        let err = match WasmLoader::default().load(DynamicSource::Path(Path::new(
            "/nonexistent/plugctx_wasm_missing.artifact",
        ))) {
            Ok(_) => panic!("expected missing path error"),
            Err(e) => e,
        };
        assert!(matches!(err, Error::WasmLoad { .. }));
    }

    /// Automate 护栏：经 `&dyn DynamicLoader` 调度仍可加载（统一契约多态）。
    #[cfg(all(feature = "dynamic-native", feature = "dynamic-wasm"))]
    #[test]
    fn load_via_trait_object_dispatch() {
        let wasm: &dyn DynamicLoader = &WasmLoader::default();
        let artifact = crate::dynamic_wasm::wasm_artifact_with_meta(
            crate::dynamic_wasm::bundled_echo_wasm(),
            "trait-obj",
            crate::dynamic_wasm::WASM_ABI_VERSION,
        );
        let boxed = wasm
            .load(DynamicSource::Bytes(&artifact))
            .expect("trait object load");
        let ctx = crate::Context::new();
        ctx.plugin(boxed).expect("install");
        ctx.start().expect("start");
        assert!(ctx.get::<crate::dynamic_wasm::WasmInvoker>().is_some());
    }

    #[cfg(feature = "dynamic-wasm-component")]
    #[test]
    fn component_loader_loads_bundled_wat() {
        let wat = crate::dynamic_wasm_component::bundled_component_add_wat();
        let boxed = ComponentLoader
            .load(DynamicSource::Bytes(wat.as_bytes()))
            .expect("component load");
        let ctx = crate::Context::new();
        ctx.plugin(boxed).expect("install");
        ctx.start().expect("start");
        assert_eq!(
            ctx.get::<crate::dynamic_wasm_component::ComponentInvoker>()
                .expect("invoker")
                .call_add(1, 2)
                .expect("add"),
            3
        );
    }

    #[cfg(all(feature = "dynamic-wasm", feature = "dynamic-wasm-component"))]
    #[test]
    fn plugin_backend_kind_discriminates() {
        let extism = PluginBackend::Extism(WasmLoader::default());
        let component = PluginBackend::Component(ComponentLoader);
        assert_eq!(extism.kind(), PluginBackendKind::Extism);
        assert_eq!(component.kind(), PluginBackendKind::Component);
    }
}
