//! 共享存储原语：默认 `Rc`/`RefCell`，`thread-safe` 下切换为 `Arc`/`parking_lot` 锁（FR22 / NFR5）。

use std::any::Any;
use std::ops::{Deref, DerefMut};

#[cfg(not(feature = "thread-safe"))]
use std::cell::{Cell, Ref, RefCell, RefMut};
#[cfg(not(feature = "thread-safe"))]
use std::rc::{Rc, Weak};

#[cfg(feature = "thread-safe")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "thread-safe")]
use std::sync::{Arc, Weak as ArcWeak};

#[cfg(feature = "thread-safe")]
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

// ----- 类型擦除盒子 -----

#[cfg(not(feature = "thread-safe"))]
pub type AnyBox = Box<dyn Any>;
#[cfg(feature = "thread-safe")]
pub type AnyBox = Box<dyn Any + Send + Sync>;

// ----- 取消 / 标志位 -----

/// 可在句柄与存储槽之间共享的布尔标志。
#[derive(Clone, Debug)]
pub struct Flag {
    #[cfg(not(feature = "thread-safe"))]
    inner: Rc<Cell<bool>>,
    #[cfg(feature = "thread-safe")]
    inner: Arc<AtomicBool>,
}

impl Flag {
    pub fn new(value: bool) -> Self {
        #[cfg(not(feature = "thread-safe"))]
        {
            Self {
                inner: Rc::new(Cell::new(value)),
            }
        }
        #[cfg(feature = "thread-safe")]
        {
            Self {
                inner: Arc::new(AtomicBool::new(value)),
            }
        }
    }

    pub fn get(&self) -> bool {
        #[cfg(not(feature = "thread-safe"))]
        {
            self.inner.get()
        }
        #[cfg(feature = "thread-safe")]
        {
            self.inner.load(Ordering::Relaxed)
        }
    }

    pub fn set(&self, value: bool) {
        #[cfg(not(feature = "thread-safe"))]
        {
            self.inner.set(value);
        }
        #[cfg(feature = "thread-safe")]
        {
            self.inner.store(value, Ordering::Relaxed);
        }
    }
}

// ----- Shared / Weak 容器 -----

/// `ContextData` 等内部状态的共享句柄。
#[derive(Debug)]
pub struct Shared<T> {
    #[cfg(not(feature = "thread-safe"))]
    inner: Rc<RefCell<T>>,
    #[cfg(feature = "thread-safe")]
    inner: Arc<RwLock<T>>,
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Shared<T> {
    pub fn new(value: T) -> Self {
        #[cfg(not(feature = "thread-safe"))]
        {
            Self {
                inner: Rc::new(RefCell::new(value)),
            }
        }
        #[cfg(feature = "thread-safe")]
        {
            Self {
                inner: Arc::new(RwLock::new(value)),
            }
        }
    }

    pub fn borrow(&self) -> SharedReadGuard<'_, T> {
        #[cfg(not(feature = "thread-safe"))]
        {
            SharedReadGuard {
                inner: self.inner.borrow(),
            }
        }
        #[cfg(feature = "thread-safe")]
        {
            SharedReadGuard {
                inner: self.inner.read(),
            }
        }
    }

    pub fn borrow_mut(&self) -> SharedWriteGuard<'_, T> {
        #[cfg(not(feature = "thread-safe"))]
        {
            SharedWriteGuard {
                inner: self.inner.borrow_mut(),
            }
        }
        #[cfg(feature = "thread-safe")]
        {
            SharedWriteGuard {
                inner: self.inner.write(),
            }
        }
    }

    pub fn downgrade(&self) -> SharedWeak<T> {
        #[cfg(not(feature = "thread-safe"))]
        {
            SharedWeak {
                inner: Rc::downgrade(&self.inner),
            }
        }
        #[cfg(feature = "thread-safe")]
        {
            SharedWeak {
                inner: Arc::downgrade(&self.inner),
            }
        }
    }

    #[allow(dead_code)]
    pub fn strong_count(&self) -> usize {
        #[cfg(not(feature = "thread-safe"))]
        {
            Rc::strong_count(&self.inner)
        }
        #[cfg(feature = "thread-safe")]
        {
            Arc::strong_count(&self.inner)
        }
    }

    #[allow(dead_code)]
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        #[cfg(not(feature = "thread-safe"))]
        {
            Rc::ptr_eq(&this.inner, &other.inner)
        }
        #[cfg(feature = "thread-safe")]
        {
            Arc::ptr_eq(&this.inner, &other.inner)
        }
    }

    /// 映射读守卫为子引用（供 `get` / `get_trait`）。
    pub fn try_map_read<'a, U: ?Sized + 'a, F>(&'a self, f: F) -> Option<ServiceRef<'a, U>>
    where
        F: FnOnce(&T) -> Option<&U>,
    {
        #[cfg(not(feature = "thread-safe"))]
        {
            Ref::filter_map(self.inner.borrow(), f).ok()
        }
        #[cfg(feature = "thread-safe")]
        {
            RwLockReadGuard::try_map(self.inner.read(), f).ok()
        }
    }

    /// 映射写守卫为子可变引用（供 `get_mut`）。
    pub fn try_map_write<'a, U: ?Sized + 'a, F>(&'a self, f: F) -> Option<ServiceMut<'a, U>>
    where
        F: FnOnce(&mut T) -> Option<&mut U>,
    {
        #[cfg(not(feature = "thread-safe"))]
        {
            RefMut::filter_map(self.inner.borrow_mut(), f).ok()
        }
        #[cfg(feature = "thread-safe")]
        {
            RwLockWriteGuard::try_map(self.inner.write(), f).ok()
        }
    }
}

/// [`Shared`] 的弱引用。
#[derive(Debug)]
pub struct SharedWeak<T> {
    #[cfg(not(feature = "thread-safe"))]
    inner: Weak<RefCell<T>>,
    #[cfg(feature = "thread-safe")]
    inner: ArcWeak<RwLock<T>>,
}

impl<T> Clone for SharedWeak<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> SharedWeak<T> {
    pub fn upgrade(&self) -> Option<Shared<T>> {
        self.inner.upgrade().map(|inner| Shared { inner })
    }

    #[allow(dead_code)] // retained for diagnostics / future retain helpers
    pub fn strong_count(&self) -> usize {
        self.inner.strong_count()
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        self.inner.ptr_eq(&other.inner)
    }
}

pub struct SharedReadGuard<'a, T> {
    #[cfg(not(feature = "thread-safe"))]
    inner: Ref<'a, T>,
    #[cfg(feature = "thread-safe")]
    inner: RwLockReadGuard<'a, T>,
}

impl<T> Deref for SharedReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}

pub struct SharedWriteGuard<'a, T> {
    #[cfg(not(feature = "thread-safe"))]
    inner: RefMut<'a, T>,
    #[cfg(feature = "thread-safe")]
    inner: RwLockWriteGuard<'a, T>,
}

impl<T> Deref for SharedWriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T> DerefMut for SharedWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

// ----- 服务引用别名（get / get_mut / get_trait） -----

#[cfg(not(feature = "thread-safe"))]
pub type ServiceRef<'a, T> = Ref<'a, T>;
#[cfg(feature = "thread-safe")]
pub type ServiceRef<'a, T> = MappedRwLockReadGuard<'a, T>;

#[cfg(not(feature = "thread-safe"))]
pub type ServiceMut<'a, T> = RefMut<'a, T>;
#[cfg(feature = "thread-safe")]
pub type ServiceMut<'a, T> = MappedRwLockWriteGuard<'a, T>;

// ----- 可变闭包单元格（事件监听器） -----

#[cfg(not(feature = "thread-safe"))]
pub type SyncHandler = Rc<RefCell<dyn FnMut(&dyn Any)>>;
#[cfg(feature = "thread-safe")]
pub type SyncHandler = Arc<Mutex<dyn FnMut(&dyn Any) + Send>>;

#[cfg(all(feature = "parallel", not(feature = "thread-safe")))]
pub type AsyncHandler = Rc<
    RefCell<
        dyn FnMut(&dyn Any) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'static>>,
    >,
>;
#[cfg(all(feature = "parallel", feature = "thread-safe"))]
pub type AsyncHandler = Arc<
    Mutex<
        dyn FnMut(
                &dyn Any,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>>
            + Send,
    >,
>;

// try_*_handler_mut 已内联到 Context::{emit,emit_parallel}。

#[cfg(not(feature = "thread-safe"))]
pub fn new_sync_handler(f: impl FnMut(&dyn Any) + 'static) -> SyncHandler {
    Rc::new(RefCell::new(f))
}

#[cfg(feature = "thread-safe")]
pub fn new_sync_handler(f: impl FnMut(&dyn Any) + Send + 'static) -> SyncHandler {
    Arc::new(Mutex::new(f))
}

/// 拦截器共享指针。
#[cfg(not(feature = "thread-safe"))]
pub type InterceptorPtr = Rc<dyn crate::interceptor::ContextInterceptor>;
#[cfg(feature = "thread-safe")]
pub type InterceptorPtr = Arc<dyn crate::interceptor::ContextInterceptor + Send + Sync>;

#[cfg(not(feature = "thread-safe"))]
pub fn new_interceptor<I: crate::interceptor::ContextInterceptor + 'static>(
    i: I,
) -> InterceptorPtr {
    Rc::new(i)
}
#[cfg(feature = "thread-safe")]
pub fn new_interceptor<I: crate::interceptor::ContextInterceptor + Send + Sync + 'static>(
    i: I,
) -> InterceptorPtr {
    Arc::new(i)
}

/// Effect cleanup 盒子。
#[cfg(not(feature = "thread-safe"))]
pub type CleanupBox = Box<dyn FnOnce()>;
#[cfg(feature = "thread-safe")]
pub type CleanupBox = Box<dyn FnOnce() + Send + Sync>;

#[cfg(not(feature = "thread-safe"))]
pub fn erase_any<T: 'static>(value: T) -> AnyBox {
    Box::new(value)
}

#[cfg(feature = "thread-safe")]
pub fn erase_any<T: Send + Sync + 'static>(value: T) -> AnyBox {
    Box::new(value)
}

#[cfg(not(feature = "thread-safe"))]
pub fn erase_cleanup<G: FnOnce() + 'static>(cleanup: G) -> CleanupBox {
    Box::new(cleanup)
}

#[cfg(feature = "thread-safe")]
pub fn erase_cleanup<G: FnOnce() + Send + Sync + 'static>(cleanup: G) -> CleanupBox {
    Box::new(cleanup)
}
