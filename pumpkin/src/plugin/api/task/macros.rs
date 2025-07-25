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
        use $crate::plugin::api::task::TaskHandler;

        struct DelayedTask<F>
        where
            F: Future<Output = ()> + Send + Sync + 'static,
        {
            server: $crate::server::Server,
            delay: u64,
            ran: Arc<AtomicBool>,
            fut: fn() -> Pin<Box<F>>,
        }

        #[async_trait]
        impl<F> TaskHandler for DelayedTask<F>
        where
            F: Future<Output = ()> + Send + Sync + 'static,
        {
            async fn run(&self) {
                if self.ran.swap(true, Ordering::SeqCst) {
                    return;
                }

                let delay = self.delay;
                let fut = self.fut;

                self.server.scheduler().schedule_delay(delay).await;
                fut().await;
            }
        }

        let ran = Arc::new(AtomicBool::new(false));
        let fut = || Box::pin(async move { $body });

        let task = DelayedTask {
            server: $server.clone(),
            delay: $delay_ticks,
            ran,
            fut,
        };

        $server.scheduler().submit(Box::new(task));
    }};
}

#[macro_export]
macro_rules! run_task_timer {
    ($server:expr, $interval_ticks:expr, $closure:expr) => {{
        use std::sync::{Arc, Mutex};

        let server = Arc::clone(&$server);
        let task_cell = Arc::new(Mutex::new(None::<Arc<dyn Fn() + Send + Sync + 'static>>));
        let user_closure = Arc::new($closure);

        let task = {
            let task_cell = Arc::clone(&task_cell);
            let server = Arc::clone(&server);
            let user_closure = Arc::clone(&user_closure);

            Arc::new(move || {
                let user_closure_for_task = Arc::clone(&user_closure);
                let task_guard = task_cell.lock().unwrap();

                if let Some(task) = task_guard.as_ref() {
                    let task_clone = Arc::clone(task);
                    drop(task_guard);

                    $crate::run_task_later!(server.clone(), $interval_ticks, {
                        user_closure_for_task().await;
                        task_clone();
                    });
                }
            }) as Arc<dyn Fn() + Send + Sync + 'static>
        };

        *task_cell.lock().unwrap() = Some(task.clone());

        let task_clone_for_initial = Arc::clone(&task);
        $crate::run_task_later!(server, $interval_ticks, {
            task_clone_for_initial();
        });
    }};
}
