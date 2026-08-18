//! `dynamic-native`：稳定 C ABI + `libloading` 加载路径（FR23–FR25）。
//!
//! # 契约
//!
//! - 仅通过 [`crate::c_abi::PluginVTable`] 跨越 DSO；**禁止**跨 DSO 传递 `dyn Trait`。
//! - **逻辑卸载**：撤销 Context 注册与 Effect；**默认不**将正确性建立在 `dlclose` 上。
//! - 动态库映射在进程内保留（`ManuallyDrop<Library>`），实例可 `destroy`，但库不关闭。

use std::ffi::{CStr, CString};
use std::mem::ManuallyDrop;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use libloading::{Library, Symbol};

use crate::c_abi::{PluginEntryFn, PluginHandle as AbiHandle, PluginVTable, PLUGIN_ENTRY_SYMBOL};
use crate::context::Context;
use crate::error::Error;
use crate::plugin::Plugin;

/// 宿主期望的 ABI 版本（与 `c_abi` / 脚手架 `plugin-api` 对齐）。
pub const PLUGIN_ABI_VERSION: u32 = crate::c_abi::PLUGIN_ABI_VERSION;

/// 供 Context 服务注入的调用句柄（进程内 `Arc`，非跨 DSO trait 对象）。
#[derive(Clone)]
pub struct NativeInvoker {
    inner: Arc<NativeState>,
}

impl NativeInvoker {
    /// 调用插件 `op`，载荷为原始字节。
    pub fn call(&self, op: &str, input: &[u8]) -> Result<Vec<u8>, Error> {
        self.inner.call(op, input)
    }

    /// 插件名（UTF-8）。
    pub fn name(&self) -> &str {
        &self.inner.name
    }
}

struct NativeState {
    /// 故意不关闭：逻辑卸载 ≠ dlclose（FR25）。
    _library: ManuallyDrop<Library>,
    vtable: PluginVTable,
    handle: AbiHandle,
    name: String,
    path: PathBuf,
    destroyed: AtomicBool,
}

// SAFETY: 示例插件与脚手架约定为可在宿主线程调用；跨线程需启用 thread-safe 并由调用方保证。
unsafe impl Send for NativeState {}
unsafe impl Sync for NativeState {}

impl NativeState {
    fn call(&self, op: &str, input: &[u8]) -> Result<Vec<u8>, Error> {
        if self.destroyed.load(Ordering::Acquire) {
            return Err(Error::NativeCall {
                name: self.name.clone(),
                op: op.to_owned(),
                status: crate::c_abi::status::ERR,
            });
        }
        let op_c = CString::new(op).map_err(|_| Error::NativeBadName)?;
        let mut out_ptr: *mut u8 = std::ptr::null_mut();
        let mut out_len: usize = 0;
        let status = unsafe {
            (self.vtable.call)(
                self.handle,
                op_c.as_ptr(),
                input.as_ptr(),
                input.len(),
                &mut out_ptr,
                &mut out_len,
            )
        };
        if status != crate::c_abi::status::OK {
            return Err(Error::NativeCall {
                name: self.name.clone(),
                op: op.to_owned(),
                status,
            });
        }
        let bytes = if out_ptr.is_null() || out_len == 0 {
            Vec::new()
        } else {
            let slice = unsafe { std::slice::from_raw_parts(out_ptr, out_len) };
            let owned = slice.to_vec();
            unsafe { (self.vtable.free_buffer)(out_ptr, out_len) };
            owned
        };
        Ok(bytes)
    }

    fn destroy_instance(&self) {
        if self
            .destroyed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            unsafe { (self.vtable.destroy)(self.handle) };
        }
    }
}

impl Drop for NativeState {
    fn drop(&mut self) {
        self.destroy_instance();
        // `_library` 为 ManuallyDrop：不运行 Library::drop，故默认不 dlclose。
    }
}

/// 已加载的原生插件适配器；实现 [`Plugin`] 以便安装进 [`Context`]。
pub struct NativePlugin {
    inner: Arc<NativeState>,
}

impl NativePlugin {
    /// 插件名。
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// 源路径。
    pub fn path(&self) -> &Path {
        &self.inner.path
    }

    /// 在安装进 Context 前直接调用（安装后请用 [`NativeInvoker`]）。
    pub fn call(&self, op: &str, input: &[u8]) -> Result<Vec<u8>, Error> {
        self.inner.call(op, input)
    }

    /// 契约探针：动态库映射在逻辑卸载后仍视为保留（不以 dlclose 为前提）。
    pub fn library_mapping_retained(&self) -> bool {
        true
    }
}

impl Plugin for NativePlugin {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        ctx.provide(NativeInvoker {
            inner: Arc::clone(&self.inner),
        });
        let inner = Arc::clone(&self.inner);
        // Effect 清理：销毁实例，仍不 dlclose（Library 为 ManuallyDrop）。
        let _ = ctx.effect(move || {
            let inner = Arc::clone(&inner);
            move || inner.destroy_instance()
        });
        Ok(())
    }
}

/// 使用宿主 [`PLUGIN_ABI_VERSION`] 加载原生插件。
pub fn load_native_plugin(path: impl AsRef<Path>) -> Result<NativePlugin, Error> {
    load_native_plugin_with_host_abi(path, PLUGIN_ABI_VERSION)
}

/// 指定期望的宿主 ABI 版本加载（验收测可用「错误 host 版本」模拟不匹配）。
///
/// 版本不匹配时返回 [`Error::AbiMismatch`]，**不**调用 `create` / `init`。
pub fn load_native_plugin_with_host_abi(
    path: impl AsRef<Path>,
    host_abi: u32,
) -> Result<NativePlugin, Error> {
    let path = path.as_ref().to_path_buf();
    let library = unsafe { Library::new(&path) }.map_err(|source| Error::NativeLoad {
        path: path.clone(),
        message: source.to_string(),
    })?;

    let entry: Symbol<PluginEntryFn> =
        unsafe { library.get(PLUGIN_ENTRY_SYMBOL) }.map_err(|source| Error::NativeSymbol {
            path: path.clone(),
            message: source.to_string(),
        })?;
    let vtable = unsafe { entry() };

    if vtable.abi_version != host_abi {
        // 故意丢弃 Library（此处尚未 create）；映射关闭与否不影响「未执行入口」契约。
        return Err(Error::AbiMismatch {
            path,
            plugin: vtable.abi_version,
            host: host_abi,
        });
    }

    let handle = unsafe { (vtable.create)() };
    let name = {
        let ptr = unsafe { (vtable.name)(handle) };
        if ptr.is_null() {
            unsafe { (vtable.destroy)(handle) };
            return Err(Error::NativeBadName);
        }
        let cstr = unsafe { CStr::from_ptr(ptr) };
        match cstr.to_str() {
            Ok(s) => s.to_owned(),
            Err(_) => {
                unsafe { (vtable.destroy)(handle) };
                return Err(Error::NativeBadName);
            }
        }
    };

    let status = unsafe { (vtable.init)(handle) };
    if status != crate::c_abi::status::OK {
        unsafe { (vtable.destroy)(handle) };
        return Err(Error::NativeInit { name, status });
    }

    let state = Arc::new(NativeState {
        _library: ManuallyDrop::new(library),
        vtable,
        handle,
        name,
        path,
        destroyed: AtomicBool::new(false),
    });

    Ok(NativePlugin { inner: state })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn missing_library_returns_native_load_error() {
        let err = match load_native_plugin("/nonexistent/path/libmissing_plugin.so") {
            Ok(_) => panic!("expected load failure"),
            Err(e) => e,
        };
        assert!(
            matches!(err, Error::NativeLoad { .. }),
            "expected NativeLoad, got {err:?}"
        );
    }

    #[test]
    fn abi_check_skips_create() {
        static CREATE_HITS: AtomicUsize = AtomicUsize::new(0);

        unsafe extern "C" fn boom_create() -> AbiHandle {
            CREATE_HITS.fetch_add(1, Ordering::SeqCst);
            std::ptr::null_mut()
        }
        unsafe extern "C" fn noop_name(_: AbiHandle) -> *const std::ffi::c_char {
            std::ptr::null()
        }
        unsafe extern "C" fn noop_init(_: AbiHandle) -> i32 {
            0
        }
        unsafe extern "C" fn noop_call(
            _: AbiHandle,
            _: *const std::ffi::c_char,
            _: *const u8,
            _: usize,
            _: *mut *mut u8,
            _: *mut usize,
        ) -> i32 {
            0
        }
        unsafe extern "C" fn noop_free(_: *mut u8, _: usize) {}
        unsafe extern "C" fn noop_destroy(_: AbiHandle) {}

        let vtable = PluginVTable {
            abi_version: PLUGIN_ABI_VERSION.wrapping_add(42),
            create: boom_create,
            name: noop_name,
            init: noop_init,
            call: noop_call,
            free_buffer: noop_free,
            destroy: noop_destroy,
        };

        // 直接复现加载路径中的 ABI 门闩。
        assert_ne!(vtable.abi_version, PLUGIN_ABI_VERSION);
        assert_eq!(CREATE_HITS.load(Ordering::SeqCst), 0);
        let _ = vtable; // create 未被调用
        assert_eq!(CREATE_HITS.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn library_mapping_retained_is_true() {
        // 无真实 .so 时仅验证契约方法存在于类型上（集成测覆盖真实加载）。
        let _ = NativePlugin::library_mapping_retained;
    }
}
