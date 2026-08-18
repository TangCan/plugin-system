//! `dynamic-wasm-component`：`wasmtime::component` 宿主嵌入（FR47 / FR48 / FR49）。
//!
//! 启用本 feature 时可选依赖 **wasmtime**（**不**进入默认依赖图，NFR14）。
//! 与 [`crate::dynamic_wasm`]（Extism / `dynamic-wasm`）**分路径、分制品**，
//! 本模块不加载 Extism PDK `.wasm`，也**不得**假设同一 `.wasm` 二进制可两吃（FR48）。
//!
//! # 最小 API
//!
//! - [`load_wasm_component`]：从组件字节（或 WAT 文本）编译并实例化。
//! - [`ComponentPlugin`]：实现 [`crate::Plugin`]，可经 [`crate::Context::plugin`] 安装。
//! - [`ComponentInvoker`]：build 期注入的调用服务；dispose Effect 释放 Store。
//! - [`bundled_component_add_wat`]：仓库内最小组件 fixture（无 WASI / 无 WIT 源）。
//! - [`bundled_wit_sample_add_wasm`]：真实 `wasm32-wasip2` + WIT world 样例客人制品（FR50）。
//!
//! # 一 Store 一实例销毁（FR49）
//!
//! 一 [`wasmtime::Store`] 绑定一组件实例。销毁路径 = **Drop Store**（及绑定实例）：
//! [`ComponentPlugin::close`] / Context dispose Effect 将 `Option<ComponentState>` 置 `None`，
//! 触发 Store Drop。[`ComponentPlugin::store_drop_count`] 对 Store 宿主数据的 Drop 计数，
//! 供 ATDD 证明「未 Drop 仍可用 / Drop 后不可用」。
//!
//! # 版本矩阵
//!
//! 见工作区 `docs/component-model-versions.md`（NFR12）。

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use wasmtime::component::{Component, Linker, TypedFunc};
use wasmtime::{Engine, Store};

use crate::context::Context;
use crate::error::Error;
use crate::plugin::Plugin;

/// 内置最小组件 WAT：导出 `add(a: s32, b: s32) -> s32`（无 WASI / 无 WIT 世界文件）。
///
/// 与 [`bundled_wit_sample_add_wasm`] 对照：本 fixture 不依赖 wasip2 工具链即可验收骨架路径。
pub fn bundled_component_add_wat() -> &'static str {
    include_str!("../testdata/component_add.wat")
}

/// 内置 **真实** `wasm32-wasip2` + wit-bindgen 0.60 样例客人组件（FR50）。
///
/// 源码与 WIT：`plugin-system/guests/wit-sample/`。CI 使用本检入 `.wasm`，**不**要求安装
/// wasip2 工具链；重建见 `scripts/build-wit-sample-guest.sh` 与
/// `docs/component-model-versions.md`。
pub fn bundled_wit_sample_add_wasm() -> &'static [u8] {
    include_bytes!("../testdata/wit_sample_add.wasm")
}

/// Store 宿主数据：Drop 时递增共享计数，证明「一 Store 一实例」销毁可观测（FR49）。
struct StoreHostData {
    store_drop_hits: Arc<AtomicUsize>,
}

impl Drop for StoreHostData {
    fn drop(&mut self) {
        self.store_drop_hits.fetch_add(1, Ordering::AcqRel);
    }
}

struct ComponentState {
    store: Store<StoreHostData>,
    /// 导出 `add`；与 `store` 成对（一 Store 一实例）。
    add: TypedFunc<(i32, i32), (i32,)>,
}

struct ComponentShared {
    name: String,
    state: Mutex<Option<ComponentState>>,
    closed: AtomicBool,
    store_drop_hits: Arc<AtomicUsize>,
}

impl ComponentShared {
    fn call_add(&self, a: i32, b: i32) -> Result<i32, Error> {
        if self.closed.load(Ordering::Acquire) {
            return Err(Error::WasmClosed {
                name: self.name.clone(),
            });
        }
        let mut guard = self.state.lock().map_err(|_| Error::WasmCall {
            name: self.name.clone(),
            op: "add".into(),
            message: "store 锁 poisoned".into(),
        })?;
        let ComponentState { store, add } = guard.as_mut().ok_or_else(|| Error::WasmCall {
            name: self.name.clone(),
            op: "add".into(),
            message: "组件 Store 已释放".into(),
        })?;
        let (sum,) = add.call(store, (a, b)).map_err(|e| Error::WasmCall {
            name: self.name.clone(),
            op: "add".into(),
            message: e.to_string(),
        })?;
        Ok(sum)
    }

    /// 释放 Store（及绑定实例）= 销毁；幂等。Store Drop 递增 [`store_drop_count`](ComponentPlugin::store_drop_count)。
    fn close(&self) {
        if self
            .closed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            if let Ok(mut guard) = self.state.lock() {
                *guard = None;
            }
        }
    }

    fn store_drop_count(&self) -> usize {
        self.store_drop_hits.load(Ordering::Acquire)
    }
}

/// 供 Context 服务注入的 Component 调用句柄。
#[derive(Clone)]
pub struct ComponentInvoker {
    inner: Arc<ComponentShared>,
}

impl ComponentInvoker {
    /// 调用导出 `add`，返回 `a + b`。
    pub fn call_add(&self, a: i32, b: i32) -> Result<i32, Error> {
        self.inner.call_add(a, b)
    }

    pub fn name(&self) -> &str {
        &self.inner.name
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    /// Store Drop 次数（FR49 探针）。未销毁为 0；`close`/dispose 后 ≥ 1。
    pub fn store_drop_count(&self) -> usize {
        self.inner.store_drop_count()
    }
}

/// 已加载的 Component Model 插件实例（可安装进 Context）。
pub struct ComponentPlugin {
    inner: Arc<ComponentShared>,
}

impl ComponentPlugin {
    /// 调用导出 `add`，返回 `a + b`。
    pub fn call_add(&self, a: i32, b: i32) -> Result<i32, Error> {
        self.inner.call_add(a, b)
    }

    pub fn name(&self) -> &str {
        &self.inner.name
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    /// Store Drop 次数（FR49 探针）。未销毁为 0；`close`/dispose 后 ≥ 1。
    pub fn store_drop_count(&self) -> usize {
        self.inner.store_drop_count()
    }

    /// 主动显式关闭：Drop Store（通常由 Context dispose Effect 调用）。
    pub fn close(&self) {
        self.inner.close();
    }
}

impl Plugin for ComponentPlugin {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        ctx.provide(ComponentInvoker {
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

/// 从组件制品（二进制或 WAT 文本）加载并实例化，解析导出 `add`。
pub fn load_wasm_component(bytes: impl AsRef<[u8]>) -> Result<ComponentPlugin, Error> {
    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config).map_err(|e| Error::WasmLoad {
        message: format!("创建 Engine 失败: {e}"),
    })?;
    let component = Component::new(&engine, bytes.as_ref()).map_err(|e| Error::WasmLoad {
        message: format!("编译/解析组件失败: {e}"),
    })?;
    let linker = Linker::new(&engine);
    let store_drop_hits = Arc::new(AtomicUsize::new(0));
    let mut store = Store::new(
        &engine,
        StoreHostData {
            store_drop_hits: Arc::clone(&store_drop_hits),
        },
    );
    let instance = linker
        .instantiate(&mut store, &component)
        .map_err(|e| Error::WasmLoad {
            message: format!("实例化组件失败: {e}"),
        })?;
    let add = instance
        .get_typed_func::<(i32, i32), (i32,)>(&mut store, "add")
        .map_err(|e| Error::WasmLoad {
            message: format!("缺少导出 add: {e}"),
        })?;
    Ok(ComponentPlugin {
        inner: Arc::new(ComponentShared {
            name: "component".into(),
            state: Mutex::new(Some(ComponentState { store, add })),
            closed: AtomicBool::new(false),
            store_drop_hits,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_wat_loads_and_adds() {
        let plugin = load_wasm_component(bundled_component_add_wat().as_bytes()).expect("load");
        assert_eq!(plugin.call_add(2, 3).expect("add"), 5);
        assert_eq!(plugin.store_drop_count(), 0);
    }

    #[test]
    fn bundled_wit_sample_wasip2_loads_and_adds() {
        let plugin = load_wasm_component(bundled_wit_sample_add_wasm()).expect("load wit sample");
        assert_eq!(plugin.call_add(2, 3).expect("add"), 5);
        assert_eq!(plugin.call_add(-4, 10).expect("add"), 6);
        assert_eq!(plugin.store_drop_count(), 0);
    }

    #[test]
    fn repeated_calls_are_stable() {
        let plugin = load_wasm_component(bundled_component_add_wat().as_bytes()).expect("load");
        for i in 0..32 {
            assert_eq!(plugin.call_add(i, i + 1).expect("add"), i + i + 1);
        }
        assert_eq!(plugin.store_drop_count(), 0);
    }

    #[test]
    fn empty_bytes_reject() {
        let err = match load_wasm_component([]) {
            Ok(_) => panic!("expected load failure"),
            Err(e) => e,
        };
        assert!(matches!(err, Error::WasmLoad { .. }));
    }

    #[test]
    fn component_without_add_export_rejects() {
        // 合法组件，但无 `add` 导出 → 加载期失败（骨架要求解析 add）。
        const WAT: &str = r#"
(component
  (core module $m
    (func (export "nop"))
  )
  (core instance $i (instantiate $m))
  (func (export "nop")
    (canon lift (core func $i "nop")))
)
"#;
        let err = match load_wasm_component(WAT.as_bytes()) {
            Ok(_) => panic!("expected missing export"),
            Err(e) => e,
        };
        match &err {
            Error::WasmLoad { message } if message.contains("add") => {}
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn close_drops_store_and_blocks_further_calls() {
        let plugin = load_wasm_component(bundled_component_add_wat().as_bytes()).expect("load");
        assert_eq!(plugin.store_drop_count(), 0);
        assert_eq!(plugin.call_add(1, 2).expect("before drop"), 3);
        plugin.close();
        plugin.close();
        assert!(plugin.is_closed());
        assert_eq!(plugin.store_drop_count(), 1);
        assert!(matches!(
            plugin.call_add(1, 2),
            Err(Error::WasmClosed { .. })
        ));
    }

    #[test]
    fn plugin_dispose_drops_store_via_effect() {
        let plugin = load_wasm_component(bundled_component_add_wat().as_bytes()).expect("load");
        let ctx = Context::new();
        let handle = ctx.plugin(plugin).expect("install");
        ctx.start().expect("start");
        // Clone 出服务句柄后再 dispose，避免 ServiceRef 借住 Context 导致 RefCell 冲突。
        let invoker = ctx.get::<ComponentInvoker>().expect("invoker").clone();
        assert_eq!(invoker.store_drop_count(), 0);
        assert_eq!(invoker.call_add(10, 32).expect("add"), 42);
        handle.dispose().expect("dispose");
        assert!(ctx.get::<ComponentInvoker>().is_none());
        assert_eq!(invoker.store_drop_count(), 1);
        assert!(matches!(
            invoker.call_add(1, 1),
            Err(Error::WasmClosed { .. })
        ));
    }
}
