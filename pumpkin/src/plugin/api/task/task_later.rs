use crate::plugin::task::TaskHandler;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[allow(dead_code)]
async fn run_task_later<H>(delay: Duration, handler: Arc<H>)
where
    H: TaskHandler + 'static,
{
    sleep(delay).await;
    handler.run().await;
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
