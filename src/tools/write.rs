use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::{Tool, ToolResult, resolve_path};

pub struct WriteTool {
    workspace: PathBuf,
}

impl WriteTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &'static str {
        "write"
    }

    fn description(&self) -> &'static str {
        "Write or edit a file. Use `old`/`new` for text replacement, or `content` to overwrite entire file. Creates file if it doesn't exist."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "content": {"type": "string"},
                "old": {"type": "string"},
                "new": {"type": "string"},
                "replace_all": {"type": "boolean"}
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let path = match args.get("path").and_then(Value::as_str) {
            Some(v) => v,
            None => return ToolResult::err("missing path"),
        };

        let file = resolve_path(&self.workspace, path);

        // Full overwrite mode: write entire content
        if let Some(content) = args.get("content").and_then(Value::as_str) {
            // Ensure parent directory exists
            if let Some(parent) = file.parent() {
                if !parent.exists() {
                    if let Err(e) = tokio::fs::create_dir_all(parent).await {
                        return ToolResult::err(format!("create dir failed: {}", e));
                    }
                }
            }
            match tokio::fs::write(&file, content).await {
                Ok(_) => ToolResult::ok(format!("wrote {} ({} bytes)", file.display(), content.len())),
                Err(e) => ToolResult::err(format!("write failed: {}", e)),
            }
        } else {
            // Patch mode: replace text
            let old = match args.get("old").and_then(Value::as_str) {
                Some(v) if !v.is_empty() => v,
                _ => return ToolResult::err("missing `old` or `content`"),
            };
            let new = match args.get("new").and_then(Value::as_str) {
                Some(v) => v,
                None => return ToolResult::err("missing `new`"),
            };

            let content = match tokio::fs::read_to_string(&file).await {
                Ok(v) => v,
                Err(e) => return ToolResult::err(format!("read failed: {}", e)),
            };

            if !content.contains(old) {
                return ToolResult::err("target text not found");
            }

            let patched = if args.get("replace_all").and_then(Value::as_bool).unwrap_or(false) {
                content.replace(old, new)
            } else {
                content.replacen(old, new, 1)
            };

            match tokio::fs::write(&file, patched).await {
                Ok(_) => ToolResult::ok(format!("patched {}", file.display())),
                Err(e) => ToolResult::err(format!("write failed: {}", e)),
            }
        }
    }
}