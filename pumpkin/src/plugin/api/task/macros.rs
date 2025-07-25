#[macro_export]
macro_rules! run_task_later {
    ($server:expr, $delay_ticks:expr, $body:block) => {{
        use async_trait::async_trait;
        use std::future::Future;
        use std::pin::Pin;
        use std::sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        };
        use $crate::plugin::api::task::TaskHandler;

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
        let future: Pin<Box<dyn Future<Output = ()> + Send>> = Box::pin(async move {
            let cancel = || {
                cancel_flag.store(true, Ordering::Relaxed);
            };

            $body
        });

        let handler = Arc::new(InlineOnceHandler {
            //cancel_flag,
            cancel_flag: cancel_flag.clone(),
            future: Mutex::new(Some(future)),
        });

        $server
            .task_scheduler
            .schedule_once($delay_ticks as u64, handler);

        cancel_flag
    }};
}

#[macro_export]
macro_rules! run_task_timer {
    ($server:expr, $interval_ticks:expr, $closure:expr) => {{
        use std::future::Future;
        use std::pin::Pin;
        use std::sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        };

        let server = Arc::clone(&$server);
        let task_cell = Arc::new(Mutex::new(None::<Arc<dyn Fn() + Send + Sync + 'static>>));
        let cancel_flag = Arc::new(AtomicBool::new(false));

        let task = {
            let task_cell = Arc::clone(&task_cell);
            let server = Arc::clone(&server);
            let cancel_flag = cancel_flag.clone();

            Arc::new(move || {
                if cancel_flag.load(Ordering::Relaxed) {
                    return;
                }

                let cancel_flag = cancel_flag.clone();
                let server = server.clone();
                let task_guard = task_cell.lock().unwrap();

                if let Some(task) = task_guard.as_ref() {
                    let task = Arc::clone(task);
                    drop(task_guard);
                    $crate::run_task_later!(server.clone(), 0, {
                        let cancel_flag = cancel_flag.clone();
                        let future = $closure(move || {
                            cancel_flag.store(true, Ordering::Relaxed);
                        });
                        future.await;

                        if !cancel_flag.load(Ordering::Relaxed) {
                            run_task_later!(server.clone(), $interval_ticks, {
                                task();
                            });
                        }
                    });
                }
            }) as Arc<dyn Fn() + Send + Sync + 'static>
        };

        *task_cell.lock().unwrap() = Some(task.clone());
        run_task_later!(server, $interval_ticks, {
            task();
        });

        cancel_flag
    }};
}
