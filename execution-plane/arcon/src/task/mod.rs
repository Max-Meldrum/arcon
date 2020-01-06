pub mod executor;

use futures::{
    future::{BoxFuture, FutureExt},
    task::{waker_ref, ArcWake},
};
use kompact::prelude::ActorRefStrong;
use std::cell::UnsafeCell;
use std::future::Future;
use std::sync::Arc;

type ExecutorMsg = ActorRefStrong<Arc<ArconTask>>;

pub struct ArconTask {
    /// Our boxed future
    future: UnsafeCell<Option<BoxFuture<'static, ()>>>,
    /// Set if future needs to be rescheduled
    executor: UnsafeCell<Option<ExecutorMsg>>,
}

impl ArconTask {
    pub fn new(f: impl Future<Output = ()> + 'static + Send) -> Arc<ArconTask> {
        let future = f.boxed();
        Arc::new(ArconTask {
            future: UnsafeCell::new(Some(future)),
            executor: UnsafeCell::new(None),
        })
    }
    pub fn set_executor(&mut self, executor: ActorRefStrong<Arc<ArconTask>>) {
        let executor_slot = self.executor.get();
        unsafe {
            *executor_slot = Some(executor);
        }
    }
}

impl ArcWake for ArconTask {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // Implement `wake` by sending this task back onto the task channel
        // so that it will be polled again by the executor.
        let cloned = arc_self.clone();
        let executor_slot = cloned.executor.get();
        if let Some(ref executor) = unsafe { (*executor_slot).as_ref() } {
            executor.tell(cloned);
        }
        // Else cancel?
    }
}

impl std::fmt::Debug for ArconTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

unsafe impl Send for ArconTask {}
unsafe impl Sync for ArconTask {}
