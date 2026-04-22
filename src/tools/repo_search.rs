use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::process::Command;

use super::{Tool, ToolResult, resolve_path};

pub struct RepoSearchTool {
    workspace: PathBuf,
}

impl RepoSearchTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for RepoSearchTool {
    fn name(&self) -> &'static str {
        "repo_search"
    }

    fn description(&self) -> &'static str {
        "Search repository text with ripgrep."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string"},
                "path": {"type": "string"},
                "max_results": {"type": "integer"}
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let pattern = match args.get("pattern").and_then(Value::as_str) {
            Some(v) if !v.is_empty() => v,
            _ => return ToolResult::err("missing pattern"),
        };
        let max_results = args
            .get("max_results")
            .and_then(Value::as_u64)
            .unwrap_or(80) as usize;
        let path = args.get("path").and_then(Value::as_str).unwrap_or(".");

        let target = resolve_path(&self.workspace, path);
        let output = Command::new("rg")
            .arg("--line-number")
            .arg("--with-filename")
            .arg("--color")
            .arg("never")
            .arg(pattern)
            .arg(target)
            .output()
            .await;

        let output = match output {
            Ok(v) => v,
            Err(e) => return ToolResult::err(format!("run rg failed: {}", e)),
        };

        if !output.status.success() && output.stdout.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return ToolResult::err(format!("rg failed: {}", stderr.trim()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().take(max_results).collect();
        if lines.is_empty() {
            ToolResult::ok("no matches")
        } else {
            ToolResult::ok(lines.join("\n"))
        }
    }
}
