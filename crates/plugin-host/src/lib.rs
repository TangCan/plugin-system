//! Host-side dynamic loader for [`plugin_api`] plugins.

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use libloading::{Library, Symbol};
use plugin_api::{
    PluginEntryFn, PluginHandle, PluginVTable, PLUGIN_ABI_VERSION, PLUGIN_ENTRY_SYMBOL,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HostError {
    #[error("failed to load library `{path}`: {source}")]
    Load {
        path: PathBuf,
        #[source]
        source: libloading::Error,
    },
    #[error("missing symbol `plugin_entry` in `{path}`: {source}")]
    Symbol {
        path: PathBuf,
        #[source]
        source: libloading::Error,
    },
    #[error("ABI mismatch in `{path}`: plugin={plugin}, host={host}")]
    AbiMismatch {
        path: PathBuf,
        plugin: u32,
        host: u32,
    },
    #[error("plugin `{name}` init failed (status={status})")]
    Init { name: String, status: i32 },
    #[error("plugin `{name}` call `{op}` failed (status={status})")]
    Call {
        name: String,
        op: String,
        status: i32,
    },
    #[error("plugin returned non-utf8 name")]
    BadName,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// One loaded plugin instance. Dropping it destroys the guest handle and closes the library.
pub struct LoadedPlugin {
    _library: Arc<Library>,
    vtable: PluginVTable,
    handle: PluginHandle,
    name: String,
    path: PathBuf,
}

// SAFETY: plugins are required to be thread-safe for this scaffold; host only calls
// them from the owning thread in the demo CLI.
unsafe impl Send for LoadedPlugin {}

impl LoadedPlugin {
    /// Load `path` (a `cdylib`), check ABI, create + init the plugin.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, HostError> {
        let path = path.as_ref().to_path_buf();
        // SAFETY: we only use the library while `self` is alive.
        let library = unsafe { Library::new(&path) }.map_err(|source| HostError::Load {
            path: path.clone(),
            source,
        })?;
        let library = Arc::new(library);

        let entry: Symbol<PluginEntryFn> =
            unsafe { library.get(PLUGIN_ENTRY_SYMBOL) }.map_err(|source| HostError::Symbol {
                path: path.clone(),
                source,
            })?;
        let vtable = unsafe { entry() };
        if vtable.abi_version != PLUGIN_ABI_VERSION {
            return Err(HostError::AbiMismatch {
                path,
                plugin: vtable.abi_version,
                host: PLUGIN_ABI_VERSION,
            });
        }

        let handle = unsafe { (vtable.create)() };
        let name = {
            let ptr = unsafe { (vtable.name)(handle) };
            if ptr.is_null() {
                return Err(HostError::BadName);
            }
            // SAFETY: plugin must return a NUL-terminated static or instance-owned C string.
            let cstr = unsafe { CStr::from_ptr(ptr) };
            cstr.to_str().map_err(|_| HostError::BadName)?.to_owned()
        };

        let status = unsafe { (vtable.init)(handle) };
        if status != plugin_api::status::OK {
            unsafe { (vtable.destroy)(handle) };
            return Err(HostError::Init { name, status });
        }

        Ok(Self {
            _library: library,
            vtable,
            handle,
            name,
            path,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Invoke `op` with UTF-8 / raw byte payload.
    pub fn call(&self, op: &str, input: &[u8]) -> Result<Vec<u8>, HostError> {
        let op_c = CString::new(op).map_err(|_| HostError::BadName)?;
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
        if status != plugin_api::status::OK {
            return Err(HostError::Call {
                name: self.name.clone(),
                op: op.to_owned(),
                status,
            });
        }
        let bytes = if out_ptr.is_null() || out_len == 0 {
            Vec::new()
        } else {
            // SAFETY: plugin allocated via plugin-api::alloc_output.
            let slice = unsafe { std::slice::from_raw_parts(out_ptr, out_len) };
            let owned = slice.to_vec();
            unsafe { (self.vtable.free_buffer)(out_ptr, out_len) };
            owned
        };
        Ok(bytes)
    }
}

impl Drop for LoadedPlugin {
    fn drop(&mut self) {
        unsafe { (self.vtable.destroy)(self.handle) };
    }
}

/// Discover `*.so` / `*.dylib` / `*.dll` under `dir` (non-recursive) and load each.
pub fn load_dir(dir: impl AsRef<Path>) -> Result<Vec<LoadedPlugin>, HostError> {
    let dir = dir.as_ref();
    let mut plugins = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let is_plugin = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| matches!(ext, "so" | "dylib" | "dll"));
        if !is_plugin {
            continue;
        }
        plugins.push(LoadedPlugin::load(&path)?);
    }
    plugins.sort_by(|a, b| a.name().cmp(b.name()));
    Ok(plugins)
}

#[cfg(test)]
mod tests {
    use std::sync::Once;

    use super::*;

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn plugin_lib_file(name: &str) -> String {
        if cfg!(target_os = "windows") {
            format!("{name}.dll")
        } else if cfg!(target_os = "macos") {
            format!("lib{name}.dylib")
        } else {
            format!("lib{name}.so")
        }
    }

    fn target_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
            roots.push(PathBuf::from(dir));
        }
        roots.push(workspace_root().join("target"));
        roots.push(PathBuf::from("target"));
        roots
    }

    fn find_built_plugin(name: &str) -> Option<PathBuf> {
        let file = plugin_lib_file(name);
        for root in target_roots() {
            for profile in ["debug", "release"] {
                let p = root.join(profile).join(&file);
                if p.exists() {
                    return Some(p);
                }
            }
        }
        None
    }

    /// `cargo test` only builds unit-test harnesses for cdylib crates, not the final
    /// `target/*/lib*.so`. Build the example plugins once when artifacts are missing.
    fn ensure_example_plugins_built() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            if find_built_plugin("hello_plugin").is_some()
                && find_built_plugin("echo_plugin").is_some()
            {
                return;
            }
            let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
            let status = std::process::Command::new(cargo)
                .args(["build", "-p", "hello_plugin", "-p", "echo_plugin"])
                .current_dir(workspace_root())
                .status()
                .expect("spawn cargo build for example plugins");
            assert!(
                status.success(),
                "cargo build -p hello_plugin -p echo_plugin failed with {status}"
            );
        });
    }

    fn example_lib(name: &str) -> PathBuf {
        ensure_example_plugins_built();
        find_built_plugin(name).unwrap_or_else(|| {
            panic!(
                "built plugin `{}` not found after cargo build -p {name}",
                plugin_lib_file(name)
            )
        })
    }

    #[test]
    fn loads_hello_and_echo() {
        let hello = LoadedPlugin::load(example_lib("hello_plugin")).expect("hello");
        assert_eq!(hello.name(), "hello");
        let out = hello.call("greet", b"world").expect("greet");
        assert_eq!(String::from_utf8_lossy(&out), "hello, world");

        let echo = LoadedPlugin::load(example_lib("echo_plugin")).expect("echo");
        assert_eq!(echo.name(), "echo");
        let out = echo.call("echo", b"ping").expect("echo");
        assert_eq!(out, b"ping");
        let err = echo.call("nope", b"").expect_err("unsupported");
        assert!(matches!(err, HostError::Call { status: 2, .. }));
    }
}
