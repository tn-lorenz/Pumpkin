#[macro_export]
macro_rules! run_task_later {
    ($server:expr, $delay_ticks:expr, $body:block) => {{
        use std::sync::Arc;
        use $crate::task::TaskHandler;
        use async_trait::async_trait;

        struct InlineHandler;

        #[async_trait]
        impl TaskHandler for InlineHandler {
            async fn run(&self) $body
        }

        let handler = Arc::new(InlineHandler);
        $server.task_scheduler.schedule_once($delay_ticks, handler);
    }};
}

#[macro_export]
macro_rules! run_task_timer {
    ($server:expr, $interval_ticks:expr, $body:tt) => {{
        use async_trait::async_trait;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        use $crate::task::TaskHandler;

        struct InlineHandler {
            cancel_flag: Arc<AtomicBool>,
        }

        #[async_trait]
        impl TaskHandler for InlineHandler {
            async fn run(&self) {
                let cancel = || {
                    self.cancel_flag.store(true, Ordering::Relaxed);
                };

                let cancel_ref = &cancel;

                async move {
                    #[allow(unused_variables)]
                    let cancel = cancel_ref;
                    $body
                }
                .await;
            }
        }

        let handler = Arc::new(InlineHandler {
            cancel_flag: Arc::new(AtomicBool::new(false)),
        });

        $server
            .task_scheduler
            .schedule_repeating($interval_ticks, handler)
    }};
}
