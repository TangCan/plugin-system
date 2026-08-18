//! Shared contract between plugin host and `cdylib` plugins.
//!
//! Plugins export a single entry symbol [`PLUGIN_ENTRY_SYMBOL`] that returns a
//! [`PluginVTable`]. The host never shares Rust trait objects across the DSO
//! boundary — only this C layout is stable.
//!
//! 本 crate `publish = false`（FR51）；ABI 正文与可发布的 `plugctx::c_abi` 同源。

#![deny(unsafe_op_in_unsafe_fn)]

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../plugctx/src/c_abi.rs"
));
