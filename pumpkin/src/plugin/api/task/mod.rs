use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::time::{Duration, sleep};
use once_cell::sync::Lazy;

pub static TOKIO_RUNTIME: Lazy<Runtime> =
    Lazy::new(|| Runtime::new().expect("Failed to create global Tokio Runtime"));

#[async_trait::async_trait]
pub trait TaskHandler: Send + Sync {
    async fn run(&self);
    async fn cancel(&self);
}

pub fn start_loop<H>(delay: Duration, handler: Arc<H>)
where
    H: TaskHandler + 'static,
{
    TOKIO_RUNTIME.spawn(run_task_timer(delay, handler));
}

async fn run_task_timer<H>(delay: Duration, handler: Arc<H>)
where
    H: TaskHandler + 'static,
{
    loop {
        sleep(delay).await;
        handler.run().await;
    }
}

async fn run_task_later<H>(delay: Duration, handler: Arc<H>)
where
    H: TaskHandler + 'static,
{
    sleep(delay).await;
    handler.run().await;
}

#[macro_export]
macro_rules! run_task_timer {
    ($delay:expr, $body:block) => {{
        use std::sync::Arc;
        use $crate::task::{start_loop, TaskHandler};

        struct InlineHandler;

        #[async_trait::async_trait]
        impl TaskHandler for InlineHandler {
            async fn run(&self) $body
        }

        let handler = Arc::new(InlineHandler);
        start_loop($delay, handler);
    }};
}

#[macro_export]
macro_rules! run_task_later {
    ($delay:expr, $body:block) => {{
        use std::sync::Arc;
        use $crate::task::{run_task_later, TaskHandler};
        use async_trait::async_trait;

        struct InlineHandler;

        #[async_trait]
        impl TaskHandler for InlineHandler {
            async fn run(&self) $body
        }

        let handler = Arc::new(InlineHandler);

        $crate::TOKIO_RUNTIME.spawn(run_task_later($delay, handler));
    }};
}

