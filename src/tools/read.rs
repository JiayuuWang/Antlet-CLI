use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::{Tool, ToolResult, resolve_path};

pub struct ReadTool {
    workspace: PathBuf,
}

impl ReadTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &'static str {
        "read"
    }

    fn description(&self) -> &'static str {
        "Read a UTF-8 text file with line numbers."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "offset": {"type": "integer"},
                "limit": {"type": "integer"}
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let path = match args.get("path").and_then(Value::as_str) {
            Some(v) => v,
            None => return ToolResult::err("missing path"),
        };
        let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(1) as usize;
        let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(500) as usize;

        let file = resolve_path(&self.workspace, path);
        let content = match tokio::fs::read_to_string(&file).await {
            Ok(v) => v,
            Err(e) => return ToolResult::err(format!("read failed: {}", e)),
        };

        let mut lines = Vec::new();
        for (i, line) in content
            .lines()
            .enumerate()
            .skip(offset.saturating_sub(1))
            .take(limit)
        {
            lines.push(format!("{:>6}|{}", i + 1, line));
        }

        if lines.is_empty() {
            return ToolResult::ok("(no content)");
        }

        ToolResult::ok(lines.join("\n"))
    }
}