mod bash;
mod glob;
mod grep;
mod ls;
mod profile_write;
mod read;
mod search;
mod write;

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};

pub use bash::BashTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use ls::LsTool;
pub use profile_write::ProfileTool;
pub use read::ReadTool;
pub use search::SearchTool;
pub use write::WriteTool;

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub success: bool,
    pub content: String,
    pub error: Option<String>,
}

impl ToolResult {
    pub fn ok(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: content.into(),
            error: None,
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            content: String::new(),
            error: Some(error.into()),
        }
    }

    pub fn as_text(&self) -> String {
        if self.success {
            self.content.clone()
        } else {
            format!(
                "error: {}",
                self.error.clone().unwrap_or_else(|| "unknown".to_string())
            )
        }
    }
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters(&self) -> Value;
    async fn execute(&self, args: Value) -> ToolResult;

    fn to_openai_schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": self.description(),
                "parameters": self.parameters(),
            }
        })
    }
}

#[derive(Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    schemas: Arc<Vec<Value>>,
}

impl ToolRegistry {
    pub fn default_for(workspace: PathBuf, config_dir: PathBuf) -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
            schemas: Arc::new(Vec::new()),
        };
        registry.register(ReadTool::new(workspace.clone()));
        registry.register(WriteTool::new(workspace.clone()));
        registry.register(GrepTool::new(workspace.clone()));
        registry.register(GlobTool::new(workspace.clone()));
        registry.register(LsTool::new(workspace.clone()));
        registry.register(BashTool::new(workspace.clone()));
        registry.register(SearchTool::new(config_dir));
        registry
    }

    pub fn with_profile(workspace: PathBuf, profile_dir: PathBuf) -> Self {
        let config_dir = profile_dir.parent().unwrap_or(&profile_dir).to_path_buf();
        let mut registry = Self::default_for(workspace, config_dir);
        registry.register(ProfileTool::new(profile_dir));
        registry
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        let schema = tool.to_openai_schema();
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
        let mut new_schemas = (*self.schemas).clone();
        new_schemas.push(schema);
        self.schemas = Arc::new(new_schemas);
    }

    pub fn schemas(&self) -> Arc<Vec<Value>> {
        self.schemas.clone()
    }

    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.tools.keys().cloned().collect();
        names.sort();
        names
    }

    pub async fn execute(&self, name: &str, args: Value) -> Result<ToolResult> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| anyhow!("unknown tool: {}", name))?;
        Ok(tool.execute(args).await)
    }
}

pub fn resolve_path(workspace: &PathBuf, path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        workspace.join(p)
    }
}