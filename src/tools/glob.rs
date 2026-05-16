use std::path::PathBuf;
use std::process::Command;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::Tool;
use super::ToolResult;

pub struct GlobTool {
    workspace: PathBuf,
}

impl GlobTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &'static str {
        "glob"
    }

    fn description(&self) -> &'static str {
        "Find files by name pattern (glob). Supports ** for matching any directories."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string", "description": "Glob pattern, e.g. **/*.rs or src/**/*.go"},
                "path": {"type": "string", "description": "Directory to search in (default: workspace)"}
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

        // Use find instead of glob since rust's glob crate is sync
        let mut cmd = Command::new("find");
        cmd.arg(path.to_str().unwrap_or("."));

        // Convert glob pattern to find predicate
        // **/*.rs -> -name "*.rs"
        // *.txt -> -name "*.txt"
        let name_pattern = pattern
            .split('/')
            .last()
            .unwrap_or(pattern);

        if name_pattern.contains("**") {
            // For ** patterns, use -name with wildcard
            let name = name_pattern.replace("**", "*");
            cmd.arg("-name").arg(&name);
        } else {
            cmd.arg("-name").arg(name_pattern);
        }

        cmd.arg("-type").arg("f");

        let output = cmd.output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if stdout.is_empty() {
                    ToolResult::ok("(no matches)".to_string())
                } else {
                    // Convert to relative paths for readability
                    let lines: Vec<&str> = stdout.lines().collect();
                    ToolResult::ok(lines.join("\n"))
                }
            }
            Err(e) => ToolResult::err(format!("glob failed: {}", e)),
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