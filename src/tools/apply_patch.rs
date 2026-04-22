use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::{Tool, ToolResult, resolve_path};

pub struct ApplyPatchTool {
    workspace: PathBuf,
}

impl ApplyPatchTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for ApplyPatchTool {
    fn name(&self) -> &'static str {
        "apply_patch"
    }

    fn description(&self) -> &'static str {
        "Patch a file by replacing text."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "old": {"type": "string"},
                "new": {"type": "string"},
                "replace_all": {"type": "boolean"}
            },
            "required": ["path", "old", "new"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let path = match args.get("path").and_then(Value::as_str) {
            Some(v) => v,
            None => return ToolResult::err("missing path"),
        };
        let old = match args.get("old").and_then(Value::as_str) {
            Some(v) if !v.is_empty() => v,
            _ => return ToolResult::err("missing old"),
        };
        let new = match args.get("new").and_then(Value::as_str) {
            Some(v) => v,
            None => return ToolResult::err("missing new"),
        };
        let replace_all = args
            .get("replace_all")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let file = resolve_path(&self.workspace, path);
        let content = match tokio::fs::read_to_string(&file).await {
            Ok(v) => v,
            Err(e) => return ToolResult::err(format!("read failed: {}", e)),
        };

        if !content.contains(old) {
            return ToolResult::err("target text not found");
        }

        let patched = if replace_all {
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
