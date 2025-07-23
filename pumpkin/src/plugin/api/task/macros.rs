#[macro_export]
macro_rules! run_task_later {
    ($server:expr, $delay_ticks:expr, $body:block) => {{
        use async_trait::async_trait;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        use $crate::plugin::api::task::TaskHandler;

        struct InlineHandler {
            cancel_flag: Arc<AtomicBool>,
        }

        #[async_trait]
        impl TaskHandler for InlineHandler {
            async fn run(&self) {
                if self.cancel_flag.load(Ordering::Relaxed) {
                    return;
                }

                $body
            }
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let handler = Arc::new(InlineHandler {
            cancel_flag: cancel_flag.clone(),
        });

        $server.task_scheduler.schedule_once($delay_ticks, handler);
        cancel_flag
    }};
}

#[macro_export]
macro_rules! run_task_timer {
    ($server:expr, $interval_ticks:expr, $body:block) => {{
        use async_trait::async_trait;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        use $crate::plugin::api::task::TaskHandler;

        struct InlineHandler {
            cancel_flag: Arc<AtomicBool>,
        }

        #[async_trait]
        impl TaskHandler for InlineHandler {
            async fn run(&self) {
                if self.cancel_flag.load(Ordering::Relaxed) {
                    return;
                }

                $body
            }
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let handler = Arc::new(InlineHandler {
            cancel_flag: cancel_flag.clone(),
        });

        $server
            .task_scheduler
            .schedule_repeating($interval_ticks, handler);
        cancel_flag
    }};
}
