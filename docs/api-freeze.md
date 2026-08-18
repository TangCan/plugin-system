# 核心 API / Error 冻结对照表

对照设计文档 [`docs/requirements/6. API 设计概览.md`](requirements/6.%20API%20设计概览.md) **§6.1 / §6.2**，记录 `plugctx` 当前公开契约的冻结状态（FR35 / NFR8）。

状态含义：

| 标注 | 含义 |
|------|------|
| **已对齐** | 与设计表一致（或仅命名/借用包装等价） |
| **偏差** | 与设计伪代码或早期 mermaid 不一致，但为刻意/兼容选择，集成方以此处为准 |
| **扩展** | 超出 §6.1 核心表，由 Cargo feature 启用 |

---

## 1. Error（设计 §6.2.5）

实现：`crates/plugctx/src/error.rs`（`thiserror` 派生）。

| 设计变体 | 实现 | 状态 |
|----------|------|------|
| `MissingDependency` | `Error::MissingDependency`（单元） | **偏差**：无 `{ plugin, service: TypeId }` 载荷，保留 `Clone + PartialEq + Eq` |
| `CircularDependency` | `Error::CircularDependency`（单元） | **偏差**：无 `plugins: Vec<TypeId>` 载荷（同上）；语义含「成环」与「≥2 插件乐观构建无法进展」（各自缺不同依赖亦此变体），见 rustdoc |
| `AlreadyStarted` | `Error::AlreadyStarted` | **已对齐** |
| `AlreadyDisposed` | `Error::AlreadyDisposed` | **已对齐**（亦用于 disposed 后 `isolate`、以及 Context 已毁时的 `PluginHandle::dispose`） |
| `ServiceNotFound` | `Error::ServiceNotFound`（单元） | **偏差**：无 `{ type_id }`；**且** `get`/`get_trait` 仍返回 `Option`，本变体预留稳定名 |
| `PluginAlreadyDisposed` | `Error::PluginAlreadyDisposed` | **已对齐** |
| `BuildFailed` | `Error::BuildFailed`（单元） | **偏差**：不包裹 `Box<dyn Error + Send + Sync>`（同上） |
| （等）native/wasm/ABI | `Native*` / `Wasm*` / `AbiMismatch` / … | **扩展**（`dynamic-native` / `dynamic-wasm`） |

---

## 2. Plugin trait（§6.2.1）

| 设计 | 实现 | 状态 |
|------|------|------|
| `fn build(&self, ctx: &mut Context) -> Result<(), Error>` | 同左（`&Context` 内部为共享可变） | **已对齐**（签名为 `&Context`，与全库一致；设计表写 `&mut Context` 为伪代码习惯） |
| `fn dependencies(&self) -> Vec<TypeId>` 默认空 | 同左 | **已对齐** |
| — | `thread-safe` 时 `Plugin: Send + Sync` | **扩展** |

---

## 3. Context 方法（§6.2.2）

| 设计方法 | 实现 | 状态 |
|----------|------|------|
| `new() -> Self` | `Context::new` | **已对齐** |
| `isolate(&self) -> Result<Self, Error>` | `Context::isolate` | **偏差**：设计表曾写 `-> Self`；已 dispose 时返回 `AlreadyDisposed`（Epic 1 retro） |
| `plugin<P: Plugin + 'static>(&self, plugin: P) -> Result<PluginHandle, Error>` | 同左 | **已对齐** |
| `provide<T>(&self, service: T) -> Option<T>` | 同左（`thread-safe` 附加 `Send + Sync`） | **已对齐** |
| `provide_trait<T: ?Sized>(&self, service: Box<T>) -> Option<Box<T>>` | 同左 | **已对齐** |
| `get<T>(&self) -> Option<Ref<'_, T>>` | `Option<ServiceRef<'_, T>>` | **偏差**：返回类型别名为 `ServiceRef`（`RefCell`/`RwLock` 包装），语义同 `Option` 借用，**不**返回 `Error::ServiceNotFound` |
| `get_mut<T>(&self) -> Option<RefMut<'_, T>>` | `Option<ServiceMut<'_, T>>` | **偏差**：同上（`ServiceMut`） |
| `get_trait<T: ?Sized>(&self) -> Option<Ref<'_, T>>` | `Option<ServiceRef<'_, T>>` | **偏差**：同 `get` |
| `on<E>(&self, handler) -> EventListenerHandle` | 同左 | **已对齐** |
| `emit<E>(&self, event: &E)` | 同左（无 `Result`） | **已对齐** |
| `effect(setup) -> EffectHandle` | 同左 | **已对齐** |
| `add_interceptor(I)` | 同左 | **已对齐** |
| `start(&self) -> Result<(), Error>` | 同左 | **已对齐** |
| `dispose(&self)` | 同左（幂等） | **已对齐** |
| `is_started` / `is_disposed` | 同左 | **已对齐** |
| — | `plugin_async` / `start_async` / `emit_parallel` / `on_async` | **扩展**（`async` / `parallel`） |
| — | `contains_plugin` / `plugin_scope` | **扩展**（诊断/精确卸载辅助，非 §6.1 表项） |

---

## 4. 句柄（§6.2.3）

| 设计 | 实现 | 状态 |
|------|------|------|
| `PluginHandle::dispose(&self) -> Result<(), Error>` | 同左；Context 已毁 → `AlreadyDisposed`；条目已卸 → `PluginAlreadyDisposed` | **偏差**：设计正文仅写再次 → `PluginAlreadyDisposed`；实现区分 Context 级与插件级（Epic 2 retro） |
| `EffectHandle::cancel(self)` | 同左 | **已对齐** |
| `EventListenerHandle::cancel(self)` | 同左 | **已对齐** |

---

## 5. ContextInterceptor（§6.2.4）

| 设计 | 实现 | 状态 |
|------|------|------|
| `before/after_plugin_build` | 同左 | **已对齐** |
| `before/after_emit` | `&dyn Any`（对象安全） | **已对齐**（设计正文已写明相对早期泛型伪代码的偏离） |

---

## 6. 扩展 API（§6.3，非本表冻结核心）

异步、并行 emit、动态加载、`stages` 生命周期事件、`tracing` 等见 README Feature 矩阵与设计 §6.3 / §6.4；**不**纳入 0.1 核心最小冻结面（发布切片见 Story 6.3）。

---

## 维护约定

- 新增公开错误变体或改核心方法签名：同步更新本表与 `error.rs` rustdoc。
- 默认 `cargo test -p plugctx --test acceptance_story_6_2` 校验本文件存在且覆盖 `ServiceNotFound` / 对齐标注。
