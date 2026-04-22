use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::{
    process::Command,
    time::{Duration, timeout},
};

use super::Tool;
use super::ToolResult;

pub struct BashTool {
    workspace: PathBuf,
}

impl BashTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn description(&self) -> &'static str {
        "Run shell command in workspace."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {"type": "string"},
                "timeout_sec": {"type": "integer"}
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let command = match args.get("command").and_then(Value::as_str) {
            Some(v) => v,
            None => return ToolResult::err("missing command"),
        };

        let timeout_sec = args
            .get("timeout_sec")
            .and_then(Value::as_u64)
            .unwrap_or(120)
            .min(600);

        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("powershell");
            c.arg("-Command").arg(command);
            c
        } else {
            let mut c = Command::new("bash");
            c.arg("-lc").arg(command);
            c
        };

        cmd.current_dir(&self.workspace);

        let run = cmd.output();
        let output = match timeout(Duration::from_secs(timeout_sec), run).await {
            Ok(v) => match v {
                Ok(ok) => ok,
                Err(e) => return ToolResult::err(format!("command failed: {}", e)),
            },
            Err(_) => return ToolResult::err("command timeout"),
        };

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        let mut out = String::new();
        if !stdout.is_empty() {
            out.push_str(&format!("stdout:\n{}\n", stdout));
        }
        if !stderr.is_empty() {
            out.push_str(&format!("stderr:\n{}\n", stderr));
        }
        out.push_str(&format!(
            "exit_code: {}",
            output.status.code().unwrap_or(-1)
        ));

        if output.status.success() {
            ToolResult::ok(out)
        } else {
            ToolResult::err(out)
        }
    }
}
