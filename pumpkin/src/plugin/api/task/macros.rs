#[macro_export]
macro_rules! run_task_later {
    ($server:expr, $delay_ticks:expr, $body:block) => {{
        use async_trait::async_trait;
        use pumpkin::plugin::api::task::TaskHandler;
        use std::future::Future;
        use std::pin::Pin;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        struct InlineHandler {
            cancel_flag: Arc<AtomicBool>,
            closure: Box<
                dyn for<'task> Fn(&'task dyn Fn()) -> Pin<Box<dyn Future<Output = ()> + Send>>
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

                (self.closure)(&cancel).await;
            }
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let closure = {
            Box::new(move |cancel: &dyn Fn()| {
                Box::pin(async move {
                    let cancel = cancel;
                    $body
                })
            })
                as Box<
                    dyn for<'task> Fn(&'task dyn Fn()) -> Pin<Box<dyn Future<Output = ()> + Send>>
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
    ($server:expr, $interval_ticks:expr, $body:block) => {{
        use async_trait::async_trait;
        use pumpkin::plugin::api::task::TaskHandler;
        use std::future::Future;
        use std::pin::Pin;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        struct InlineHandler {
            cancel_flag: Arc<AtomicBool>,
            closure: Box<
                dyn for<'task> Fn(&'task dyn Fn()) -> Pin<Box<dyn Future<Output = ()> + Send>>
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

                (self.closure)(&cancel).await;
            }
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let closure = {
            Box::new(move |cancel: &dyn Fn()| {
                Box::pin(async move {
                    let cancel = cancel;
                    $body
                })
            })
                as Box<
                    dyn for<'task> Fn(&'task dyn Fn()) -> Pin<Box<dyn Future<Output = ()> + Send>>
                        + Send
                        + Sync,
                >
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
