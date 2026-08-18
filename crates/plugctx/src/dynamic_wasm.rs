//! `dynamic-wasm`：WASM 插件实例加载与显式关闭（FR23, FR26）。
//!
//! # 引擎（Extism）
//!
//! 启用 feature `dynamic-wasm` 时接入 **Extism** 真实运行时（可选依赖，**不**进入
//! 默认工作区依赖图）。制品须为合法 WASM（魔数 `\0asm`），通常为 Extism PDK 编译产物。
//!
//! - 实例级显式 [`WasmPlugin::close`] / dispose Effect（FR26）会 `drop` Extism
//!   [`extism::Plugin`]，释放运行时资源；不仅依赖宿主句柄 Drop。
//! - ABI 协商：`WasmLoadConfig::abi_override`、制品 custom section `plugctx.abi`，
//!   缺省视为与宿主 [`WASM_ABI_VERSION`] 相同（NFR6）。
//! - 名称：`name_override` 或 custom section `plugctx.name`。
//!
//! 内置验收制品见 [`bundled_echo_wasm`]（`testdata/echo.wasm`，由 `wasm_echo` 重建）。
//!
//! # 并行模型与实例池
//!
//! NFR10：并行 fan-out 应发生在**宿主侧**，不假定 WASM guest 内多线程。
//!
//! ## 两层概念（FR46）
//!
//! | 层 | 含义 | 本模块 |
//! |----|------|--------|
//! | **逻辑 InstancePool** | 应用层有界 checkout / 超时 / 归还 reset / 显式 destroy | [`WasmInstancePool`] / [`WasmPoolConfig`] / [`WasmCheckoutGuard`] |
//! | **Wasmtime 资源 pooling** | 运行时 `PoolingAllocationConfig` 等内存/表槽复用 | **不**作为本 crate 公开 API |
//!
//! 启用 `dynamic-wasm` 后可用 [`WasmInstancePool`] 做有界 checkout；归还路径为
//! `reset` + 工厂重建；毒化实例用 [`WasmCheckoutGuard::destroy`]（不归还）。
//!
//! ## 线程安全（NFR11）
//!
//! - [`WasmInstancePool`] 可跨线程共享（`Clone` + 内部同步原语）。
//! - 每次 [`WasmInstancePool::checkout`] 得到的 [`WasmCheckoutGuard`] 对**单实例**拥有
//!   独占所有权：不得跨线程并发共享同一 Guard / 同一 Extism 插件实例。
//! - Guard **Drop 归还**（reset + 重建入队）；[`WasmCheckoutGuard::destroy`] **销毁不归还**。
//!
//! ## 与 `PluginHandle::dispose` / `WasmPlugin::close` 的语义对照（FR45）
//!
//! | 操作 | 作用对象 | 归还池？ | 效果 |
//! |------|----------|----------|------|
//! | [`WasmCheckoutGuard`] Drop | 池借出实例 | 是（+reset/重建） | 槽位保留，`live_count` 不变 |
//! | [`WasmCheckoutGuard::destroy`] | 池借出实例 | 否 | 释放实例，`live_count-1` |
//! | [`WasmPlugin::close`] | 非池单实例 | N/A | Extism `Plugin` drop（FR26） |
//! | [`crate::PluginHandle::dispose`] | Context 已安装插件 | N/A | 精确卸载插件条目 + effects；**不是** WASM 池 API |

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::context::Context;
use crate::error::Error;
use crate::plugin::Plugin;

/// 宿主期望的 WASM 宿主 ABI 版本（与制品 `plugctx.abi` / `abi_override` 协商；NFR6）。
pub const WASM_ABI_VERSION: u32 = 1;

const WASM_MAGIC: &[u8] = b"\0asm";

/// 加载配置。
#[derive(Debug, Clone, Default)]
pub struct WasmLoadConfig {
    /// 若设置，覆盖制品解析出的插件名。
    pub name_override: Option<String>,
    /// 若设置，覆盖制品解析出的插件 ABI（缺省 custom section 时也可用此强制插件侧版本）。
    pub abi_override: Option<u32>,
}

/// 供 Context 服务注入的调用句柄。
#[derive(Clone)]
pub struct WasmInvoker {
    inner: Arc<WasmState>,
}

impl WasmInvoker {
    /// 调用 Extism 导出函数 `op`（例如 PDK `echo`）。
    pub fn call(&self, op: &str, input: &[u8]) -> Result<Vec<u8>, Error> {
        self.inner.call(op, input)
    }

    pub fn name(&self) -> &str {
        &self.inner.name
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    pub fn runtime_freed(&self) -> bool {
        !self.inner.runtime_live.load(Ordering::Acquire)
    }

    /// 仅由显式 [`WasmState::close`] 递增（不含 Drop 兜底），供 FR26 回归测。
    pub fn explicit_close_count(&self) -> usize {
        self.inner.explicit_close_hits.load(Ordering::Acquire)
    }
}

struct WasmState {
    name: String,
    /// Extism 实例；显式 close / Drop 兜底时置 `None` 以释放运行时。
    engine: Mutex<Option<extism::Plugin>>,
    closed: AtomicBool,
    /// 显式 close 次数（Effect / API）；Drop 兜底不增加此计数。
    explicit_close_hits: AtomicUsize,
    runtime_live: AtomicBool,
    drop_fallback_hits: AtomicUsize,
}

impl WasmState {
    fn call(&self, op: &str, input: &[u8]) -> Result<Vec<u8>, Error> {
        if self.closed.load(Ordering::Acquire) {
            return Err(Error::WasmClosed {
                name: self.name.clone(),
            });
        }
        let mut guard = self.engine.lock().map_err(|_| Error::WasmCall {
            name: self.name.clone(),
            op: op.to_owned(),
            message: "引擎锁 poisoned".into(),
        })?;
        let plugin = guard.as_mut().ok_or_else(|| Error::WasmCall {
            name: self.name.clone(),
            op: op.to_owned(),
            message: "运行时资源已释放".into(),
        })?;
        plugin
            .call::<&[u8], Vec<u8>>(op, input)
            .map_err(|e| Error::WasmCall {
                name: self.name.clone(),
                op: op.to_owned(),
                message: e.to_string(),
            })
    }

    /// 实例级显式关闭（FR26）；幂等。
    fn close(&self) {
        if self
            .closed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.explicit_close_hits.fetch_add(1, Ordering::AcqRel);
            self.drop_engine();
        }
    }

    /// Drop 兜底：若忘记显式 close，仍释放运行时，但不计入 explicit_close_hits。
    fn close_from_drop(&self) {
        if self
            .closed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.drop_fallback_hits.fetch_add(1, Ordering::AcqRel);
            self.drop_engine();
        }
    }

    fn drop_engine(&self) {
        if let Ok(mut guard) = self.engine.lock() {
            *guard = None;
        }
        self.runtime_live.store(false, Ordering::Release);
    }
}

impl Drop for WasmState {
    fn drop(&mut self) {
        self.close_from_drop();
    }
}

/// 已加载的 WASM 插件适配器（Extism 实例）。
pub struct WasmPlugin {
    inner: Arc<WasmState>,
}

impl WasmPlugin {
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    pub fn call(&self, op: &str, input: &[u8]) -> Result<Vec<u8>, Error> {
        self.inner.call(op, input)
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    pub fn runtime_freed(&self) -> bool {
        !self.inner.runtime_live.load(Ordering::Acquire)
    }

    pub fn explicit_close_count(&self) -> usize {
        self.inner.explicit_close_hits.load(Ordering::Acquire)
    }

    /// 主动显式关闭（通常由 Context dispose Effect 调用）。
    pub fn close(&self) {
        self.inner.close();
    }
}

impl Plugin for WasmPlugin {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        ctx.provide(WasmInvoker {
            inner: Arc::clone(&self.inner),
        });
        let inner = Arc::clone(&self.inner);
        let _ = ctx.effect(move || {
            let inner = Arc::clone(&inner);
            move || inner.close()
        });
        Ok(())
    }
}

/// 内置 Extism PDK `echo` 制品（`testdata/echo.wasm`）。
pub fn bundled_echo_wasm() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/echo.wasm"))
}

/// WASM 逻辑实例池配置（FR43）。
#[derive(Debug, Clone)]
pub struct WasmPoolConfig {
    /// 池内最大实例数（含已借出 + 空闲）；不得无限创建。
    pub max_instances: usize,
}

impl Default for WasmPoolConfig {
    fn default() -> Self {
        Self {
            max_instances: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
        }
    }
}

type PluginSource = dyn Fn() -> Result<extism::Plugin, extism::Error> + Send + Sync;

struct PoolInner {
    plugin_source: Box<PluginSource>,
    available: VecDeque<extism::Plugin>,
    current_size: usize,
    max_size: usize,
}

/// 宿主侧逻辑 WASM 实例池（FR43–FR45）。
///
/// 与 Wasmtime `PoolingAllocationConfig`（运行时资源槽）不同——本类型是应用层
/// checkout / 超时借出 / 归还重置 / 显式销毁（FR46）。详见模块级「两层概念」与
/// `docs/feature-matrix.md`。
///
/// Extism 官方 `Pool` 的 Drop 仅归还不 `reset`、且无 destroy API，故此处自研有界池，
/// 工厂仍使用 `extism::Plugin::new`。
///
/// # 线程安全
///
/// 池本身可跨线程共享；[`checkout`](Self::checkout) 返回的 Guard 不可并发共享（NFR11）。
#[derive(Clone)]
pub struct WasmInstancePool {
    inner: Arc<Mutex<PoolInner>>,
    cond: Arc<Condvar>,
    max_instances: usize,
}

impl WasmInstancePool {
    /// 用制品字节与 [`WasmPoolConfig`] 创建池。
    ///
    /// `max_instances == 0` 视为无效配置。制品须为合法 WASM（与
    /// [`load_wasm_plugin`] 相同魔数约束）；工厂闭包在借出时按需实例化。
    pub fn new(artifact: impl AsRef<[u8]>, config: WasmPoolConfig) -> Result<Self, Error> {
        if config.max_instances == 0 {
            return Err(Error::WasmLoad {
                message: "WasmPoolConfig.max_instances 必须 ≥ 1".into(),
            });
        }
        let artifact = artifact.as_ref();
        if artifact.is_empty() {
            return Err(Error::WasmLoad {
                message: "WASM 制品为空".into(),
            });
        }
        if !artifact.starts_with(WASM_MAGIC) {
            return Err(Error::WasmLoad {
                message: "制品不是合法 WASM（需要魔数 \\0asm；InstancePool 使用 Extism）".into(),
            });
        }

        let bytes = artifact.to_vec();
        let inner = PoolInner {
            plugin_source: Box::new(move || extism::Plugin::new(bytes.clone(), [], false)),
            available: VecDeque::new(),
            current_size: 0,
            max_size: config.max_instances,
        };

        Ok(Self {
            inner: Arc::new(Mutex::new(inner)),
            cond: Arc::new(Condvar::new()),
            max_instances: config.max_instances,
        })
    }

    /// 配置的实例上界。
    pub fn max_instances(&self) -> usize {
        self.max_instances
    }

    /// 当前存活实例数（已借出 + 空闲）。
    pub fn live_count(&self) -> usize {
        self.inner.lock().expect("pool mutex").current_size
    }

    /// 带超时从池中借出实例。
    ///
    /// - `Ok(Some(guard))`：借出成功；Guard **Drop 归还**（reset + 工厂重建入队，FR44）。
    /// - `Ok(None)`：在 `timeout` 内无法获得空闲槽且已达 `max_instances`（FR43）。
    /// - `Err`：底层创建/引擎错误。
    ///
    /// 显式销毁见 [`WasmCheckoutGuard::destroy`]（不归还，FR45）。
    pub fn checkout(&self, timeout: Duration) -> Result<Option<WasmCheckoutGuard>, Error> {
        let start = std::time::Instant::now();
        let mut guard = self.inner.lock().map_err(|_| Error::WasmLoad {
            message: "WasmInstancePool 锁 poisoned".into(),
        })?;

        loop {
            if let Some(plugin) = guard.available.pop_front() {
                return Ok(Some(WasmCheckoutGuard {
                    plugin: Some(plugin),
                    pool: Arc::downgrade(&self.inner),
                    cond: Arc::clone(&self.cond),
                    return_to_pool: true,
                }));
            }

            if guard.current_size < guard.max_size {
                let plugin = (guard.plugin_source)().map_err(|e| Error::WasmLoad {
                    message: format!("Extism Pool 创建实例失败: {e}"),
                })?;
                guard.current_size += 1;
                return Ok(Some(WasmCheckoutGuard {
                    plugin: Some(plugin),
                    pool: Arc::downgrade(&self.inner),
                    cond: Arc::clone(&self.cond),
                    return_to_pool: true,
                }));
            }

            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return Ok(None);
            }
            let remaining = timeout - elapsed;
            let (g, wait_result) =
                self.cond
                    .wait_timeout(guard, remaining)
                    .map_err(|_| Error::WasmLoad {
                        message: "WasmInstancePool 等待被中断".into(),
                    })?;
            guard = g;
            if wait_result.timed_out() {
                return Ok(None);
            }
        }
    }
}

/// 从 [`WasmInstancePool`] 借出的实例守卫（FR44 / FR45）。
///
/// - **Drop**：归还池——先 `Plugin::reset`，再以工厂**重建**实例入空闲队列（文档化等价路径，
///   防止 guest static / 线性内存跨 checkout 泄漏；`live_count` 不变）。
/// - [`destroy`](Self::destroy)：显式销毁——**不**入池，释放 Extism 实例，`live_count-1`。
///
/// **独占**持有底层插件实例——不得跨线程并发共享同一 Guard（NFR11）。
///
/// 与 [`crate::PluginHandle::dispose`] 不同：后者卸载 Context 插件条目；本类型只管理池内
/// WASM 实例生命周期。
pub struct WasmCheckoutGuard {
    plugin: Option<extism::Plugin>,
    pool: std::sync::Weak<Mutex<PoolInner>>,
    cond: Arc<Condvar>,
    /// `true` = Drop 时归还；`destroy` 将其置 `false`。
    return_to_pool: bool,
}

impl WasmCheckoutGuard {
    /// Extism 插件实例 ID（字符串形式；用于断言归还复用路径 vs destroy 新建）。
    pub fn plugin_id(&self) -> String {
        self.plugin
            .as_ref()
            .expect("plugin is Some until Drop/destroy")
            .id
            .to_string()
    }

    /// 调用 Extism 导出函数（例如 PDK `echo` / `set_state` / `get_state`）。
    pub fn call(&mut self, op: &str, input: &[u8]) -> Result<Vec<u8>, Error> {
        self.plugin
            .as_mut()
            .expect("plugin is Some until Drop/destroy")
            .call::<&[u8], Vec<u8>>(op, input)
            .map_err(|e| Error::WasmCall {
                name: "wasm-pool".into(),
                op: op.to_owned(),
                message: e.to_string(),
            })
    }

    /// 显式销毁借出实例：不归还池，释放运行时资源（FR45）。
    ///
    /// 对齐单实例 [`WasmPlugin::close`] / `extism_plugin_free` 的「真销毁」语义；
    /// **不同于** Drop 归还，也**不同于** [`crate::PluginHandle::dispose`]（Context 插件卸载）。
    pub fn destroy(mut self) {
        self.return_to_pool = false;
        if let Some(plugin) = self.plugin.take() {
            drop(plugin);
            if let Some(inner) = self.pool.upgrade() {
                if let Ok(mut guard) = inner.lock() {
                    if guard.current_size > 0 {
                        guard.current_size -= 1;
                    }
                }
                self.cond.notify_one();
            }
        }
    }
}

impl Drop for WasmCheckoutGuard {
    fn drop(&mut self) {
        let Some(mut plugin) = self.plugin.take() else {
            return;
        };
        let Some(inner) = self.pool.upgrade() else {
            // 池已销毁：直接丢弃借出实例
            return;
        };

        if !self.return_to_pool {
            // destroy() 已处理递减；防御性兜底
            return;
        }

        // FR44：先 soft reset，再工厂重建入队（等价硬复位，清 guest static）
        let _ = plugin.reset();
        drop(plugin);

        let mut guard = match inner.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        match (guard.plugin_source)() {
            Ok(fresh) => {
                guard.available.push_back(fresh);
            }
            Err(_) => {
                // 重建失败：释放槽位，避免占满无法再借
                if guard.current_size > 0 {
                    guard.current_size -= 1;
                }
            }
        }
        drop(guard);
        self.cond.notify_one();
    }
}

/// 在合法 WASM 上附加 `plugctx.name` / `plugctx.abi` custom sections（验收/演示用）。
pub fn wasm_artifact_with_meta(wasm: &[u8], name: &str, abi: u32) -> Vec<u8> {
    let mut out = wasm.to_vec();
    out = append_custom_section(out, "plugctx.name", name.as_bytes());
    out = append_custom_section(out, "plugctx.abi", abi.to_string().as_bytes());
    out
}

/// 使用宿主 [`WASM_ABI_VERSION`] 加载 WASM 插件（Extism）。
pub fn load_wasm_plugin(
    artifact: impl AsRef<[u8]>,
    config: WasmLoadConfig,
) -> Result<WasmPlugin, Error> {
    load_wasm_plugin_with_host_abi(artifact, config, WASM_ABI_VERSION)
}

/// 指定期望的宿主 WASM ABI 版本加载（验收测可用「错误 host 版本」模拟不匹配）。
///
/// 版本不匹配时返回 [`Error::WasmAbiMismatch`]，**不**实例化 Extism 插件。
pub fn load_wasm_plugin_with_host_abi(
    artifact: impl AsRef<[u8]>,
    config: WasmLoadConfig,
    host_abi: u32,
) -> Result<WasmPlugin, Error> {
    let artifact = artifact.as_ref();
    if artifact.is_empty() {
        return Err(Error::WasmLoad {
            message: "WASM 制品为空".into(),
        });
    }
    if !artifact.starts_with(WASM_MAGIC) {
        return Err(Error::WasmLoad {
            message: "制品不是合法 WASM（需要魔数 \\0asm；dynamic-wasm 现为 Extism 真实引擎）"
                .into(),
        });
    }

    let sections = parse_custom_sections(artifact);
    let plugin_abi = config
        .abi_override
        .or_else(|| {
            sections
                .iter()
                .find(|(n, _)| n == "plugctx.abi")
                .and_then(|(_, v)| std::str::from_utf8(v).ok()?.parse().ok())
        })
        .unwrap_or(WASM_ABI_VERSION);

    if plugin_abi != host_abi {
        return Err(Error::WasmAbiMismatch {
            plugin: plugin_abi,
            host: host_abi,
        });
    }

    let parsed_name = sections
        .iter()
        .find(|(n, _)| n == "plugctx.name")
        .and_then(|(_, v)| {
            let s = std::str::from_utf8(v).ok()?.trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_owned())
            }
        });

    let name = config
        .name_override
        .or(parsed_name)
        .unwrap_or_else(|| "wasm-plugin".into());

    if name.is_empty() {
        return Err(Error::WasmLoad {
            message: "插件名为空".into(),
        });
    }

    // ABI 已协商通过后再实例化，避免不匹配时泄漏运行时。
    let plugin = extism::Plugin::new(artifact, [], false).map_err(|e| Error::WasmLoad {
        message: format!("Extism 加载失败: {e}"),
    })?;

    let state = Arc::new(WasmState {
        name,
        engine: Mutex::new(Some(plugin)),
        closed: AtomicBool::new(false),
        explicit_close_hits: AtomicUsize::new(0),
        runtime_live: AtomicBool::new(true),
        drop_fallback_hits: AtomicUsize::new(0),
    });

    Ok(WasmPlugin { inner: state })
}

fn append_custom_section(mut module: Vec<u8>, name: &str, data: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    write_u32_leb128(&mut payload, name.len() as u32);
    payload.extend_from_slice(name.as_bytes());
    payload.extend_from_slice(data);
    module.push(0); // custom section id
    write_u32_leb128(&mut module, payload.len() as u32);
    module.extend(payload);
    module
}

fn write_u32_leb128(buf: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn read_u32_leb128(input: &[u8], mut offset: usize) -> Option<(u32, usize)> {
    let mut result: u32 = 0;
    let mut shift = 0;
    loop {
        let byte = *input.get(offset)?;
        offset += 1;
        result |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some((result, offset));
        }
        shift += 7;
        if shift > 28 {
            return None;
        }
    }
}

/// 扫描模块 custom sections（容错：解析失败则返回已收集部分）。
fn parse_custom_sections(wasm: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    if wasm.len() < 8 || !wasm.starts_with(WASM_MAGIC) {
        return out;
    }
    let mut offset = 8; // skip magic + version
    while offset < wasm.len() {
        let id = match wasm.get(offset) {
            Some(b) => *b,
            None => break,
        };
        offset += 1;
        let (size, next) = match read_u32_leb128(wasm, offset) {
            Some(v) => v,
            None => break,
        };
        offset = next;
        let end = match offset.checked_add(size as usize) {
            Some(e) if e <= wasm.len() => e,
            _ => break,
        };
        if id == 0 {
            let section = &wasm[offset..end];
            if let Some((name_len, after_len)) = read_u32_leb128(section, 0) {
                let name_end = after_len + name_len as usize;
                if name_end <= section.len() {
                    if let Ok(name) = std::str::from_utf8(&section[after_len..name_end]) {
                        out.push((name.to_owned(), section[name_end..].to_vec()));
                    }
                }
            }
        }
        offset = end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(name: &str) -> Vec<u8> {
        wasm_artifact_with_meta(bundled_echo_wasm(), name, WASM_ABI_VERSION)
    }

    #[test]
    fn empty_artifact_errors() {
        let err = match load_wasm_plugin([], WasmLoadConfig::default()) {
            Ok(_) => panic!("expected load failure"),
            Err(e) => e,
        };
        assert!(matches!(err, Error::WasmLoad { .. }));
    }

    #[test]
    fn non_wasm_artifact_rejected() {
        let err = match load_wasm_plugin(b"plugctx-wasm-mock\nname=x\n", WasmLoadConfig::default())
        {
            Ok(_) => panic!("expected load failure"),
            Err(e) => e,
        };
        assert!(matches!(err, Error::WasmLoad { .. }));
    }

    #[test]
    fn abi_mismatch_rejects_before_instantiate() {
        let err = match load_wasm_plugin_with_host_abi(
            sample("x"),
            WasmLoadConfig::default(),
            WASM_ABI_VERSION.wrapping_add(9),
        ) {
            Ok(_) => panic!("expected abi mismatch"),
            Err(e) => e,
        };
        match err {
            Error::WasmAbiMismatch { plugin, host } => {
                assert_eq!(plugin, 1);
                assert_eq!(host, WASM_ABI_VERSION.wrapping_add(9));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn missing_abi_section_defaults_to_host_version() {
        let plugin = load_wasm_plugin(bundled_echo_wasm(), WasmLoadConfig::default()).unwrap();
        assert_eq!(plugin.name(), "wasm-plugin");
        let out = plugin.call("echo", b"hi").unwrap();
        assert_eq!(out, b"hi");
    }

    #[test]
    fn explicit_artifact_abi_must_match_host() {
        let err = match load_wasm_plugin(
            wasm_artifact_with_meta(bundled_echo_wasm(), "bad", WASM_ABI_VERSION.wrapping_add(1)),
            WasmLoadConfig::default(),
        ) {
            Ok(_) => panic!("expected mismatch"),
            Err(e) => e,
        };
        assert!(matches!(err, Error::WasmAbiMismatch { .. }));
    }

    #[test]
    fn close_is_idempotent_and_blocks_call() {
        let plugin = load_wasm_plugin(sample("x"), WasmLoadConfig::default()).unwrap();
        plugin.close();
        plugin.close();
        assert_eq!(plugin.explicit_close_count(), 1);
        assert!(plugin.is_closed());
        assert!(plugin.runtime_freed());
        assert!(matches!(
            plugin.call("echo", b"a"),
            Err(Error::WasmClosed { .. })
        ));
    }

    #[test]
    fn close_via_api_is_explicit_not_drop_fallback() {
        let plugin = load_wasm_plugin(sample("x"), WasmLoadConfig::default()).unwrap();
        plugin.close();
        assert_eq!(plugin.explicit_close_count(), 1);
        assert_eq!(
            plugin.inner.drop_fallback_hits.load(Ordering::Acquire),
            0,
            "显式 close 不应计入 Drop 兜底"
        );
        assert!(plugin.runtime_freed());
    }

    #[test]
    fn abi_override_forces_plugin_side_version() {
        let err = match load_wasm_plugin(
            bundled_echo_wasm(),
            WasmLoadConfig {
                name_override: None,
                abi_override: Some(WASM_ABI_VERSION.wrapping_add(5)),
            },
        ) {
            Ok(_) => panic!("expected mismatch via abi_override"),
            Err(e) => e,
        };
        match err {
            Error::WasmAbiMismatch { plugin, host } => {
                assert_eq!(plugin, WASM_ABI_VERSION.wrapping_add(5));
                assert_eq!(host, WASM_ABI_VERSION);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn name_override_wins() {
        let plugin = load_wasm_plugin(
            sample("from-artifact"),
            WasmLoadConfig {
                name_override: Some("override".into()),
                abi_override: None,
            },
        )
        .unwrap();
        assert_eq!(plugin.name(), "override");
    }

    /// Automate 护栏：未走显式 close 时，Drop 兜底路径计入 drop_fallback，不计 explicit。
    #[test]
    fn drop_fallback_without_explicit_close() {
        let plugin = load_wasm_plugin(sample("drop-only"), WasmLoadConfig::default()).unwrap();
        let inner = Arc::clone(&plugin.inner);
        drop(plugin);
        assert_eq!(Arc::strong_count(&inner), 1);
        assert!(!inner.closed.load(Ordering::Acquire));
        assert_eq!(inner.explicit_close_hits.load(Ordering::Acquire), 0);

        inner.close_from_drop();
        assert!(inner.closed.load(Ordering::Acquire));
        assert_eq!(inner.explicit_close_hits.load(Ordering::Acquire), 0);
        assert_eq!(inner.drop_fallback_hits.load(Ordering::Acquire), 1);
        assert!(!inner.runtime_live.load(Ordering::Acquire));
        assert!(inner.engine.lock().unwrap().is_none());
    }

    #[test]
    fn real_extism_echo_roundtrip() {
        let plugin = load_wasm_plugin(sample("roundtrip"), WasmLoadConfig::default()).unwrap();
        assert_eq!(plugin.name(), "roundtrip");
        let out = plugin.call("echo", b"ping-extism").unwrap();
        assert_eq!(out, b"ping-extism");
    }

    /// Automate：max_instances=0 拒绝建池。
    #[test]
    fn pool_rejects_zero_max_instances() {
        let err =
            match WasmInstancePool::new(bundled_echo_wasm(), WasmPoolConfig { max_instances: 0 }) {
                Ok(_) => panic!("zero max should fail"),
                Err(e) => e,
            };
        assert!(matches!(err, Error::WasmLoad { .. }));
    }

    /// Automate：空制品拒绝建池。
    #[test]
    fn pool_rejects_empty_artifact() {
        let err = match WasmInstancePool::new([], WasmPoolConfig { max_instances: 1 }) {
            Ok(_) => panic!("empty should fail"),
            Err(e) => e,
        };
        assert!(matches!(err, Error::WasmLoad { .. }));
    }

    /// Automate：串行借出归还后 live_count 不增长超界，并可再次 call。
    #[test]
    fn pool_serial_checkout_and_return() {
        let pool = WasmInstancePool::new(bundled_echo_wasm(), WasmPoolConfig { max_instances: 1 })
            .unwrap();
        {
            let mut g = pool
                .checkout(Duration::from_secs(1))
                .unwrap()
                .expect("checkout");
            assert_eq!(pool.live_count(), 1);
            assert_eq!(g.call("echo", b"a").unwrap(), b"a");
        }
        assert_eq!(pool.live_count(), 1, "归还后实例仍计为存活空闲");
        let mut g2 = pool
            .checkout(Duration::from_millis(200))
            .unwrap()
            .expect("reuse");
        assert_eq!(g2.call("echo", b"b").unwrap(), b"b");
    }

    /// Automate：destroy 后 live_count 下降，且不得静默把毒化实例放回。
    #[test]
    fn pool_destroy_decrements_live_count() {
        let pool = WasmInstancePool::new(bundled_echo_wasm(), WasmPoolConfig { max_instances: 1 })
            .unwrap();
        let g = pool
            .checkout(Duration::from_secs(1))
            .unwrap()
            .expect("checkout");
        assert_eq!(pool.live_count(), 1);
        let id = g.plugin_id();
        g.destroy();
        assert_eq!(pool.live_count(), 0, "destroy must free the slot");

        let mut g2 = pool
            .checkout(Duration::from_millis(500))
            .unwrap()
            .expect("new instance after destroy");
        assert_ne!(g2.plugin_id(), id, "destroyed instance must not be reused");
        assert_eq!(g2.call("echo", b"x").unwrap(), b"x");
    }

    /// Automate：归还后 guest 状态不可见（reset + 重建）。
    #[test]
    fn pool_return_clears_guest_state() {
        let pool = WasmInstancePool::new(bundled_echo_wasm(), WasmPoolConfig { max_instances: 1 })
            .unwrap();
        {
            let mut g = pool
                .checkout(Duration::from_secs(1))
                .unwrap()
                .expect("checkout");
            g.call("set_state", b"poison").unwrap();
            assert_eq!(g.call("get_state", b"").unwrap(), b"poison");
        }
        let mut g2 = pool
            .checkout(Duration::from_millis(500))
            .unwrap()
            .expect("after return");
        assert_eq!(
            g2.call("get_state", b"").unwrap(),
            b"",
            "guest state must not leak across return"
        );
    }
}
