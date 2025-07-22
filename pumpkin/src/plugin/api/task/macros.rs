#[macro_export]
macro_rules! run_task_later {
    ($server:expr, $delay_ticks:expr, $closure:expr) => {{
        use async_trait::async_trait;
        use pumpkin::plugin::api::task::TaskHandler;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        struct InlineHandler {
            cancel_flag: Arc<AtomicBool>,
            closure: Box<
                dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
                    + Send
                    + Sync,
            >,
        }

        #[async_trait]
        impl TaskHandler for InlineHandler {
            async fn run(&self) {
                if self.cancel_flag.load(Ordering::Relaxed) {
                    return;
                }

                let cancel_flag = self.cancel_flag.clone();
                let cancel = || {
                    cancel_flag.store(true, Ordering::Relaxed);
                };

                (self.closure)().await;
            }
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));

        let closure = {
            let cancel_flag = cancel_flag.clone();
            Box::new(move || {
                let cancel = || cancel_flag.store(true, Ordering::Relaxed);
                Box::pin(({ $closure })())
            })
                as Box<
                    dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
                        + Send
                        + Sync,
                >
        };

        let handler = Arc::new(InlineHandler {
            cancel_flag,
            closure,
        });

        let delay: u64 = $delay_ticks as u64;
        $server.task_scheduler.schedule_once(delay, handler);
    }};
}

#[macro_export]
macro_rules! run_task_timer {
    ($server:expr, $interval_ticks:expr, $closure:expr) => {{
        use async_trait::async_trait;
        use pumpkin::plugin::api::task::TaskHandler;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        struct InlineHandler {
            cancel_flag: Arc<AtomicBool>,
            closure: Box<
                dyn Fn(
                        Box<dyn Fn() + Send + Sync>,
                    )
                        -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
                    + Send
                    + Sync,
            >,
        }

        #[async_trait]
        impl TaskHandler for InlineHandler {
            async fn run(&self) {
                if self.cancel_flag.load(Ordering::Relaxed) {
                    return;
                }

                let cancel_flag = self.cancel_flag.clone();

                let cancel = Box::new(move || {
                    cancel_flag.store(true, Ordering::Relaxed);
                });

                (self.closure)(cancel).await;
            }
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));

        let closure = {
            let cancel_flag = cancel_flag.clone();
            Box::new(move |cancel: Box<dyn Fn() + Send + Sync>| Box::pin($closure(cancel)))
        };

        let handler = Arc::new(InlineHandler {
            cancel_flag,
            closure,
        });

        $server
            .task_scheduler
            .schedule_repeating($interval_ticks as u64, handler);
    }};
}
