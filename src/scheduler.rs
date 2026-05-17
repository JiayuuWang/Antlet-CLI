use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use chrono::{Timelike, Utc};
use tokio::sync::Mutex;
use tokio::time::{interval, Duration, Instant};

use crate::agent::Agent;
use crate::schedule_store::{Schedule, ScheduledTask, ScheduleStore};

pub struct Scheduler {
    store: ScheduleStore,
}

impl Scheduler {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            store: ScheduleStore::new(&data_dir),
        }
    }

    pub async fn load_tasks(&self) -> Result<Vec<ScheduledTask>> {
        self.store.load().await
    }

    #[allow(dead_code)]
    pub async fn add_task(&self, task: ScheduledTask) -> Result<()> {
        let mut tasks = self.store.load().await.unwrap_or_default();
        tasks.push(task);
        self.store.save(&tasks).await
    }

    #[allow(dead_code)]
    pub async fn save_tasks(&self, tasks: &[ScheduledTask]) -> Result<()> {
        self.store.save(tasks).await
    }

    #[allow(dead_code)]
    pub async fn remove_task(&self, id: &str) -> Result<()> {
        let mut tasks = self.store.load().await.unwrap_or_default();
        tasks.retain(|t| t.id != id);
        self.store.save(&tasks).await
    }

    #[allow(dead_code)]
    pub async fn list_tasks(&self) -> Result<Vec<ScheduledTask>> {
        self.store.load().await
    }

    #[allow(dead_code)]
    pub async fn add_from_cli(
        &self,
        schedule: &str,
        name: &str,
        task_text: &str,
        session: &str,
        workspace: &str,
    ) -> Result<ScheduledTask> {
        let now = Utc::now().timestamp();

        // Parse schedule: if it looks like a timestamp, it's "once"; otherwise treat as cron
        let schedule = if let Ok(ts) = schedule.parse::<i64>() {
            Schedule::once(ts)
        } else {
            Schedule::cron(schedule)
        };

        let next_run = compute_next_run(&schedule, now);

        let task = ScheduledTask {
            id: generate_id(),
            name: name.to_string(),
            schedule,
            task: task_text.to_string(),
            session: session.to_string(),
            workspace: workspace.to_string(),
            enabled: true,
            created_at: now,
            last_run: None,
            next_run,
        };

        self.store.save(&[task.clone()]).await?;
        Ok(task)
    }

    /// Run the scheduler loop. agent must be wrapped in Arc<Mutex<_>>
    pub async fn run(self, agent: Arc<Mutex<Box<Agent>>>) -> Result<()> {
        loop {
            let tasks = match self.store.load().await {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("scheduler: failed to load tasks: {}", e);
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    continue;
                }
            };

            // Find the next soonest task
            let now_ts = Utc::now().timestamp();
            let next_task = tasks
                .iter()
                .filter(|t| t.enabled && t.next_run.is_some())
                .min_by_key(|t| t.next_run.unwrap())
                .cloned();

            if let Some(next) = next_task {
                let wait_secs = next.next_run.unwrap().saturating_sub(now_ts);
                if wait_secs > 0 {
                    // Wait until next task, checking every minute for new tasks
                    let deadline = Instant::now() + Duration::from_secs(wait_secs.max(0) as u64);
                    let mut ticker = interval(Duration::from_secs(60));

                    while Instant::now() < deadline {
                        ticker.tick().await;
                        // Check if new task has earlier time
                        let fresh = self.store.load().await.ok();
                        if let Some(tasks) = fresh {
                            if let Some(earlier) = tasks
                                .iter()
                                .filter(|t| t.enabled && t.next_run.is_some())
                                .min_by_key(|t| t.next_run.unwrap())
                            {
                                let elapsed = deadline.saturating_duration_since(Instant::now()).as_secs() as i64;
                                if earlier.next_run.unwrap() < now_ts + elapsed {
                                    break; // found earlier task, re-evaluate
                                }
                            }
                        }
                    }
                }

                // Fire the task
                if let Err(e) = self.fire_task(&next, agent.clone()).await {
                    eprintln!("scheduler: task {} failed: {}", next.id, e);
                }
            } else {
                // No tasks, sleep and recheck
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        }
    }

    async fn fire_task(&self, task: &ScheduledTask, agent: Arc<Mutex<Box<Agent>>>) -> Result<()> {
        eprintln!(
            "scheduler: firing task '{}' (id={})",
            task.name, task.id
        );

        let result: Result<String, anyhow::Error> = {
            let mut ag = agent.lock().await;
            ag.run_task(task.task.as_str()).await
        };

        match result {
            Ok(response) => {
                eprintln!(
                    "scheduler: task '{}' completed: {}",
                    task.name,
                    truncate(&response, 200)
                );
            }
            Err(e) => {
                eprintln!("scheduler: task '{}' error: {}", task.name, e);
            }
        }

        // Update task: set last_run, compute next_run for recurring
        let mut tasks = self.store.load().await?;
        if let Some(t) = tasks.iter_mut().find(|t| t.id == task.id) {
            t.last_run = Some(Utc::now().timestamp());

            // Recompute next_run for cron schedules
            if let Schedule::Cron { expression: _ } = &t.schedule {
                let now = Utc::now().timestamp();
                t.next_run = compute_next_run(&t.schedule, now);
            } else {
                // One-shot: disable after firing
                t.enabled = false;
                t.next_run = None;
            }
        }

        self.store.save(&tasks).await?;
        Ok(())
    }
}

fn compute_next_run(schedule: &Schedule, from_ts: i64) -> Option<i64> {
    match schedule {
        Schedule::Cron { expression } => cron_next_or_now(expression, from_ts),
        Schedule::Once { timestamp } => {
            if *timestamp > from_ts {
                Some(*timestamp)
            } else {
                None // already fired
            }
        }
    }
}

fn cron_next_or_now(cron_expr: &str, from_ts: i64) -> Option<i64> {
    let parts: Vec<&str> = cron_expr.trim().split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }

    let now = chrono::DateTime::from_timestamp(from_ts, 0)?;
    let local = now.with_timezone(&chrono::Local);

    let minute_field = parts[0];
    let hour_field = parts[1];

    // For */n patterns, parse step; for * or exact value, store as Option
    let minute_step = if minute_field.starts_with("*/") {
        minute_field[2..].parse::<u32>().ok().filter(|&s| s > 0 && s <= 59)
    } else {
        None
    };
    let minute_target = if minute_step.is_none() {
        parse_cron_field(minute_field, 0, 59)? as u32
    } else {
        0 // not used when step is set
    };

    let hour_step = if hour_field.starts_with("*/") {
        hour_field[2..].parse::<u32>().ok().filter(|&s| s > 0 && s <= 23)
    } else {
        None
    };
    let hour_target = if hour_step.is_none() {
        parse_cron_field(hour_field, 0, 23)? as u32
    } else {
        0 // not used when step is set
    };

    // Try next occurrences within next 1440 minutes (1 day)
    for offset in 1..=1440 {
        let candidate = local + chrono::Duration::minutes(offset);

        // Check hour match
        let hour_match = if let Some(step) = hour_step {
            candidate.hour() % step == 0
        } else {
            candidate.hour() == hour_target
        };

        // Check minute match
        let min_match = if let Some(step) = minute_step {
            candidate.minute() % step == 0
        } else {
            candidate.minute() == minute_target
        };

        if hour_match && min_match {
            let ts = candidate
                .with_second(0)
                .and_then(|d| d.with_nanosecond(0))?
                .timestamp();
            if ts > from_ts {
                return Some(ts);
            }
        }
    }

    None
}

fn parse_cron_field(field: &str, min: u64, max: u64) -> Option<u64> {
    if field == "*" {
        return Some(min);
    }
    // Handle */n patterns like */10
    if let Some((_, step_str)) = field.split_once('/') {
        if let Ok(_step) = step_str.parse::<u64>() {
            // For */n, return the min value (actual matching done by iteration)
            return Some(min);
        }
        return None;
    }
    field.parse::<u64>().ok().filter(|v| *v >= min && *v <= max)
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("task_{:x}", now)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "..."
    }
}