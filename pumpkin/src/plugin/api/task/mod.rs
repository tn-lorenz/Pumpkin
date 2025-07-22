pub mod task_later;
pub mod task_timer;

use std::sync::{Arc, Mutex, LazyLock};
use tokio::runtime::Runtime;
use tokio::sync::watch;

pub static TOKIO_RUNTIME: LazyLock<Runtime> =
    LazyLock::new(|| Runtime::new().expect("Failed to create global Tokio Runtime"));

#[async_trait::async_trait]
pub trait TaskHandler: Send + Sync {
    async fn run(&self);
}

#[async_trait::async_trait]
pub trait Cancelable: Send + Sync {
    async fn cancel(&self);
    async fn should_cancel(&self) -> bool;
}

pub struct CancelableHandler<T: TaskHandler> {
    inner: Arc<T>,
    cancel_rx: Mutex<watch::Receiver<bool>>,
    cancel_tx: watch::Sender<bool>,
}

#[derive(Clone)]
pub struct CancelHandle {
    cancel_tx: watch::Sender<bool>,
}

impl CancelHandle {
    pub fn cancel(&self) {
        let _ = self.cancel_tx.send(true);
    }
}

impl Drop for CancelHandle {
    fn drop(&mut self) {
        let _ = self.cancel_tx.send(true);
    }
}

impl<T: TaskHandler> CancelableHandler<T> {
    pub fn new(inner: Arc<T>) -> Arc<Self> {
        let (tx, rx) = watch::channel(false);
        Arc::new(Self {
            inner,
            cancel_rx: Mutex::new(rx),
            cancel_tx: tx,
        })
    }

    pub fn cancel_handle(&self) -> CancelHandle {
        CancelHandle {
            cancel_tx: self.cancel_tx.clone(),
        }
    }
}

#[async_trait::async_trait]
impl<T: TaskHandler> TaskHandler for CancelableHandler<T> {
    async fn run(&self) {
        self.inner.run().await;
    }
}

#[async_trait::async_trait]
impl<T: TaskHandler> Cancelable for CancelableHandler<T> {
    async fn cancel(&self) {
        let _ = self.cancel_tx.send(true);
    }

    async fn should_cancel(&self) -> bool {
        let rx = self.cancel_rx.lock().unwrap();
        rx.has_changed().unwrap_or(false) && *rx.borrow()
    }
}
