#[macro_export]
macro_rules! run_task_later {
    ($server:expr, $delay_ticks:expr, $body:block) => {{
        use async_trait::async_trait;
        use pumpkin::plugin::api::task::TaskHandler;
        use std::future::Future;
        use std::pin::Pin;
        use std::sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        };

        struct InlineOnceHandler {
            cancel_flag: Arc<AtomicBool>,
            future: Mutex<Option<Pin<Box<dyn Future<Output = ()> + Send>>>>,
        }

        #[async_trait]
        impl TaskHandler for InlineOnceHandler {
            async fn run(&self) {
                if self.cancel_flag.load(Ordering::Relaxed) {
                    return;
                }

                let fut = {
                    let mut guard = self.future.lock().unwrap();
                    guard.take()
                };

                if let Some(fut) = fut {
                    fut.await;
                }
            }
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));

        let future: Pin<Box<dyn Future<Output = ()> + Send>> = Box::pin(async move { $body });

        let handler = Arc::new(InlineOnceHandler {
            cancel_flag,
            future: std::sync::Mutex::new(Some(future)),
        });

        $server
            .task_scheduler
            .schedule_once($delay_ticks as u64, handler);
    }};
}

#[macro_export]
macro_rules! run_task_timer {
    ($server:expr, $interval_ticks:expr, $body:block) => {{
        use pumpkin::plugin::api::server;
        use std::sync::Arc;

        fn schedule_next(server: Arc<Server>, interval: u64, task: Arc<dyn Fn() + Send + Sync>) {
            run_task_later!(server, interval, {
                task();
            });
        }

        let server = Arc::new($server.clone());
        let task: Arc<dyn Fn() + Send + Sync> = Arc::new({
            let server = server.clone();
            move || {
                let server = server.clone();
                run_task_later!(server.clone(), 0, $body);
                schedule_next(server, $interval_ticks as u64, Arc::clone(&task));
            }
        });

        schedule_next(server, $interval_ticks as u64, task);
    }};
}

/*#[macro_export]
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
            closure: Arc<
                dyn Fn(Arc<dyn Fn() + Send + Sync>) -> Pin<Box<dyn Future<Output = ()> + Send>>
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
                let cancel = Arc::new(move || {
                    cancel_flag.store(true, Ordering::Relaxed);
                });

                (self.closure)(cancel).await;
            }
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let closure = Arc::new(move |cancel: Arc<dyn Fn() + Send + Sync>| {
            Box::pin(async move {
                let cancel = cancel;
                $body
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        });

        let handler = Arc::new(InlineHandler {
            cancel_flag,
            closure,
        });

        $server
            .task_scheduler
            .schedule_repeating($interval_ticks as u64, handler);
    }};
}*/
