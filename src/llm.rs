use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::schema::{FunctionCall, Message, ToolCall};

#[derive(Debug, Clone)]
pub struct LlmClient {
    http: reqwest::Client,
    api_key: String,
    api_base: String,
    model: String,
}

#[derive(Debug, Clone)]
pub struct LlmReply {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ApiMessage,
}

#[derive(Debug, Deserialize)]
struct ApiMessage {
    #[serde(default)]
    content: String,
    #[serde(default)]
    tool_calls: Vec<ApiToolCall>,
}

#[derive(Debug, Deserialize)]
struct ApiToolCall {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    function: ApiFunction,
}

#[derive(Debug, Deserialize)]
struct ApiFunction {
    name: String,
    arguments: String,
}

impl LlmClient {
    pub fn new(api_key: String, api_base: String, model: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key,
            api_base: api_base.trim_end_matches('/').to_string(),
            model,
        }
    }

    pub async fn generate(&self, messages: &[Message], tools: &[Value]) -> Result<LlmReply> {
        let url = format!("{}/chat/completions", self.api_base);
        let req = ChatRequest {
            model: self.model.clone(),
            messages: convert_messages(messages)?,
            tools: if tools.is_empty() {
                None
            } else {
                Some(tools.to_vec())
            },
        };

        let resp = self
            .http
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("llm request failed: {} {}", status, body));
        }

        let parsed = resp.json::<ChatResponse>().await?;
        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("empty choices from llm"))?;

        let tool_calls = choice
            .message
            .tool_calls
            .into_iter()
            .map(|c| {
                let args: Value =
                    serde_json::from_str(&c.function.arguments).unwrap_or_else(|_| json!({}));
                ToolCall {
                    id: c.id,
                    kind: c.kind,
                    function: FunctionCall {
                        name: c.function.name,
                        arguments: args,
                    },
                }
            })
            .collect();

        Ok(LlmReply {
            content: choice.message.content,
            tool_calls,
        })
    }
}

fn convert_messages(messages: &[Message]) -> Result<Vec<Value>> {
    let mut out = Vec::with_capacity(messages.len());

    for m in messages {
        match m.role.as_str() {
            "system" | "user" => {
                out.push(json!({"role": m.role, "content": m.content}));
            }
            "assistant" => {
                let mut obj = json!({"role": "assistant", "content": m.content});
                if let Some(calls) = &m.tool_calls {
                    let tc: Vec<Value> = calls
                        .iter()
                        .map(|c| {
                            json!({
                                "id": c.id,
                                "type": "function",
                                "function": {
                                    "name": c.function.name,
                                    "arguments": c.function.arguments.to_string()
                                }
                            })
                        })
                        .collect();
                    obj["tool_calls"] = Value::Array(tc);
                }
                out.push(obj);
            }
            "tool" => {
                let tool_call_id = m
                    .tool_call_id
                    .clone()
                    .ok_or_else(|| anyhow!("tool message missing tool_call_id"))?;
                out.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_call_id,
                    "content": m.content,
                }));
            }
            other => return Err(anyhow!("unsupported role: {}", other)),
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::schema::{FunctionCall, Message, ToolCall};

    use super::convert_messages;

    #[test]
    fn convert_assistant_tool_calls() {
        let msg = Message::assistant(
            "",
            Some(vec![ToolCall {
                id: "call_1".to_string(),
                kind: "function".to_string(),
                function: FunctionCall {
                    name: "read_file".to_string(),
                    arguments: json!({"path": "src/main.rs"}),
                },
            }]),
        );
        let out = convert_messages(&[msg]).unwrap();
        assert_eq!(out[0]["role"], "assistant");
        assert_eq!(out[0]["tool_calls"][0]["function"]["name"], "read_file");
    }
}
