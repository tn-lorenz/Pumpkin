use crate::plugin::task::{Cancelable, TOKIO_RUNTIME, TaskHandler};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

pub fn start_loop<H>(delay: Duration, handler: Arc<H>)
where
    H: TaskHandler + Cancelable + 'static,
{
    TOKIO_RUNTIME.spawn(run_task_timer(delay, handler));
}

async fn run_task_timer<H>(delay: Duration, handler: Arc<H>)
where
    H: TaskHandler + Cancelable + 'static,
{
    loop {
        sleep(delay).await;
        handler.run().await;

        if handler.should_cancel().await {
            break;
        }
    }
}

#[macro_export]
macro_rules! run_task_timer {
    ($delay:expr, $body:block) => {{
        use async_trait::async_trait;
        use std::sync::Arc;
        use $crate::task::{CancelableHandler, TaskHandler, start_loop};

        struct InlineHandler;

        #[async_trait]
        impl TaskHandler for InlineHandler {
            async fn run(&self) $body
        }

        let base = Arc::new(InlineHandler);
        let handler = CancelableHandler::new(base);
        start_loop($delay, handler.clone());
        handler
    }};
}
