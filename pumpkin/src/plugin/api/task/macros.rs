#[macro_export]
macro_rules! run_task_later {
    ($server:expr, $delay_ticks:expr, $body:block) => {{
        use async_trait::async_trait;
        use pumpkin::plugin::api::task::TaskHandler;
        use std::sync::Arc;

        struct InlineHandler;

        #[async_trait]
        impl TaskHandler for InlineHandler {
            async fn run(&self) {
                $body
            }
        }

        let handler = Arc::new(InlineHandler);
        let delay: u64 = $delay_ticks as u64;
        $server.task_scheduler.schedule_once(delay, handler);
    }};
}

#[macro_export]
macro_rules! run_task_timer {
    ($server:expr, $interval_ticks:expr, $($body:tt)*) => {{
        use async_trait::async_trait;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        use pumpkin::plugin::api::task::TaskHandler;

        struct InlineHandler {
            cancel_flag: Arc<AtomicBool>,
        }

        #[async_trait]
        impl TaskHandler for InlineHandler {
            async fn run(&self) {
                let cancel_flag = self.cancel_flag.clone();

                let cancel = || {
                    cancel_flag.store(true, Ordering::Relaxed);
                };

                async move {
                    $($body)*
                }.await;
            }
        }

        let handler = Arc::new(InlineHandler {
            cancel_flag: Arc::new(AtomicBool::new(false)),
        });

        $server
            .task_scheduler
            .schedule_repeating($interval_ticks as u64, handler)
    }};
}
