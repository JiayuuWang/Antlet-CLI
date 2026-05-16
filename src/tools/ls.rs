use std::path::PathBuf;
use std::process::Command;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::Tool;
use super::ToolResult;

pub struct LsTool {
    workspace: PathBuf,
}

impl LsTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for LsTool {
    fn name(&self) -> &'static str {
        "ls"
    }

    fn description(&self) -> &'static str {
        "List directory contents with file sizes and modification times."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Directory to list (default: workspace root)"}
            },
            "required": []
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .map(|p| resolve_path(&self.workspace, p))
            .unwrap_or_else(|| self.workspace.clone());

        let mut cmd = Command::new("ls");
        cmd.arg("-la");
        cmd.arg(path.to_str().unwrap_or("."));

        let output = cmd.output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

                if !stdout.is_empty() {
                    ToolResult::ok(stdout)
                } else if !stderr.is_empty() {
                    ToolResult::err(stderr)
                } else {
                    ToolResult::ok("(empty)".to_string())
                }
            }
            Err(e) => ToolResult::err(format!("ls failed: {}", e)),
        }
    }
}

fn resolve_path(workspace: &PathBuf, path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        workspace.join(p)
    }
}