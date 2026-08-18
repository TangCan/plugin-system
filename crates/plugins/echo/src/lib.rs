//! Example plugin that echoes the input payload for op `echo`.

use std::ffi::{c_char, CStr, CString};

use plugin_api::{
    alloc_output, free_output, status, PluginHandle, PluginVTable, PLUGIN_ABI_VERSION,
};

struct EchoPlugin {
    name: CString,
}

impl EchoPlugin {
    fn new() -> Self {
        Self {
            name: CString::new("echo").expect("static name"),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_entry() -> PluginVTable {
    PluginVTable {
        abi_version: PLUGIN_ABI_VERSION,
        create,
        name,
        init,
        call,
        free_buffer,
        destroy,
    }
}

unsafe extern "C" fn create() -> PluginHandle {
    Box::into_raw(Box::new(EchoPlugin::new())) as PluginHandle
}

unsafe extern "C" fn name(handle: PluginHandle) -> *const c_char {
    let plugin = unsafe { &*(handle as *const EchoPlugin) };
    plugin.name.as_ptr()
}

unsafe extern "C" fn init(_handle: PluginHandle) -> i32 {
    status::OK
}

unsafe extern "C" fn call(
    _handle: PluginHandle,
    op: *const c_char,
    input: *const u8,
    input_len: usize,
    output: *mut *mut u8,
    output_len: *mut usize,
) -> i32 {
    if op.is_null() || output.is_null() || output_len.is_null() {
        return status::ERR;
    }
    let op = match unsafe { CStr::from_ptr(op) }.to_str() {
        Ok(s) => s,
        Err(_) => return status::ERR,
    };
    if op != "echo" {
        return status::UNSUPPORTED;
    }
    let bytes = if input.is_null() || input_len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(input, input_len) }
    };
    let (ptr, len) = alloc_output(bytes);
    unsafe {
        *output = ptr;
        *output_len = len;
    }
    status::OK
}

unsafe extern "C" fn free_buffer(buf: *mut u8, len: usize) {
    unsafe { free_output(buf, len) };
}

unsafe extern "C" fn destroy(handle: PluginHandle) {
    if handle.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(handle as *mut EchoPlugin));
    }
}
