//! Extism PDK guest used to rebuild `plugctx/testdata/echo.wasm`.
//!
//! 本 crate **不**加入 workspace members，避免默认 `cargo test --workspace` 编译 PDK。
//!
//! ```bash
//! rustup target add wasm32-unknown-unknown
//! cd crates/plugins/wasm_echo
//! cargo build --target wasm32-unknown-unknown --release
//! cp target/wasm32-unknown-unknown/release/wasm_echo.wasm \
//!    ../../plugctx/testdata/echo.wasm
//! ```

use extism_pdk::*;
use std::sync::Mutex;

/// Guest-visible scratch state for pool reset ATDD (survives Extism soft `reset`).
static STATE: Mutex<Vec<u8>> = Mutex::new(Vec::new());

#[plugin_fn]
pub fn echo(input: Vec<u8>) -> FnResult<Vec<u8>> {
    Ok(input)
}

/// 写入可观测客人状态（Story 7.2 / FR44）。
#[plugin_fn]
pub fn set_state(input: Vec<u8>) -> FnResult<()> {
    let mut guard = STATE.lock().map_err(|_| {
        WithReturnCode::new(Error::msg("state mutex poisoned"), 1)
    })?;
    *guard = input;
    Ok(())
}

/// 读取客人状态；空表示未见过写入或实例已重建。
#[plugin_fn]
pub fn get_state(_: ()) -> FnResult<Vec<u8>> {
    let guard = STATE.lock().map_err(|_| {
        WithReturnCode::new(Error::msg("state mutex poisoned"), 1)
    })?;
    Ok(guard.clone())
}
