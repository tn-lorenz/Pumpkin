#[macro_export]
macro_rules! run_task_timer {
    ($server:expr, $interval:expr, $body:block) => {{
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
                let cancel_flag = self.cancel_flag.clone();

                fn make_cancel(cancel_flag: Arc<AtomicBool>) -> impl Fn() {
                    move || {
                        cancel_flag.store(true, Ordering::Relaxed);
                    }
                }

                let cancel = make_cancel(cancel_flag.clone());

                $body
            }
        }

        let handler = Arc::new(InlineHandler {
            cancel_flag: Arc::new(AtomicBool::new(false)),
        });

        let cancel_flag = handler.cancel_flag.clone();
        $server
            .task_scheduler
            .schedule_repeating($interval, handler);

        cancel_flag
    }};
}
