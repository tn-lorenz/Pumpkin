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
    ($server:expr, $delay:expr, $body:block) => {{
        use std::sync::Arc;
        use $crate::task::TaskHandler;
        use async_trait::async_trait;

        struct InlineHandler;

        #[async_trait]
        impl TaskHandler for InlineHandler {
            async fn run(&self) $body
        }

        let handler = Arc::new(InlineHandler);
        $server.task_scheduler.schedule_once($delay, handler);
    }};
}
