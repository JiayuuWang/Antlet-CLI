use std::path::PathBuf;
use std::process::Command;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::Tool;
use super::ToolResult;

pub struct GrepTool {
    workspace: PathBuf,
}

impl GrepTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Search for pattern in files using grep. Returns matching lines with line numbers."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string", "description": "Regex pattern to search"},
                "path": {"type": "string", "description": "Directory or file to search in (default: workspace)"},
                "recursive": {"type": "boolean", "description": "Search subdirectories (default: true)"},
                "ignore_case": {"type": "boolean", "description": "Case insensitive search (default: false)"}
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let pattern = match args.get("pattern").and_then(Value::as_str) {
            Some(v) if !v.is_empty() => v,
            _ => return ToolResult::err("missing pattern"),
        };

        let path = args
            .get("path")
            .and_then(Value::as_str)
            .map(|p| resolve_path(&self.workspace, p))
            .unwrap_or_else(|| self.workspace.clone());

        let recursive = args.get("recursive").and_then(Value::as_bool).unwrap_or(true);
        let ignore_case = args.get("ignore_case").and_then(Value::as_bool).unwrap_or(false);

        let mut cmd = Command::new("grep");
        if ignore_case {
            cmd.arg("-i");
        }
        cmd.arg("-n"); // line numbers

        if recursive && path.is_dir() {
            cmd.arg("-r");
        }

        cmd.arg("--");
        cmd.arg(pattern);

        if path.is_dir() {
            cmd.arg(path.to_str().unwrap_or("."));
        } else {
            cmd.arg(&path);
        }

        let output = cmd.output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

                if stdout.is_empty() && !stderr.is_empty() {
                    ToolResult::ok(format!("(no matches)"))
                } else {
                    ToolResult::ok(stdout)
                }
            }
            Err(e) => ToolResult::err(format!("grep failed: {}", e)),
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