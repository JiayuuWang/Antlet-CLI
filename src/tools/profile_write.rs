use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::{Tool, ToolResult};

pub struct ProfileTool {
    profile_dir: PathBuf,
}

impl ProfileTool {
    pub fn new(profile_dir: PathBuf) -> Self {
        Self { profile_dir }
    }
}

#[async_trait]
impl Tool for ProfileTool {
    fn name(&self) -> &'static str {
        "write_profile"
    }

    fn description(&self) -> &'static str {
        "Write content to a profile file (identities.md, self_knowledge.md, behavior.md). persona.md is read-only."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file": {"type": "string", "enum": ["identities.md", "self_knowledge.md", "behavior.md"]},
                "content": {"type": "string"}
            },
            "required": ["file", "content"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let file = match args.get("file").and_then(Value::as_str) {
            Some(v) => v,
            None => return ToolResult::err("missing file"),
        };

        if file == "persona.md" {
            return ToolResult::err("persona.md is read-only");
        }

        let content = match args.get("content").and_then(Value::as_str) {
            Some(v) => v,
            None => return ToolResult::err("missing content"),
        };

        let path = self.profile_dir.join(file);

        match tokio::fs::write(&path, content).await {
            Ok(_) => ToolResult::ok(format!("wrote {} ({} bytes)", file, content.len())),
            Err(e) => ToolResult::err(format!("write failed: {}", e)),
        }
    }
}