pub mod task_later;
pub mod task_timer;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[async_trait::async_trait]
pub trait TaskHandler: Send + Sync + 'static {
    async fn run(&self);
}

pub enum ScheduledTaskType {
    Later {
        run_at: Instant,
    },
    Timer {
        interval: Duration,
        next_run: Instant,
    },
}

pub struct ScheduledTask {
    pub task_type: ScheduledTaskType,
    pub handler: Arc<dyn TaskHandler>,
    pub cancel_flag: Arc<AtomicBool>,
}

pub struct TaskScheduler {
    tasks: Mutex<Vec<ScheduledTask>>,
}

impl TaskScheduler {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(Vec::new()),
        }
    }

    pub fn schedule_once(&self, delay: Duration, handler: Arc<dyn TaskHandler>) {
        self.tasks.lock().unwrap().push(ScheduledTask {
            task_type: ScheduledTaskType::Later {
                run_at: Instant::now() + delay,
            },
            handler,
            cancel_flag: Arc::new(AtomicBool::new(false)),
        });
    }

    pub fn schedule_repeating(
        &self,
        interval: Duration,
        handler: Arc<dyn TaskHandler>,
    ) -> Arc<AtomicBool> {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        self.tasks.lock().unwrap().push(ScheduledTask {
            task_type: ScheduledTaskType::Timer {
                interval,
                next_run: Instant::now() + interval,
            },
            handler,
            cancel_flag: cancel_flag.clone(),
        });
        cancel_flag
    }

    pub fn tick(&self) {
        let now = Instant::now();
        let mut tasks = self.tasks.lock().unwrap();

        tasks.retain_mut(|task| {
            if task.cancel_flag.load(Ordering::Relaxed) {
                return false;
            }

            match &mut task.task_type {
                ScheduledTaskType::Later { run_at } => {
                    if *run_at <= now {
                        let handler = task.handler.clone();
                        tokio::spawn(async move {
                            handler.run().await;
                        });
                        false
                    } else {
                        true
                    }
                }
                ScheduledTaskType::Timer { interval, next_run } => {
                    if *next_run <= now {
                        *next_run = now + *interval;
                        let handler = task.handler.clone();
                        tokio::spawn(async move {
                            handler.run().await;
                        });
                    }
                    true
                }
            }
        });
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}

