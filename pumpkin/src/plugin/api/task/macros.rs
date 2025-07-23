#[macro_export]
macro_rules! run_task_later {
    ($server:expr, $delay_ticks:expr, $body:expr) => {{
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

        let future: Pin<Box<dyn Future<Output = ()> + Send>> = match async { $body }.await {
            _ => Box::pin(async {}),
        };

        let handler = Arc::new(InlineOnceHandler {
            cancel_flag,
            future: Mutex::new(Some(future)),
        });

        $server
            .task_scheduler
            .schedule_once($delay_ticks as u64, handler);
    }};
}

#[macro_export]
macro_rules! run_task_timer {
    ($server:expr, $interval_ticks:expr, $body:block) => {{
        use std::sync::{Arc, Mutex};

        let server = Arc::clone(&$server);
        let task_cell = Arc::new(Mutex::new(None::<Arc<dyn Fn() + Send + Sync + 'static>>));

        let task = {
            let task_cell = Arc::clone(&task_cell);
            let server = Arc::clone(&server);

            Arc::new(move || {
                run_task_later!(server.clone(), 0, { $body });

                if let Some(task) = task_cell.lock().unwrap().as_ref() {
                    run_task_later!(server.clone(), $interval_ticks, {
                        task();
                    });
                }
            }) as Arc<dyn Fn() + Send + Sync + 'static>
        };

        *task_cell.lock().unwrap() = Some(task.clone());

        run_task_later!(server, $interval_ticks, {
            task();
        });
    }};
}

/*#[macro_export]
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
}*/

/*#[macro_export]
macro_rules! run_task_timer {
    ($server:expr, $interval_ticks:expr, $($body:tt)*) => {{
        use pumpkin::server::Server;
        use std::sync::{Arc, Mutex};

        let server: Arc<Server> = $server;
        let task_ref = Arc::new(Mutex::new(None));

        let task_closure: Arc<dyn Fn() + Send + Sync + 'static> = {
            let server = Arc::clone(&server);
            let task_ref = Arc::clone(&task_ref);

            Arc::new(move || {
                let server = server.clone();
                let task = task_ref.lock().unwrap().clone().unwrap();

                let future = async move {
                    $($body)*
                };

                run_task_later!(server.clone(), 0, future);

                run_task_later!(server.clone(), $interval_ticks as u64, async move {
                    task();
                });
            })
        };

        *task_ref.lock().unwrap() = Some(task_closure.clone());
        run_task_later!(server.clone(), $interval_ticks as u64, async move {
            task_closure();
        });
    }};
}*/

/*#[macro_export]
macro_rules! run_task_timer {
    ($server:expr, $interval_ticks:expr, $body:expr) => {{
        use pumpkin::server::Server;
        use std::sync::{Arc, Mutex};

        fn schedule_next(server: Arc<Server>, interval: u64, task: Arc<dyn Fn() + Send + Sync>) {
            run_task_later!(server.clone(), interval, {
                task();
            });
        }

        let server: Arc<Server> = $server;
        let task_ref = Arc::new(Mutex::new(None));

        let task_closure: Arc<dyn Fn() + Send + Sync> = {
            let server = Arc::clone(&server);
            let task_ref_clone = Arc::clone(&task_ref);

            Arc::new(move || {
                let server = Arc::clone(&server);
                let task = Arc::clone(task_ref_clone.lock().unwrap().as_ref().unwrap());

                let body = $body;
                run_task_later!(server.clone(), 0, body);

                schedule_next(server, $interval_ticks as u64, task);
            })
        };

        *task_ref.lock().unwrap() = Some(Arc::clone(&task_closure));
        schedule_next(server, $interval_ticks as u64, Arc::clone(&task_closure));
    }};
}*/

/*#[macro_export]
macro_rules! run_task_timer {
    ($server:expr, $interval_ticks:expr, $body:block) => {{
        use pumpkin::server::Server;
        use std::sync::{Arc, Mutex};

        fn schedule_next(server: Arc<Server>, interval: u64, task: Arc<dyn Fn() + Send + Sync>) {
            run_task_later!(server.clone(), interval, {
                task();
            });
        }

        let server: Arc<Server> = $server;
        let task_ref = Arc::new(Mutex::new(None));

        let task_closure: Arc<dyn Fn() + Send + Sync> = {
            let server = Arc::clone(&server);
            let task_ref_clone = Arc::clone(&task_ref);

            Arc::new(move || {
                let server = Arc::clone(&server);
                let task = Arc::clone(task_ref_clone.lock().unwrap().as_ref().unwrap());

                run_task_later!(server.clone(), 0, {
                    { $body }
                });

                schedule_next(server, $interval_ticks as u64, task);
            })
        };

        *task_ref.lock().unwrap() = Some(Arc::clone(&task_closure));
        schedule_next(server, $interval_ticks as u64, Arc::clone(&task_closure));
    }};
}*/

/*#[macro_export]
macro_rules! run_task_timer {
    ($server:expr, $interval_ticks:expr, $body:block) => {{
        use pumpkin::server::Server;
        use std::sync::{Arc, Mutex};

        fn schedule_next(server: Arc<Server>, interval: u64, task: Arc<dyn Fn() + Send + Sync>) {
            run_task_later!(server, interval, {
                task();
            });
        }

        let server: Arc<Server> = $server;
        let task_ref = Arc::new(Mutex::new(None));

        let task_closure: Arc<dyn Fn() + Send + Sync> = {
            let server = Arc::clone(&server);
            let task_ref_clone = Arc::clone(&task_ref);

            Arc::new(move || {
                let server = Arc::clone(&server);
                let task = Arc::clone(task_ref_clone.lock().unwrap().as_ref().unwrap());

                run_task_later!(server.clone(), 0, $body);
                schedule_next(server, $interval_ticks as u64, task);
            })
        };

        *task_ref.lock().unwrap() = Some(Arc::clone(&task_closure));

        schedule_next(server, $interval_ticks as u64, Arc::clone(&task_closure));
    }};
}*/

/*#[macro_export]
macro_rules! run_task_timer {
    ($server:expr, $interval_ticks:expr, $body:block) => {{
        use pumpkin::server::Server;
        use std::sync::Arc;

        fn schedule_next(server: Arc<Server>, interval: u64, task: Arc<dyn Fn() + Send + Sync>) {
            run_task_later!(server, interval, {
                task();
            });
        }

        let server = Arc::clone(&$server);
        let task_ref = Arc::new(std::sync::Mutex::new(None));

        let task_closure: Arc<dyn Fn() + Send + Sync> = {
            let server = Arc::clone(&server);
            let task_ref_clone = Arc::clone(&task_ref);

            Arc::new(move || {
                let server = Arc::clone(&server);
                let task = Arc::clone(&task_ref_clone.lock().unwrap().as_ref().unwrap());

                run_task_later!(server.clone(), 0, $body);
                schedule_next(server, $interval_ticks as u64, task);
            })
        };

        *task_ref.lock().unwrap() = Some(Arc::clone(&task_closure));

        schedule_next(server, $interval_ticks as u64, Arc::clone(&task_closure));
    }};
}*/

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
