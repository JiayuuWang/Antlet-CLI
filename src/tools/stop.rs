use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolResult};
use crate::subagent::{ChildStatus, StopOutcome, SubAgentManager};

pub struct StopTool {
    manager: Arc<SubAgentManager>,
}

impl StopTool {
    pub fn new(manager: Arc<SubAgentManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for StopTool {
    fn name(&self) -> &'static str {
        "stop_agent"
    }

    fn description(&self) -> &'static str {
        "Inspect and stop sub-agents you spawned. \
         To check progress without stopping anyone, pass `list: true` — this \
         returns each sub-agent's id, status (running/completed/failed/stopped) \
         and result if finished. \
         To close a sub-agent, pass its `agent_id` (or `all: true` for every \
         one). With `wait: true` (default) you harvest its full result — if it \
         is still running you block until it reaches a clean stopping point. \
         With `wait: false` you forcefully cancel it. Stopping cascades to all \
         of that sub-agent's descendants and frees its resources. Always stop \
         sub-agents once their work is done."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "list": {
                    "type": "boolean",
                    "description": "If true, just report the status of all sub-agents without stopping any."
                },
                "agent_id": {
                    "type": "string",
                    "description": "The id of the sub-agent to stop (e.g. 'root.1')."
                },
                "all": {
                    "type": "boolean",
                    "description": "If true, stop every sub-agent."
                },
                "wait": {
                    "type": "boolean",
                    "description": "true (default): wait for graceful completion and return the full result. false: forcefully cancel."
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let list = args.get("list").and_then(Value::as_bool).unwrap_or(false);
        let all = args.get("all").and_then(Value::as_bool).unwrap_or(false);
        let wait = args.get("wait").and_then(Value::as_bool).unwrap_or(true);

        // Pure inspection mode.
        if list {
            let snaps = self.manager.list().await;
            if snaps.is_empty() {
                return ToolResult::ok("No sub-agents.".to_string());
            }
            let mut out = format!("{} sub-agent(s):\n", snaps.len());
            for s in snaps {
                let res = match (s.status, &s.result) {
                    (ChildStatus::Completed, Some(r)) | (ChildStatus::Failed, Some(r)) => {
                        format!(" result: {}", truncate(r, 200))
                    }
                    _ => String::new(),
                };
                out.push_str(&format!(
                    "- {} [{}] task: {}{}\n",
                    s.id,
                    s.status.as_str(),
                    truncate(&s.task, 100),
                    res
                ));
            }
            return ToolResult::ok(out);
        }

        if all {
            match self.manager.stop_all(wait).await {
                Ok(outcomes) => {
                    if outcomes.is_empty() {
                        return ToolResult::ok("No sub-agents to stop.".to_string());
                    }
                    ToolResult::ok(format_outcomes(&outcomes))
                }
                Err(e) => ToolResult::err(format!("stop_all failed: {}", e)),
            }
        } else if let Some(id) = args.get("agent_id").and_then(Value::as_str) {
            match self.manager.stop(id, wait).await {
                Ok(outcome) => ToolResult::ok(format_outcomes(&[outcome])),
                Err(e) => ToolResult::err(format!("stop failed: {}", e)),
            }
        } else {
            ToolResult::err("specify `agent_id`, `all: true`, or `list: true`")
        }
    }
}

fn format_outcomes(outcomes: &[StopOutcome]) -> String {
    let mut out = String::new();
    for o in outcomes {
        let mode = if o.forced { "forced" } else { "harvested" };
        out.push_str(&format!(
            "Sub-agent {} stopped ({}, status={}).\n",
            o.id,
            mode,
            o.status.as_str()
        ));
        if let Some(r) = &o.result {
            out.push_str("Result:\n");
            out.push_str(r);
            out.push('\n');
        } else {
            out.push_str("(no result captured)\n");
        }
        out.push_str("---\n");
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "..."
    }
}
