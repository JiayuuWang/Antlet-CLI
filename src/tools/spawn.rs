use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolResult};
use crate::profile::SubProfileInit;
use crate::subagent::{SpawnSpec, SubAgentManager};

pub struct SpawnTool {
    manager: Arc<SubAgentManager>,
}

impl SpawnTool {
    pub fn new(manager: Arc<SubAgentManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for SpawnTool {
    fn name(&self) -> &'static str {
        "spawn_agents"
    }

    fn description(&self) -> &'static str {
        "Spawn one or more sub-agents that run tasks in the BACKGROUND, IN PARALLEL. \
         The size of the `agents` array is the number of sub-agents launched at once.\n\n\
         WHEN TO USE — be proactive: whenever a task can be broken into independent \
         sub-tasks, DECOMPOSE it and spawn one sub-agent per sub-task instead of doing \
         the work yourself sequentially. This is the single biggest lever for speed. \
         Good examples:\n\
         - Translating or summarizing a book → one sub-agent per chapter/section.\n\
         - Reviewing a codebase → one sub-agent per module/directory/concern.\n\
         - Researching a broad topic → one sub-agent per sub-question.\n\
         - Generating a multi-section document → one sub-agent per section.\n\n\
         RECURSE for maximum parallelism: a sub-task that is still large should itself be \
         decomposed. Sub-agents carry these same tools, so a chapter agent can spawn a \
         sub-agent per paragraph, and so on. Prefer a wide, shallow tree of many small \
         agents over a few agents doing large serial work — this maximizes parallel \
         speedup.\n\n\
         Each sub-agent gets its own isolated session and profile, so they never \
         interfere with each other or with you. This call is NON-BLOCKING: it returns \
         the assigned ids immediately while the sub-agents run. Later use `stop_agent` \
         to harvest a finished sub-agent's result (wait=true) or to stop one \
         (wait=false). Always give each sub-agent a short `label` describing its slice \
         (e.g. \"ch1\", \"intro\", \"auth-module\") so its id and output are easy to track."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "agents": {
                    "type": "array",
                    "minItems": 1,
                    "description": "One entry per sub-agent to launch. To parallelize, decompose the task and add one entry per independent slice.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "system_prompt": {
                                "type": "string",
                                "description": "Required. The sub-agent's persona/role and instructions (becomes its persona.md)."
                            },
                            "task": {
                                "type": "string",
                                "description": "Optional. The concrete sub-task to execute. If omitted, the sub-agent works from its system_prompt."
                            },
                            "label": {
                                "type": "string",
                                "description": "Optional but recommended. A short human-meaningful name for this sub-agent's slice (e.g. 'ch1', 'auth-module'). It is appended to the sub-agent's hierarchical id (e.g. 'root.1-ch1') and shown as the prefix on its output, so you can track which sub-agent is which."
                            },
                            "identities": {
                                "type": "string",
                                "description": "Optional initial content for the sub-agent's identities.md."
                            },
                            "self_knowledge": {
                                "type": "string",
                                "description": "Optional initial content for the sub-agent's self_knowledge.md."
                            },
                            "behavior": {
                                "type": "string",
                                "description": "Optional initial content for the sub-agent's behavior.md."
                            }
                        },
                        "required": ["system_prompt"]
                    }
                }
            },
            "required": ["agents"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let arr = match args.get("agents").and_then(Value::as_array) {
            Some(a) if !a.is_empty() => a,
            Some(_) => return ToolResult::err("`agents` array is empty"),
            None => return ToolResult::err("missing `agents` array"),
        };

        let mut specs = Vec::with_capacity(arr.len());
        for (i, item) in arr.iter().enumerate() {
            let system_prompt = match item.get("system_prompt").and_then(Value::as_str) {
                Some(s) if !s.trim().is_empty() => s.to_string(),
                _ => {
                    return ToolResult::err(format!(
                        "agents[{}] missing required non-empty `system_prompt`",
                        i
                    ))
                }
            };
            let task = item
                .get("task")
                .and_then(Value::as_str)
                .map(|s| s.to_string());
            let label = item
                .get("label")
                .and_then(Value::as_str)
                .map(|s| s.to_string());
            let init = SubProfileInit {
                persona: None, // persona is filled from system_prompt by the manager
                identities: item
                    .get("identities")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string()),
                self_knowledge: item
                    .get("self_knowledge")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string()),
                behavior: item
                    .get("behavior")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string()),
            };
            specs.push(SpawnSpec {
                system_prompt,
                task,
                label,
                init,
            });
        }

        match self.manager.spawn(specs).await {
            Ok(spawned) => {
                let lines: Vec<String> = spawned
                    .iter()
                    .map(|(id, task)| format!("- {} (running): {}", id, truncate(task, 120)))
                    .collect();
                ToolResult::ok(format!(
                    "Spawned {} sub-agent(s), running in background:\n{}\n\nUse stop_agent with wait=true to harvest a result when ready, or wait=false to stop forcefully.",
                    spawned.len(),
                    lines.join("\n")
                ))
            }
            Err(e) => ToolResult::err(format!("spawn failed: {}", e)),
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "..."
    }
}
