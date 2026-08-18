//! Example plugin that answers `greet` with `hello, <payload>`.

use std::ffi::{c_char, CStr, CString};

use plugin_api::{
    alloc_output, free_output, status, PluginHandle, PluginVTable, PLUGIN_ABI_VERSION,
};

struct HelloPlugin {
    name: CString,
}

impl HelloPlugin {
    fn new() -> Self {
        Self {
            name: CString::new("hello").expect("static name"),
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
    Box::into_raw(Box::new(HelloPlugin::new())) as PluginHandle
}

unsafe extern "C" fn name(handle: PluginHandle) -> *const c_char {
    let plugin = unsafe { &*(handle as *const HelloPlugin) };
    plugin.name.as_ptr()
}

unsafe extern "C" fn init(_handle: PluginHandle) -> i32 {
    status::OK
}

unsafe extern "C" fn call(
    handle: PluginHandle,
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
    if op != "greet" {
        return status::UNSUPPORTED;
    }
    let payload = if input.is_null() || input_len == 0 {
        ""
    } else {
        match std::str::from_utf8(unsafe { std::slice::from_raw_parts(input, input_len) }) {
            Ok(s) => s,
            Err(_) => return status::ERR,
        }
    };
    let msg = if payload.is_empty() {
        "hello".to_owned()
    } else {
        format!("hello, {payload}")
    };
    let (ptr, len) = alloc_output(msg.as_bytes());
    unsafe {
        *output = ptr;
        *output_len = len;
    }
    let _ = handle;
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
        drop(Box::from_raw(handle as *mut HelloPlugin));
    }
}
