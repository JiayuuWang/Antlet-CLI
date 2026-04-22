use anyhow::Result;

use crate::{
    llm::LlmClient, schema::Message, session_store::SessionStore, tools::ToolRegistry, ui::Color,
};

pub struct Agent {
    llm: LlmClient,
    tools: ToolRegistry,
    messages: Vec<Message>,
    session: SessionStore,
    max_steps: usize,
}

impl Agent {
    pub async fn new(
        llm: LlmClient,
        tools: ToolRegistry,
        session: SessionStore,
        system_prompt: String,
        max_steps: usize,
    ) -> Result<Self> {
        let mut messages = session.load().await?;
        let system = Message::system(system_prompt);

        if messages.is_empty() {
            messages = vec![system.clone()];
        } else if messages[0].role == "system" {
            messages[0] = system.clone();
        } else {
            messages.insert(0, system.clone());
        }

        session.rewrite(&messages).await?;

        Ok(Self {
            llm,
            tools,
            messages,
            session,
            max_steps,
        })
    }

    pub fn clear_history_keep_system(&mut self) {
        if let Some(system) = self.messages.first().cloned() {
            self.messages = vec![system];
        }
    }

    pub async fn persist_all(&self) -> Result<()> {
        self.session.rewrite(&self.messages).await
    }

    pub fn history_len(&self) -> usize {
        self.messages.len()
    }

    pub async fn run_task(&mut self, input: &str) -> Result<String> {
        let user = Message::user(input.to_string());
        self.messages.push(user.clone());
        self.session.append(&user).await?;

        for step in 0..self.max_steps {
            println!(
                "{}[step {}/{}] thinking{}",
                Color::CYAN,
                step + 1,
                self.max_steps,
                Color::RESET
            );

            let reply = self
                .llm
                .generate(&self.messages, &self.tools.schemas())
                .await?;

            let assistant =
                Message::assistant(reply.content.clone(), Some(reply.tool_calls.clone()));
            self.messages.push(assistant.clone());
            self.session.append(&assistant).await?;

            if !reply.content.is_empty() {
                println!(
                    "{}assistant>{} {}",
                    Color::BLUE,
                    Color::RESET,
                    summarize(&reply.content)
                );
            }

            if reply.tool_calls.is_empty() {
                return Ok(reply.content);
            }

            for call in reply.tool_calls {
                let tool_name = call.function.name.clone();
                let args = call.function.arguments.clone();
                let args_str =
                    serde_json::to_string_pretty(&args).unwrap_or_else(|_| "{}".to_string());

                println!(
                    "{}tool>{} {}{}{}",
                    Color::MAGENTA,
                    Color::RESET,
                    Color::BOLD,
                    tool_name,
                    Color::RESET
                );
                println!("{}tool.args>{}\n{}", Color::DIM, Color::RESET, args_str);

                let result = self.tools.execute(&tool_name, args).await?;
                let text = result.as_text();
                let tool_msg = Message::tool(call.id, tool_name, text.clone());
                self.messages.push(tool_msg.clone());
                self.session.append(&tool_msg).await?;

                if result.success {
                    println!(
                        "{}tool.ok>{} {}",
                        Color::GREEN,
                        Color::RESET,
                        summarize(&text)
                    );
                } else {
                    println!(
                        "{}tool.err>{} {}",
                        Color::RED,
                        Color::RESET,
                        summarize(&text)
                    );
                }
            }
        }

        Ok(format!(
            "Task couldn't be completed after {} steps.",
            self.max_steps
        ))
    }
}

fn summarize(s: &str) -> String {
    const MAX: usize = 220;
    if s.chars().count() <= MAX {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(MAX).collect();
        out.push_str("...");
        out
    }
}
