//! Context：插件运行环境入口与生命周期控制。
//!
//! 插件条目以 `slotmap::SlotMap` 存储，[`PluginId`] 为稳定键（设计 §7.4）。
//! 插件 `build` 期间通过 [`PluginScope`] 栈自动记录注册，供后续精确卸载（设计 §3.6）。
//!
//! 公开方法与设计 §6.1 / §6.2 对照（含 `get`→[`Option`] 等偏差）见工作区 `docs/api-freeze.md`。

use std::any::{Any, TypeId};
use std::collections::HashMap;
#[cfg(feature = "parallel")]
use std::future::Future;
#[cfg(feature = "parallel")]
use std::pin::Pin;

use slotmap::SlotMap;

#[cfg(feature = "async")]
use crate::async_plugin::AsyncPlugin;
use crate::effect::EffectHandle;
use crate::error::Error;
use crate::event::{DisposeEvent, EventListenerHandle, ReadyEvent};
#[cfg(feature = "stages")]
use crate::event::{InitEvent, PostStartEvent, PreDisposeEvent};
use crate::interceptor::ContextInterceptor;
use crate::plugin::{Plugin, PluginEntry, PluginHandle, PluginId, PluginScope, StoredPlugin};
#[cfg(feature = "parallel")]
use crate::shared::AsyncHandler;
use crate::shared::{
    new_interceptor, new_sync_handler, AnyBox, CleanupBox, Flag, InterceptorPtr, ServiceMut,
    ServiceRef, Shared, SharedWeak, SyncHandler,
};

/// 类型擦除的事件监听器槽：`emit` 时传入 `&dyn Any` 再 internally downcast。
struct ListenerSlot {
    id: usize,
    cancelled: Flag,
    handler: SyncHandler,
}

/// 异步事件监听器槽（`parallel`）：handler 接收已克隆的事件并返回 `'static` Future。
#[cfg(feature = "parallel")]
struct AsyncListenerSlot {
    id: usize,
    cancelled: Flag,
    handler: AsyncHandler,
}

/// 已登记的副作用 cleanup 槽。
struct EffectSlot {
    id: usize,
    cancelled: Flag,
    cleanup: Option<CleanupBox>,
}

/// 内部可变状态。
struct ContextData {
    started: bool,
    /// 正在执行 `dispose`（含 DisposeEvent 分发）；用于防重入且仍允许事件阶段登记 effect。
    disposing: bool,
    disposed: bool,
    /// 已安装插件（含未构建）；键为稳定 [`PluginId`]。
    plugin_entries: SlotMap<PluginId, PluginEntry>,
    /// 正在构建的插件作用域栈；栈顶为 `(PluginId, PluginScope)`。
    /// 根级（栈空）注册不计入任何插件 scope。
    plugin_scope_stack: Vec<(PluginId, PluginScope)>,
    /// 具体类型服务：`TypeId` → 类型擦除实例。
    services: HashMap<TypeId, AnyBox>,
    /// 当前具体类型服务的提供者插件；根级 `provide` 时清除对应条目（§5.3 覆盖不误删）。
    service_owners: HashMap<TypeId, PluginId>,
    /// trait 对象服务：`TypeId::of::<dyn Trait>()` → `Box<Box<dyn Trait>>`（擦除为 `Box<dyn Any>`）。
    trait_services: HashMap<TypeId, AnyBox>,
    /// 当前 trait 对象服务的提供者插件；根级 `provide_trait` 时清除对应条目。
    trait_service_owners: HashMap<TypeId, PluginId>,
    /// 同步事件监听器：`TypeId` → 有序槽位列表。
    events: HashMap<TypeId, Vec<ListenerSlot>>,
    /// 异步事件监听器（仅 `parallel`）：与同步分轨，避免污染 `emit`。
    #[cfg(feature = "parallel")]
    async_events: HashMap<TypeId, Vec<AsyncListenerSlot>>,
    next_listener_id: usize,
    /// 副作用 cleanup，销毁时逆序执行。
    effects: Vec<EffectSlot>,
    next_effect_id: usize,
    /// 子上下文（弱引用，避免与子持有的父句柄形成循环）。
    children: Vec<SharedWeak<ContextData>>,
    /// 拦截器列表（注册序）；`Rc` 便于调用前快照并释放 `ContextData` 借用。
    interceptors: Vec<InterceptorPtr>,
}

/// 插件框架上下文句柄。
///
/// 默认内部为 `Rc<RefCell<ContextData>>`；启用 `thread-safe` 后为 `Arc<RwLock<ContextData>>`
///（经内部 `Shared` 抽象）。`parent` 为父上下文强引用链；子列表用弱引用登记以便级联销毁。
///
/// # 锁与重入（`thread-safe`）
///
/// - 对外 API 在调用期间短暂持锁；`emit` / 拦截器仍先快照再回调，允许有限重入。
/// - 持有 [`ServiceRef`] / [`ServiceMut`]（`get`/`get_mut`/`get_trait`）期间，**勿**再调用会取写锁的 API
///   （如 `provide`/`emit`/`start`），否则可能死锁。
/// - 监听器 `FnMut`：默认路径重入同一槽时跳过（`try_borrow_mut`）；`thread-safe` 下对同槽
///   **阻塞串行**（跨线程并发 `emit` 不丢事件）。同线程嵌套再次进入同一监听器可能死锁，请避免。
#[derive(Clone)]
pub struct Context {
    data: Shared<ContextData>,
    parent: Option<Box<Context>>,
}

impl Context {
    /// 创建无父级的根上下文。
    pub fn new() -> Self {
        Self {
            data: Shared::new(ContextData {
                started: false,
                disposing: false,
                disposed: false,
                plugin_entries: SlotMap::with_key(),
                plugin_scope_stack: Vec::new(),
                services: HashMap::new(),
                service_owners: HashMap::new(),
                trait_services: HashMap::new(),
                trait_service_owners: HashMap::new(),
                events: HashMap::new(),
                #[cfg(feature = "parallel")]
                async_events: HashMap::new(),
                next_listener_id: 0,
                effects: Vec::new(),
                next_effect_id: 0,
                children: Vec::new(),
                interceptors: Vec::new(),
            }),
            parent: None,
        }
    }

    /// 创建子上下文：可读取父级服务；本级 `provide` 不污染父级；父 `dispose` 时级联销毁。
    ///
    /// 子上下文**不**继承父级拦截器；若需相同钩子请在子级自行 `add_interceptor`。
    ///
    /// 若当前正处于某插件 `build`（作用域栈非空），则更新该插件的 `children_start`/`children_count`。
    ///
    /// # 错误
    ///
    /// - [`Error::AlreadyDisposed`]：父上下文已完全销毁（`is_disposed()`）。`DisposeEvent` /
    ///   effect cleanup 窗口内（`disposing`、尚未 `disposed`）仍可 `isolate`。
    pub fn isolate(&self) -> Result<Context, Error> {
        {
            let data = self.data.borrow();
            if data.disposed {
                return Err(Error::AlreadyDisposed);
            }
        }
        let child = Context {
            data: Shared::new(ContextData {
                started: false,
                disposing: false,
                disposed: false,
                plugin_entries: SlotMap::with_key(),
                plugin_scope_stack: Vec::new(),
                services: HashMap::new(),
                service_owners: HashMap::new(),
                trait_services: HashMap::new(),
                trait_service_owners: HashMap::new(),
                events: HashMap::new(),
                #[cfg(feature = "parallel")]
                async_events: HashMap::new(),
                next_listener_id: 0,
                effects: Vec::new(),
                next_effect_id: 0,
                children: Vec::new(),
                interceptors: Vec::new(),
            }),
            parent: Some(Box::new(self.clone())),
        };
        let mut data = self.data.borrow_mut();
        let children_len = data.children.len();
        data.children.push(child.data.downgrade());
        if let Some((_, scope)) = data.plugin_scope_stack.last_mut() {
            if scope.children_count == 0 {
                scope.children_start = children_len;
            }
            scope.children_count += 1;
        }
        Ok(child)
    }

    /// 提供类型 `T` 的服务；若已存在同类型则替换并返回旧值（仅写本级）。
    ///
    /// 插件 `build` 期间会将 `TypeId::of::<T>()` 记入当前 [`PluginScope`]，并更新
    /// `service_owners`；根级调用不记账，并清除该类型的所有者（§5.3）。
    ///
    /// 销毁窗口（`DisposeEvent` / cleanup / 完全 disposed）内的允许性见工作区
    /// `docs/dispose-registration-window.md`。
    #[cfg(not(feature = "thread-safe"))]
    pub fn provide<T: 'static>(&self, service: T) -> Option<T> {
        let mut data = self.data.borrow_mut();
        let tid = TypeId::of::<T>();
        let owner = data.plugin_scope_stack.last().map(|(id, _)| *id);
        if let Some((_, scope)) = data.plugin_scope_stack.last_mut() {
            scope.provided_services.push(tid);
        }
        match owner {
            Some(id) => {
                data.service_owners.insert(tid, id);
            }
            None => {
                data.service_owners.remove(&tid);
            }
        }
        data.services
            .insert(tid, crate::shared::erase_any(service))
            .and_then(|old| old.downcast::<T>().ok().map(|boxed| *boxed))
    }

    /// 同 [`Self::provide`]；`thread-safe` 下要求 `Send + Sync`。
    #[cfg(feature = "thread-safe")]
    pub fn provide<T: Send + Sync + 'static>(&self, service: T) -> Option<T> {
        let mut data = self.data.borrow_mut();
        let tid = TypeId::of::<T>();
        let owner = data.plugin_scope_stack.last().map(|(id, _)| *id);
        if let Some((_, scope)) = data.plugin_scope_stack.last_mut() {
            scope.provided_services.push(tid);
        }
        match owner {
            Some(id) => {
                data.service_owners.insert(tid, id);
            }
            None => {
                data.service_owners.remove(&tid);
            }
        }
        data.services
            .insert(tid, crate::shared::erase_any(service))
            .and_then(|old| old.downcast::<T>().ok().map(|boxed| *boxed))
    }

    /// 获取类型 `T` 的不可变引用；本级未命中时沿父链查找。
    pub fn get<T: 'static>(&self) -> Option<ServiceRef<'_, T>> {
        if self.data.borrow().services.contains_key(&TypeId::of::<T>()) {
            return self.data.try_map_read(|data| {
                data.services
                    .get(&TypeId::of::<T>())
                    .and_then(|s| s.downcast_ref::<T>())
            });
        }
        self.parent.as_ref().and_then(|p| p.get::<T>())
    }

    /// 获取类型 `T` 的可变引用；本级未命中时沿父链查找（可变父级同一实例）。
    pub fn get_mut<T: 'static>(&self) -> Option<ServiceMut<'_, T>> {
        if self.data.borrow().services.contains_key(&TypeId::of::<T>()) {
            return self.data.try_map_write(|data| {
                data.services
                    .get_mut(&TypeId::of::<T>())
                    .and_then(|s| s.downcast_mut::<T>())
            });
        }
        self.parent.as_ref().and_then(|p| p.get_mut::<T>())
    }

    /// 提供 trait 对象服务；键为 `TypeId::of::<T>()`（通常为 `dyn Trait`）。
    ///
    /// 若本级已存在同 trait 服务则替换并返回旧 `Box`。仅写本级，不污染父级。
    /// 插件 `build` 期间记入 [`PluginScope::provided_trait_services`] 并更新
    /// `trait_service_owners`；根级不记账并清除所有者（§5.3）。
    #[cfg(not(feature = "thread-safe"))]
    pub fn provide_trait<T: ?Sized + 'static>(&self, service: Box<T>) -> Option<Box<T>> {
        let mut data = self.data.borrow_mut();
        let tid = TypeId::of::<T>();
        let owner = data.plugin_scope_stack.last().map(|(id, _)| *id);
        if let Some((_, scope)) = data.plugin_scope_stack.last_mut() {
            scope.provided_trait_services.push(tid);
        }
        match owner {
            Some(id) => {
                data.trait_service_owners.insert(tid, id);
            }
            None => {
                data.trait_service_owners.remove(&tid);
            }
        }
        data.trait_services
            .insert(tid, crate::shared::erase_any(service))
            .and_then(|old| old.downcast::<Box<T>>().ok().map(|boxed| *boxed))
    }

    /// 同 [`Self::provide_trait`]；`thread-safe` 下要求 `Send + Sync`。
    #[cfg(feature = "thread-safe")]
    pub fn provide_trait<T: ?Sized + Send + Sync + 'static>(
        &self,
        service: Box<T>,
    ) -> Option<Box<T>> {
        let mut data = self.data.borrow_mut();
        let tid = TypeId::of::<T>();
        let owner = data.plugin_scope_stack.last().map(|(id, _)| *id);
        if let Some((_, scope)) = data.plugin_scope_stack.last_mut() {
            scope.provided_trait_services.push(tid);
        }
        match owner {
            Some(id) => {
                data.trait_service_owners.insert(tid, id);
            }
            None => {
                data.trait_service_owners.remove(&tid);
            }
        }
        data.trait_services
            .insert(tid, crate::shared::erase_any(service))
            .and_then(|old| old.downcast::<Box<T>>().ok().map(|boxed| *boxed))
    }

    /// 获取 trait 对象服务的不可变引用；本级未命中时沿父链查找。
    pub fn get_trait<T: ?Sized + 'static>(&self) -> Option<ServiceRef<'_, T>> {
        if self
            .data
            .borrow()
            .trait_services
            .contains_key(&TypeId::of::<T>())
        {
            return self.data.try_map_read(|data| {
                data.trait_services
                    .get(&TypeId::of::<T>())
                    .and_then(|s| s.downcast_ref::<Box<T>>())
                    .map(|boxed| &**boxed as &T)
            });
        }
        self.parent.as_ref().and_then(|p| p.get_trait::<T>())
    }

    /// 为事件类型 `E` 注册监听器，按注册顺序在 `emit` 时同步调用；返回取消句柄。
    ///
    /// 插件 `build` 期间会将 `(TypeId, 列表下标)` 记入当前 [`PluginScope`]；根级调用不记账。
    #[cfg(not(feature = "thread-safe"))]
    pub fn on<E: 'static>(&self, mut handler: impl FnMut(&E) + 'static) -> EventListenerHandle {
        let erased = new_sync_handler(move |ev: &dyn Any| {
            let event = ev
                .downcast_ref::<E>()
                .expect("emit TypeId matches registered listener");
            handler(event);
        });
        self.register_listener(TypeId::of::<E>(), erased)
    }

    #[cfg(feature = "thread-safe")]
    pub fn on<E: Send + Sync + 'static>(
        &self,
        mut handler: impl FnMut(&E) + Send + 'static,
    ) -> EventListenerHandle {
        let erased = new_sync_handler(move |ev: &dyn Any| {
            let event = ev
                .downcast_ref::<E>()
                .expect("emit TypeId matches registered listener");
            handler(event);
        });
        self.register_listener(TypeId::of::<E>(), erased)
    }

    fn register_listener(&self, type_id: TypeId, erased: SyncHandler) -> EventListenerHandle {
        let cancelled = Flag::new(false);
        let listener_id = {
            let mut data = self.data.borrow_mut();
            let id = data.next_listener_id;
            data.next_listener_id += 1;
            let slots = data.events.entry(type_id).or_default();
            let index = slots.len();
            slots.push(ListenerSlot {
                id,
                cancelled: cancelled.clone(),
                handler: erased,
            });
            if let Some((_, scope)) = data.plugin_scope_stack.last_mut() {
                scope.registered_events.push((type_id, index));
            }
            id
        };

        EventListenerHandle {
            ctx: self.clone(),
            type_id,
            listener_id,
            cancelled,
        }
    }

    /// 同步触发事件 `E` 的全部未取消监听器（注册顺序）。无监听器时为 no-op（仍会跑拦截器）。
    ///
    /// 触发前克隆监听器 `Rc` 列表并释放对 `ContextData` 的借用，因此回调内可再次
    /// `on` / `emit` / `provide` / `plugin` 等而不因 `ContextData` 的 `RefCell` 冲突 panic。
    ///
    /// 若嵌套 `emit` 再次命中**正在执行**的同一监听器（`FnMut` 仍被借用），该次调用会被跳过，
    /// 以避免监听器 `RefCell` panic；其它空闲监听器仍正常执行。
    ///
    /// 已注册的 [`ContextInterceptor`] 按注册序在监听器前后调用 `before_emit` / `after_emit`。
    ///
    /// 启用 `tracing` feature 时发出 `plugctx.emit` span（FR37）。
    pub fn emit<E: 'static>(&self, event: &E) {
        #[cfg(feature = "tracing")]
        let _span = tracing::info_span!(
            "plugctx.emit",
            event = %std::any::type_name::<E>()
        )
        .entered();

        let interceptors = self.snapshot_interceptors();
        let event_any: &dyn Any = event;
        for interceptor in &interceptors {
            interceptor.before_emit(event_any);
        }

        let snapshot: Vec<(Flag, SyncHandler)> = {
            let data = self.data.borrow();
            match data.events.get(&TypeId::of::<E>()) {
                Some(slots) => slots
                    .iter()
                    .map(|s| (s.cancelled.clone(), s.handler.clone()))
                    .collect(),
                None => Vec::new(),
            }
        };

        for (cancelled, handler) in snapshot {
            if cancelled.get() {
                continue;
            }
            // 默认：重入同一槽跳过。thread-safe：阻塞串行。
            #[cfg(not(feature = "thread-safe"))]
            if let Ok(mut handler) = handler.try_borrow_mut() {
                handler(event_any);
            }
            #[cfg(feature = "thread-safe")]
            {
                let mut handler = handler.lock();
                handler(event_any);
            }
        }

        for interceptor in &interceptors {
            interceptor.after_emit(event_any);
        }
    }

    /// 注册异步事件监听器（`parallel` feature）。事件在 `emit_parallel` 时按值克隆进各 Future。
    ///
    /// 与同步 [`Self::on`] 分轨：`emit` 不会调用本方法注册的监听器。
    ///
    /// 插件 `build` 期间会将 `(TypeId, 异步列表下标)` 记入当前 [`PluginScope`]。
    #[cfg(all(feature = "parallel", not(feature = "thread-safe")))]
    pub fn on_async<E, F, Fut>(&self, mut handler: F) -> EventListenerHandle
    where
        E: Clone + 'static,
        F: FnMut(E) -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        let type_id = TypeId::of::<E>();
        let cancelled = Flag::new(false);
        let erased: AsyncHandler =
            std::rc::Rc::new(std::cell::RefCell::new(move |ev: &dyn Any| {
                let event = ev
                    .downcast_ref::<E>()
                    .expect("emit_parallel TypeId matches registered async listener")
                    .clone();
                Box::pin(handler(event)) as Pin<Box<dyn Future<Output = ()> + 'static>>
            }));
        self.register_async_listener(type_id, cancelled, erased)
    }

    /// 同 [`Self::on_async`]；`thread-safe` 下要求 `Send` Future / 闭包。
    #[cfg(all(feature = "parallel", feature = "thread-safe"))]
    pub fn on_async<E, F, Fut>(&self, mut handler: F) -> EventListenerHandle
    where
        E: Clone + Send + Sync + 'static,
        F: FnMut(E) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let type_id = TypeId::of::<E>();
        let cancelled = Flag::new(false);
        let erased: AsyncHandler =
            std::sync::Arc::new(parking_lot::Mutex::new(move |ev: &dyn Any| {
                let event = ev
                    .downcast_ref::<E>()
                    .expect("emit_parallel TypeId matches registered async listener")
                    .clone();
                Box::pin(handler(event)) as Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            }));
        self.register_async_listener(type_id, cancelled, erased)
    }

    #[cfg(feature = "parallel")]
    fn register_async_listener(
        &self,
        type_id: TypeId,
        cancelled: Flag,
        erased: AsyncHandler,
    ) -> EventListenerHandle {
        let listener_id = {
            let mut data = self.data.borrow_mut();
            let id = data.next_listener_id;
            data.next_listener_id += 1;
            let slots = data.async_events.entry(type_id).or_default();
            let index = slots.len();
            slots.push(AsyncListenerSlot {
                id,
                cancelled: cancelled.clone(),
                handler: erased,
            });
            if let Some((_, scope)) = data.plugin_scope_stack.last_mut() {
                scope.registered_async_events.push((type_id, index));
            }
            id
        };

        EventListenerHandle {
            ctx: self.clone(),
            type_id,
            listener_id,
            cancelled,
        }
    }

    /// 宿主侧并发触发事件 `E` 的全部未取消**异步**监听器（`futures::join_all`）。
    ///
    /// 并行发生在宿主调度的 Future fan-out，不要求插件内多线程；框架不绑定具体运行时。
    /// 同步 [`Self::emit`] 语义不变。无异步监听器时为 `Ok(())`（仍会跑拦截器）。
    #[cfg(feature = "parallel")]
    pub async fn emit_parallel<E: Clone + 'static>(&self, event: &E) -> Result<(), Error> {
        let interceptors = self.snapshot_interceptors();
        let event_any: &dyn Any = event;
        for interceptor in &interceptors {
            interceptor.before_emit(event_any);
        }

        let snapshot: Vec<(Flag, AsyncHandler)> = {
            let data = self.data.borrow();
            match data.async_events.get(&TypeId::of::<E>()) {
                Some(slots) => slots
                    .iter()
                    .map(|s| (s.cancelled.clone(), s.handler.clone()))
                    .collect(),
                None => Vec::new(),
            }
        };

        let mut futs = Vec::with_capacity(snapshot.len());
        for (cancelled, handler) in snapshot {
            if cancelled.get() {
                continue;
            }
            #[cfg(not(feature = "thread-safe"))]
            let fut = match handler.try_borrow_mut() {
                Ok(mut handler) => handler(event_any),
                Err(_) => continue,
            };
            #[cfg(feature = "thread-safe")]
            let fut = {
                let mut handler = handler.lock();
                handler(event_any)
            };
            futs.push(async move {
                if cancelled.get() {
                    return;
                }
                fut.await;
            });
        }

        futures::future::join_all(futs).await;

        for interceptor in &interceptors {
            interceptor.after_emit(event_any);
        }
        Ok(())
    }

    /// 注册上下文拦截器；按注册顺序在插件 `build` 与 `emit` 前后同步调用（FR16）。
    ///
    /// 子上下文不继承本列表。钩子内可有限重入 Context API；调用前会快照列表。
    #[cfg(not(feature = "thread-safe"))]
    pub fn add_interceptor<I: ContextInterceptor + 'static>(&self, interceptor: I) {
        self.data
            .borrow_mut()
            .interceptors
            .push(new_interceptor(interceptor));
    }

    #[cfg(feature = "thread-safe")]
    pub fn add_interceptor<I: ContextInterceptor + Send + Sync + 'static>(&self, interceptor: I) {
        self.data
            .borrow_mut()
            .interceptors
            .push(new_interceptor(interceptor));
    }

    /// 安装插件。未启动时延迟构建；已启动时立即构建。
    ///
    /// 返回的 [`PluginHandle`] 持有 [`PluginId`] 稳定键，可在部分卸载后继续定位其余插件。
    pub fn plugin<P: Plugin + 'static>(&self, plugin: P) -> Result<PluginHandle, Error> {
        self.install_stored(StoredPlugin::Sync(Box::new(plugin)))
    }

    /// 安装异步插件（`async` feature）。未启动时延迟到 [`Self::start_async`]；
    /// 已启动时立即走同步 [`Plugin::build`]（与 [`Self::plugin`] 对称，因同步 API 无法 await）。
    #[cfg(feature = "async")]
    pub fn plugin_async<P: AsyncPlugin + 'static>(&self, plugin: P) -> Result<PluginHandle, Error> {
        self.install_stored(StoredPlugin::Async(Box::new(plugin)))
    }

    fn install_stored(&self, stored: StoredPlugin) -> Result<PluginHandle, Error> {
        let id = {
            let mut data = self.data.borrow_mut();
            if data.disposed {
                return Err(Error::AlreadyDisposed);
            }
            data.plugin_entries.insert(PluginEntry {
                plugin: Some(stored),
                built: false,
                scope: None,
            })
        };

        if self.is_started() {
            if !self.dependencies_satisfied(id) {
                self.data.borrow_mut().plugin_entries.remove(id);
                return Err(Error::MissingDependency);
            }
            if let Err(err) = self.build_plugin_at(id) {
                self.data.borrow_mut().plugin_entries.remove(id);
                return Err(err);
            }
        }

        Ok(PluginHandle {
            ctx: self.clone(),
            plugin_id: id,
        })
    }

    /// 指定稳定键对应的插件条目是否仍存在。
    pub fn contains_plugin(&self, id: PluginId) -> bool {
        self.data.borrow().plugin_entries.contains_key(id)
    }

    /// 已成功构建的插件作用域快照；未构建、构建失败或条目已移除则为 `None`。
    pub fn plugin_scope(&self, id: PluginId) -> Option<PluginScope> {
        self.data
            .borrow()
            .plugin_entries
            .get(id)
            .and_then(|e| e.scope.clone())
    }

    /// 按 [`PluginScope`] 精确卸载插件（设计 §3.6.3 / §5.3 / FR15 / FR33）。
    ///
    /// 回滚顺序：先移除条目（防 cleanup 重入二次 dispose），再服务 → 事件 → effects → 子上下文。
    /// 服务仅在当前值仍由本插件提供时移除（被后续插件或根级覆盖则跳过）。
    ///
    /// # 错误
    ///
    /// - [`Error::AlreadyDisposed`]：所属 [`Context`] 已完全销毁（优先于插件级错误）。
    /// - [`Error::PluginAlreadyDisposed`]：上下文仍存活，但该插件条目已不存在（二次 dispose）。
    pub(crate) fn dispose_plugin(&self, id: PluginId) -> Result<(), Error> {
        if self.data.borrow().disposed {
            return Err(Error::AlreadyDisposed);
        }

        // 先移除条目，避免 effect/cleanup 重入再次 dispose 同一插件时重复回滚。
        let scope = {
            let mut data = self.data.borrow_mut();
            let entry = data
                .plugin_entries
                .remove(id)
                .ok_or(Error::PluginAlreadyDisposed)?;
            entry.scope
        };

        if let Some(scope) = scope {
            // 1) 具体类型服务 / trait 对象服务：仅当当前所有者仍是本插件时移除
            {
                let mut data = self.data.borrow_mut();
                for tid in scope.provided_services.iter().rev() {
                    if data.service_owners.get(tid) == Some(&id) {
                        data.services.remove(tid);
                        data.service_owners.remove(tid);
                    }
                }
                for tid in scope.provided_trait_services.iter().rev() {
                    if data.trait_service_owners.get(tid) == Some(&id) {
                        data.trait_services.remove(tid);
                        data.trait_service_owners.remove(tid);
                    }
                }
            }

            // 2) 事件：按类型分组后索引从大到小删除，再修正其他插件记录的下标
            {
                let mut by_type: HashMap<TypeId, Vec<usize>> = HashMap::new();
                for (tid, idx) in &scope.registered_events {
                    by_type.entry(*tid).or_default().push(*idx);
                }
                {
                    let mut data = self.data.borrow_mut();
                    for (tid, mut idxs) in by_type {
                        idxs.sort_unstable_by(|a, b| b.cmp(a));
                        if let Some(slots) = data.events.get_mut(&tid) {
                            for index in idxs {
                                if index < slots.len() {
                                    slots.remove(index);
                                }
                            }
                        }
                    }
                }
                self.fixup_event_indices_after_plugin_unload(&scope);
            }

            // 2b) 异步事件（parallel）：同索引删除规则
            #[cfg(feature = "parallel")]
            {
                let mut by_type: HashMap<TypeId, Vec<usize>> = HashMap::new();
                for (tid, idx) in &scope.registered_async_events {
                    by_type.entry(*tid).or_default().push(*idx);
                }
                {
                    let mut data = self.data.borrow_mut();
                    for (tid, mut idxs) in by_type {
                        idxs.sort_unstable_by(|a, b| b.cmp(a));
                        if let Some(slots) = data.async_events.get_mut(&tid) {
                            for index in idxs {
                                if index < slots.len() {
                                    slots.remove(index);
                                }
                            }
                        }
                    }
                }
                self.fixup_async_event_indices_after_plugin_unload(&scope);
            }

            // 3) effects：drain 连续区间，逆序执行，并修正后续插件的 effects_start
            let effect_slots = {
                let mut data = self.data.borrow_mut();
                let start = scope.effects_start;
                let count = scope.effects_count;
                let end = start.saturating_add(count).min(data.effects.len());
                let start = start.min(end);
                let drained: Vec<EffectSlot> = data.effects.drain(start..end).collect();
                let removed = drained.len();
                if removed > 0 {
                    for entry in data.plugin_entries.values_mut() {
                        if let Some(s) = entry.scope.as_mut() {
                            if s.effects_start >= end {
                                s.effects_start -= removed;
                            }
                        }
                    }
                }
                drained
            };
            for mut slot in effect_slots.into_iter().rev() {
                if slot.cancelled.get() {
                    continue;
                }
                if let Some(cleanup) = slot.cleanup.take() {
                    cleanup();
                }
            }

            // 4) 子上下文：先按本插件区间 drain（含死弱引用），再 dispose 存活子树，
            // 并以实际槽位数下调后续插件的 children_start（避免全表 retain 失真）。
            let child_rcs = {
                let mut data = self.data.borrow_mut();
                let start = scope.children_start.min(data.children.len());
                let end = start
                    .saturating_add(scope.children_count)
                    .min(data.children.len());
                let drained: Vec<_> = data.children.drain(start..end).collect();
                let removed_slots = drained.len();
                if removed_slots > 0 {
                    for entry in data.plugin_entries.values_mut() {
                        if let Some(s) = entry.scope.as_mut() {
                            if s.children_start >= end {
                                s.children_start = s.children_start.saturating_sub(removed_slots);
                            }
                        }
                    }
                }
                drained
                    .into_iter()
                    .filter_map(|w| w.upgrade())
                    .collect::<Vec<_>>()
            };
            for child_data in child_rcs {
                Context {
                    data: child_data,
                    parent: Some(Box::new(self.clone())),
                }
                .dispose();
            }
        }

        Ok(())
    }

    /// 本插件按索引删除监听器后，下调其他插件 scope 中同事件类型、更大下标的记录。
    fn fixup_event_indices_after_plugin_unload(&self, unloaded: &PluginScope) {
        let mut by_type: HashMap<TypeId, Vec<usize>> = HashMap::new();
        for (tid, idx) in &unloaded.registered_events {
            by_type.entry(*tid).or_default().push(*idx);
        }
        for idxs in by_type.values_mut() {
            idxs.sort_unstable();
        }

        let mut data = self.data.borrow_mut();
        for entry in data.plugin_entries.values_mut() {
            let Some(scope) = entry.scope.as_mut() else {
                continue;
            };
            for (tid, idx) in scope.registered_events.iter_mut() {
                if let Some(removed_idxs) = by_type.get(tid) {
                    let less = removed_idxs.iter().filter(|r| **r < *idx).count();
                    *idx = idx.saturating_sub(less);
                }
            }
        }
    }

    /// 同 [`Self::fixup_event_indices_after_plugin_unload`]，作用于异步事件登记。
    #[cfg(feature = "parallel")]
    fn fixup_async_event_indices_after_plugin_unload(&self, unloaded: &PluginScope) {
        let mut by_type: HashMap<TypeId, Vec<usize>> = HashMap::new();
        for (tid, idx) in &unloaded.registered_async_events {
            by_type.entry(*tid).or_default().push(*idx);
        }
        for idxs in by_type.values_mut() {
            idxs.sort_unstable();
        }

        let mut data = self.data.borrow_mut();
        for entry in data.plugin_entries.values_mut() {
            let Some(scope) = entry.scope.as_mut() else {
                continue;
            };
            for (tid, idx) in scope.registered_async_events.iter_mut() {
                if let Some(removed_idxs) = by_type.get(tid) {
                    let less = removed_idxs.iter().filter(|r| **r < *idx).count();
                    *idx = idx.saturating_sub(less);
                }
            }
        }
    }

    /// 启动上下文：按依赖乐观排序构建全部延迟插件；成功后发出内核保证的 [`ReadyEvent`]（恰好一次）。
    ///
    /// # 错误
    ///
    /// - [`Error::AlreadyStarted`]：已成功 `start` 后再次调用。
    /// - [`Error::AlreadyDisposed`]：上下文已销毁。
    /// - [`Error::MissingDependency`] / [`Error::CircularDependency`] / [`Error::BuildFailed`]：构建期失败；
    ///   此时**不**触发 `ReadyEvent`，且不进入 Started。
    ///
    /// 启用 feature `stages` 时额外触发：构建前 `InitEvent`，Ready 之后 `PostStartEvent`（设计 §4.7）。
    ///
    /// 若存在经 `plugin_async`（`async` feature）安装的条目，本方法对其调用同步 [`Plugin::build`]
    ///（无法 await）；异步初始化请使用 `start_async`。
    ///
    /// 启用 `tracing` feature 时发出 `plugctx.start` span（FR37 / 设计 §7.6）。
    pub fn start(&self) -> Result<(), Error> {
        #[cfg(feature = "tracing")]
        let _span = tracing::info_span!("plugctx.start").entered();

        self.begin_start()?;
        #[cfg(feature = "stages")]
        self.emit(&InitEvent);
        self.build_pending_plugins()?;
        self.finish_start();
        Ok(())
    }

    /// 异步启动：按与 [`Self::start`] 相同的依赖序构建；异步条目走 `build_async`，同步条目走 `build`。
    ///
    /// 框架不绑定具体运行时；由调用方 `.await`（tokio / async-std 等）。失败时不标记 Started、不发 ReadyEvent。
    /// 启用 `stages` 时与同步 `start` 相同顺序触发 Init / PostStart。
    #[cfg(feature = "async")]
    pub async fn start_async(&self) -> Result<(), Error> {
        self.begin_start()?;
        #[cfg(feature = "stages")]
        self.emit(&InitEvent);
        self.build_pending_plugins_async().await?;
        self.finish_start();
        Ok(())
    }

    fn begin_start(&self) -> Result<(), Error> {
        let data = self.data.borrow();
        if data.disposed {
            return Err(Error::AlreadyDisposed);
        }
        if data.started {
            return Err(Error::AlreadyStarted);
        }
        Ok(())
    }

    fn finish_start(&self) {
        self.data.borrow_mut().started = true;
        self.emit(&ReadyEvent);
        #[cfg(feature = "stages")]
        self.emit(&PostStartEvent);
    }

    /// 立即执行 `setup`，将其返回的 cleanup 登记；`dispose` 时逆序执行未取消的 cleanup。
    ///
    /// 若上下文已销毁：仍立即执行 `setup`，但**不**登记 cleanup（已无销毁周期可挂接）。
    /// 插件 `build` 期间会更新当前 [`PluginScope`] 的 `effects_start`/`effects_count`；根级不记账。
    #[cfg(not(feature = "thread-safe"))]
    pub fn effect<F, G>(&self, setup: F) -> EffectHandle
    where
        F: FnOnce() -> G + 'static,
        G: FnOnce() + 'static,
    {
        let cleanup = setup();
        let cancelled = Flag::new(false);
        let effect_id = {
            let mut data = self.data.borrow_mut();
            if data.disposed {
                return EffectHandle {
                    ctx: self.clone(),
                    effect_id: usize::MAX,
                    cancelled,
                };
            }
            let id = data.next_effect_id;
            data.next_effect_id += 1;
            let effects_len = data.effects.len();
            if let Some((_, scope)) = data.plugin_scope_stack.last_mut() {
                if scope.effects_count == 0 {
                    scope.effects_start = effects_len;
                }
                scope.effects_count += 1;
            }
            data.effects.push(EffectSlot {
                id,
                cancelled: cancelled.clone(),
                cleanup: Some(crate::shared::erase_cleanup(cleanup)),
            });
            id
        };
        EffectHandle {
            ctx: self.clone(),
            effect_id,
            cancelled,
        }
    }

    /// 同 [`Self::effect`]；`thread-safe` 下要求 cleanup 闭包 `Send + Sync`。
    #[cfg(feature = "thread-safe")]
    pub fn effect<F, G>(&self, setup: F) -> EffectHandle
    where
        F: FnOnce() -> G + Send + Sync + 'static,
        G: FnOnce() + Send + Sync + 'static,
    {
        let cleanup = setup();
        let cancelled = Flag::new(false);
        let effect_id = {
            let mut data = self.data.borrow_mut();
            if data.disposed {
                return EffectHandle {
                    ctx: self.clone(),
                    effect_id: usize::MAX,
                    cancelled,
                };
            }
            let id = data.next_effect_id;
            data.next_effect_id += 1;
            let effects_len = data.effects.len();
            if let Some((_, scope)) = data.plugin_scope_stack.last_mut() {
                if scope.effects_count == 0 {
                    scope.effects_start = effects_len;
                }
                scope.effects_count += 1;
            }
            data.effects.push(EffectSlot {
                id,
                cancelled: cancelled.clone(),
                cleanup: Some(crate::shared::erase_cleanup(cleanup)),
            });
            id
        };
        EffectHandle {
            ctx: self.clone(),
            effect_id,
            cancelled,
        }
    }

    /// 销毁上下文：先触发内核保证的 [`DisposeEvent`]，再逆序执行 effects，再级联子上下文；幂等。
    ///
    /// 顺序（设计约定）：
    /// 1. 进入 `disposing`（防重入二次销毁；此时 `is_disposed()` 仍为 false）；
    /// 2. 启用 `stages` 时先 `emit(PreDisposeEvent)`，再 `emit(DisposeEvent)`（events 仍在；监听器列表由 `emit` 克隆后调用）；
    /// 3. `take` 旧 effects / 注册表，标记 `disposed`，再逆序跑 cleanup（cleanup 内 `provide`/`on` 可保留）；
    /// 4. 级联销毁子上下文。
    ///
    /// 重复调用安全（无 panic）。
    ///
    /// 与 [`Self::start`] 配对：典型用法为 `new` → `plugin`… → `start` → 业务 → `dispose`。
    ///
    /// 启用 `tracing` feature 时发出 `plugctx.dispose` span（FR37 / 设计 §7.6）。
    pub fn dispose(&self) {
        #[cfg(feature = "tracing")]
        let _span = tracing::info_span!("plugctx.dispose").entered();

        {
            let mut data = self.data.borrow_mut();
            if data.disposed || data.disposing {
                return;
            }
            // disposing 期间仍可登记 effect（尚未 disposed）；嵌套 dispose 直接返回。
            data.disposing = true;
        }

        // 须在 take(events) 之前触发，否则监听器已被清空。
        #[cfg(feature = "stages")]
        self.emit(&PreDisposeEvent);
        self.emit(&DisposeEvent);

        let (cleanups, children) = {
            let mut data = self.data.borrow_mut();
            let slots = std::mem::take(&mut data.effects);
            let cleanups: Vec<(Flag, CleanupBox)> = slots
                .into_iter()
                .rev()
                .filter_map(|mut s| s.cleanup.take().map(|c| (s.cancelled, c)))
                .collect();
            let child_rcs: Vec<Shared<ContextData>> = std::mem::take(&mut data.children)
                .into_iter()
                .filter_map(|w| w.upgrade())
                .collect();
            // 丢弃销毁前的注册表；cleanup 内新写入可保留。
            let _ = std::mem::take(&mut data.services);
            let _ = std::mem::take(&mut data.service_owners);
            let _ = std::mem::take(&mut data.trait_services);
            let _ = std::mem::take(&mut data.trait_service_owners);
            let _ = std::mem::take(&mut data.events);
            #[cfg(feature = "parallel")]
            {
                let _ = std::mem::take(&mut data.async_events);
            }
            let _ = std::mem::take(&mut data.plugin_entries);
            let _ = std::mem::take(&mut data.plugin_scope_stack);
            let _ = std::mem::take(&mut data.interceptors);
            data.disposed = true;
            data.disposing = false;
            (cleanups, child_rcs)
        };

        // 从父的 children 列表移除自身（若父仍在），避免悬挂弱引用。
        if let Some(parent) = self.parent.as_ref() {
            let self_weak = self.data.downgrade();
            parent
                .data
                .borrow_mut()
                .children
                .retain(|w| !w.ptr_eq(&self_weak));
        }

        for (cancelled, cleanup) in cleanups {
            // 允许先运行的 cleanup 取消后续槽；调用前再读标志。
            if cancelled.get() {
                continue;
            }
            cleanup();
        }

        for child_data in children {
            Context {
                data: child_data,
                parent: None,
            }
            .dispose();
        }
    }

    /// 是否已成功 `start`。
    pub fn is_started(&self) -> bool {
        self.data.borrow().started
    }

    /// 是否已 `dispose`。
    pub fn is_disposed(&self) -> bool {
        self.data.borrow().disposed
    }

    /// 标记并移除指定监听器（由 [`EventListenerHandle::cancel`] 调用）。
    pub(crate) fn cancel_event_listener(&self, type_id: TypeId, listener_id: usize) {
        let mut data = self.data.borrow_mut();
        if let Some(slots) = data.events.get_mut(&type_id) {
            slots.retain(|s| s.id != listener_id);
        }
        #[cfg(feature = "parallel")]
        if let Some(slots) = data.async_events.get_mut(&type_id) {
            slots.retain(|s| s.id != listener_id);
        }
    }

    /// 移除指定 effect 槽且不执行 cleanup（由 [`EffectHandle::cancel`] 调用）。
    pub(crate) fn cancel_effect(&self, effect_id: usize) {
        let mut data = self.data.borrow_mut();
        data.effects.retain(|s| s.id != effect_id);
    }

    /// 本级或父链是否已提供该 `TypeId` 服务。
    fn has_service(&self, type_id: TypeId) -> bool {
        if self.data.borrow().services.contains_key(&type_id) {
            return true;
        }
        self.parent.as_ref().is_some_and(|p| p.has_service(type_id))
    }

    /// 乐观构建：每轮按安装顺序选择第一个依赖已满足的插件构建；
    /// 一轮无进展则按约定返回 `MissingDependency` 或 `CircularDependency`。
    fn build_pending_plugins(&self) -> Result<(), Error> {
        loop {
            let Some(key) = self.next_buildable_plugin()? else {
                return Ok(());
            };
            self.build_plugin_at(key)?;
        }
    }

    /// 异步乐观构建：与 [`Self::build_pending_plugins`] 同序；Async 条目 await `build_async`。
    #[cfg(feature = "async")]
    async fn build_pending_plugins_async(&self) -> Result<(), Error> {
        loop {
            let Some(key) = self.next_buildable_plugin()? else {
                return Ok(());
            };
            self.build_plugin_at_async(key).await?;
        }
    }

    /// 返回下一个可构建的插件键；无待构建返回 `Ok(None)`；无法进展返回依赖错误。
    fn next_buildable_plugin(&self) -> Result<Option<PluginId>, Error> {
        // SlotMap 迭代保持插入序，与原先 Vec 安装顺序语义一致。
        let pending_keys: Vec<PluginId> = self
            .data
            .borrow()
            .plugin_entries
            .iter()
            .filter(|(_, e)| !e.built)
            .map(|(k, _)| k)
            .collect();

        if pending_keys.is_empty() {
            return Ok(None);
        }

        for key in pending_keys.iter().copied() {
            if self.dependencies_satisfied(key) {
                return Ok(Some(key));
            }
        }

        Err(if pending_keys.len() == 1 {
            Error::MissingDependency
        } else {
            Error::CircularDependency
        })
    }

    fn dependencies_satisfied(&self, key: PluginId) -> bool {
        let deps = {
            let data = self.data.borrow();
            let entry = data.plugin_entries.get(key).expect("plugin key present");
            let plugin = entry
                .plugin
                .as_ref()
                .expect("plugin present for dependency check");
            plugin.dependencies()
        };
        deps.into_iter().all(|dep| self.has_service(dep))
    }

    fn build_plugin_at(&self, key: PluginId) -> Result<(), Error> {
        #[cfg(feature = "tracing")]
        let _span = tracing::debug_span!("plugctx.build_plugin").entered();
        #[cfg(feature = "tracing")]
        tracing::debug!(?key, "building plugin");

        let stored = {
            let mut data = self.data.borrow_mut();
            let entry = data
                .plugin_entries
                .get_mut(key)
                .expect("plugin key present");
            entry.plugin.take().expect("plugin present for build")
        };

        self.data
            .borrow_mut()
            .plugin_scope_stack
            .push((key, PluginScope::default()));

        let interceptors = self.snapshot_interceptors();
        for interceptor in &interceptors {
            interceptor.before_plugin_build(stored.as_plugin());
        }

        let mut ctx = self.clone();
        let result = stored.as_plugin().build(&mut ctx);

        if result.is_ok() {
            for interceptor in &interceptors {
                interceptor.after_plugin_build(stored.as_plugin());
            }
        }

        self.commit_build_result(key, stored, result)
    }

    /// 异步构建单个插件：Async 变体调用 `build_async`，Sync 变体调用 `build`。
    #[cfg(feature = "async")]
    async fn build_plugin_at_async(&self, key: PluginId) -> Result<(), Error> {
        let stored = {
            let mut data = self.data.borrow_mut();
            let entry = data
                .plugin_entries
                .get_mut(key)
                .expect("plugin key present");
            entry.plugin.take().expect("plugin present for build")
        };

        self.data
            .borrow_mut()
            .plugin_scope_stack
            .push((key, PluginScope::default()));

        let interceptors = self.snapshot_interceptors();
        for interceptor in &interceptors {
            interceptor.before_plugin_build(stored.as_plugin());
        }

        let mut ctx = self.clone();
        let result = match &stored {
            StoredPlugin::Sync(p) => p.build(&mut ctx),
            StoredPlugin::Async(p) => p.build_async(&mut ctx).await,
        };

        if result.is_ok() {
            for interceptor in &interceptors {
                interceptor.after_plugin_build(stored.as_plugin());
            }
        }

        self.commit_build_result(key, stored, result)
    }

    fn commit_build_result(
        &self,
        key: PluginId,
        stored: StoredPlugin,
        result: Result<(), Error>,
    ) -> Result<(), Error> {
        {
            let mut data = self.data.borrow_mut();
            let (_owner, built_scope) = data
                .plugin_scope_stack
                .pop()
                .expect("scope stack balanced after build");
            let entry = data
                .plugin_entries
                .get_mut(key)
                .expect("plugin key present");
            entry.plugin = Some(stored);
            if result.is_ok() {
                entry.built = true;
                entry.scope = Some(built_scope);
            } else {
                entry.scope = None;
            }
        }

        result.map_err(|_| Error::BuildFailed)
    }

    /// 快照拦截器列表并释放对 `ContextData` 的借用，供钩子内重入。
    fn snapshot_interceptors(&self) -> Vec<InterceptorPtr> {
        self.data.borrow().interceptors.clone()
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

/// 模块标识，供骨架可达性测试使用。
pub const MODULE_NAME: &str = "context";
