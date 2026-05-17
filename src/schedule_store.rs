use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Schedule {
    #[serde(rename = "cron")]
    Cron { expression: String },
    #[serde(rename = "once")]
    Once { timestamp: i64 },
}

impl Schedule {
    pub fn cron(expression: &str) -> Self {
        Schedule::Cron {
            expression: expression.to_string(),
        }
    }

    pub fn once(timestamp: i64) -> Self {
        Schedule::Once { timestamp }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub schedule: Schedule,
    pub task: String,
    pub session: String,
    pub workspace: String,
    pub enabled: bool,
    pub created_at: i64,
    pub last_run: Option<i64>,
    pub next_run: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ScheduleFile {
    tasks: Vec<ScheduledTask>,
}

pub struct ScheduleStore {
    path: PathBuf,
}

impl ScheduleStore {
    pub fn new(data_dir: &PathBuf) -> Self {
        let path = data_dir.join("scheduled_tasks.json");
        Self { path }
    }

    pub async fn load(&self) -> Result<Vec<ScheduledTask>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&self.path)
            .await
            .context("failed to read schedule file")?;
        let file: ScheduleFile =
            serde_json::from_str(&content).context("failed to parse schedule file")?;
        Ok(file.tasks)
    }

    pub async fn save(&self, tasks: &[ScheduledTask]) -> Result<()> {
        let file = ScheduleFile {
            tasks: tasks.to_vec(),
        };
        let content = serde_json::to_string_pretty(&file)
            .context("failed to serialize schedule")?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&self.path, content)
            .await
            .context("failed to write schedule file")?;
        Ok(())
    }
}