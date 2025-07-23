#[macro_export]
macro_rules! run_task_later {
    ($server:expr, $delay_ticks:expr, $body:block) => {{
        use async_trait::async_trait;
        use std::future::Future;
        use std::pin::Pin;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        struct InlineHandler {
            cancel_flag: Arc<AtomicBool>,
            closure: Arc<
                dyn Fn(Arc<dyn Fn() + Send + Sync>) -> Pin<Box<dyn Future<Output = ()> + Send>>
                    + Send
                    + Sync,
            >,
        }

        #[async_trait]
        impl pumpkin::plugin::api::task::TaskHandler for InlineHandler {
            async fn run(&self) {
                if self.cancel_flag.load(Ordering::Relaxed) {
                    return;
                }

                let cancel_flag = self.cancel_flag.clone();
                let cancel = Arc::new(move || {
                    cancel_flag.store(true, Ordering::Relaxed);
                });

                (self.closure)(cancel.clone()).await;
            }
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let closure = {
            let cancel_flag = cancel_flag.clone();
            Arc::new(move |cancel: Arc<dyn Fn() + Send + Sync>| {
                Box::pin(async move {
                    let cancel = &*cancel;
                    $body
                }) as Pin<Box<dyn Future<Output = ()> + Send>>
            })
        };

        let handler = Arc::new(InlineHandler {
            cancel_flag,
            closure,
        });

        $server
            .task_scheduler
            .schedule_once($delay_ticks as u64, handler);
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
            closure:
                Box<dyn Fn(&dyn Fn()) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>,
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
        let closure = Box::new(move |cancel: &dyn Fn()| {
            Box::pin(async move {
                let cancel = cancel;
                (async move { $body }).await;
            })
        })
            as Box<dyn Fn(&dyn Fn()) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

        let handler = Arc::new(InlineHandler {
            cancel_flag,
            closure,
        });

        $server
            .task_scheduler
            .schedule_repeating($interval_ticks as u64, handler);
    }};
}
