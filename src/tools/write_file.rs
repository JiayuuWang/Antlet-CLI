use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::{Tool, ToolResult, resolve_path};

pub struct WriteFileTool {
    workspace: PathBuf,
}

impl WriteFileTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Write full content to a UTF-8 text file."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "content": {"type": "string"}
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let path = match args.get("path").and_then(Value::as_str) {
            Some(v) => v,
            None => return ToolResult::err("missing path"),
        };
        let content = match args.get("content").and_then(Value::as_str) {
            Some(v) => v,
            None => return ToolResult::err("missing content"),
        };

        let file = resolve_path(&self.workspace, path);
        if let Some(parent) = file.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                return ToolResult::err(format!("create parent failed: {}", e));
            }
        }

        match tokio::fs::write(&file, content).await {
            Ok(_) => ToolResult::ok(format!("written {}", file.display())),
            Err(e) => ToolResult::err(format!("write failed: {}", e)),
        }
    }
}
