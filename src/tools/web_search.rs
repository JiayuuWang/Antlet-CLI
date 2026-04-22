use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{Tool, ToolResult};

pub struct WebSearchTool;

impl WebSearchTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Serialize)]
struct TavilyRequest<'a> {
    api_key: &'a str,
    query: &'a str,
    max_results: usize,
}

#[derive(Debug, Deserialize)]
struct TavilyResponse {
    #[serde(default)]
    results: Vec<TavilyItem>,
}

#[derive(Debug, Deserialize)]
struct TavilyItem {
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    content: String,
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "web_search"
    }

    fn description(&self) -> &'static str {
        "Search web via Tavily API."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "max_results": {"type": "integer"}
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let query = match args.get("query").and_then(Value::as_str) {
            Some(v) if !v.is_empty() => v,
            _ => return ToolResult::err("missing query"),
        };
        let max_results = args
            .get("max_results")
            .and_then(Value::as_u64)
            .unwrap_or(5)
            .min(10) as usize;

        let api_key = match std::env::var("TAVILY_API_KEY") {
            Ok(v) if !v.is_empty() => v,
            _ => return ToolResult::err("missing TAVILY_API_KEY"),
        };

        let client = reqwest::Client::new();
        let req = TavilyRequest {
            api_key: &api_key,
            query,
            max_results,
        };

        let res = client
            .post("https://api.tavily.com/search")
            .json(&req)
            .send()
            .await;

        let res = match res {
            Ok(v) => v,
            Err(e) => return ToolResult::err(format!("request failed: {}", e)),
        };

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return ToolResult::err(format!("http {}: {}", status, text));
        }

        let parsed = match res.json::<TavilyResponse>().await {
            Ok(v) => v,
            Err(e) => return ToolResult::err(format!("parse failed: {}", e)),
        };

        if parsed.results.is_empty() {
            return ToolResult::ok("no results");
        }

        let mut out = Vec::new();
        for (idx, item) in parsed.results.iter().enumerate() {
            out.push(format!(
                "{}. {}\nurl: {}\n{}",
                idx + 1,
                item.title,
                item.url,
                item.content
            ));
        }

        ToolResult::ok(out.join("\n\n"))
    }
}
