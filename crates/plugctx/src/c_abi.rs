// 稳定 C ABI 布局（与脚手架 `plugin-api` 同源；`dynamic-native` 自包含以便 crates.io 发布）。
//
// `plugin-api`（`publish = false`）通过 `include!` 复用本文件，避免宿主/示例与核心 ABI 漂移。
// 使用普通 `//` 注释：本文件会被 `include!`，不可使用 `//!` 内联文档。

use std::ffi::c_char;
use std::os::raw::c_void;

/// Bump when the vtable layout or semantics change incompatibly.
pub const PLUGIN_ABI_VERSION: u32 = 1;

/// Exported symbol name every plugin `cdylib` must provide.
pub const PLUGIN_ENTRY_SYMBOL: &[u8] = b"plugin_entry\0";

/// Result codes returned across the ABI.
pub mod status {
    pub const OK: i32 = 0;
    pub const ERR: i32 = 1;
    pub const UNSUPPORTED: i32 = 2;
}

/// Opaque plugin instance owned by the plugin library.
pub type PluginHandle = *mut c_void;

/// Function pointer types for [`PluginVTable`].
pub type PluginNameFn = unsafe extern "C" fn(handle: PluginHandle) -> *const c_char;
pub type PluginInitFn = unsafe extern "C" fn(handle: PluginHandle) -> i32;
pub type PluginCallFn = unsafe extern "C" fn(
    handle: PluginHandle,
    op: *const c_char,
    input: *const u8,
    input_len: usize,
    output: *mut *mut u8,
    output_len: *mut usize,
) -> i32;
pub type PluginFreeFn = unsafe extern "C" fn(buf: *mut u8, len: usize);
pub type PluginDestroyFn = unsafe extern "C" fn(handle: PluginHandle);

/// C-compatible vtable returned by `plugin_entry`.
#[repr(C)]
pub struct PluginVTable {
    pub abi_version: u32,
    pub create: unsafe extern "C" fn() -> PluginHandle,
    pub name: PluginNameFn,
    pub init: PluginInitFn,
    pub call: PluginCallFn,
    pub free_buffer: PluginFreeFn,
    pub destroy: PluginDestroyFn,
}

/// Type of the exported `plugin_entry` symbol.
pub type PluginEntryFn = unsafe extern "C" fn() -> PluginVTable;

/// Allocate an output buffer that the host will free via [`PluginVTable::free_buffer`].
pub fn alloc_output(bytes: &[u8]) -> (*mut u8, usize) {
    let mut vec = bytes.to_vec().into_boxed_slice();
    let len = vec.len();
    let ptr = vec.as_mut_ptr();
    std::mem::forget(vec);
    (ptr, len)
}

/// Free a buffer previously produced by [`alloc_output`].
///
/// # Safety
/// `buf` must be null or a pointer from [`alloc_output`] with matching `len`.
pub unsafe fn free_output(buf: *mut u8, len: usize) {
    if buf.is_null() || len == 0 {
        return;
    }
    // SAFETY: caller guarantees the allocation came from `alloc_output`.
    unsafe {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(buf, len)));
    }
}
