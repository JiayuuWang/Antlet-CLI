use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use futures::future::join_all;

use crate::{
    llm::LlmClient, profile::ProfileFiles, schema::Message, session_store::SessionStore,
    tools::ToolRegistry, ui::Color,
};

const MEMORY_SUMMARY_INTERVAL: usize = 20;

pub struct Agent {
    llm: LlmClient,
    tools: ToolRegistry,
    messages: Vec<Message>,
    session: SessionStore,
    max_steps: usize,
    step_count: usize,
    profile_files: ProfileFiles,
    /// Cooperative cancellation flag. When set, `run_task` exits gracefully at
    /// the next step boundary, leaving session/profile files in a clean state.
    cancel: Option<Arc<AtomicBool>>,
    /// Short identity label shown at the start of every output line so the user
    /// can tell which agent is speaking (e.g. the agent-id / session name).
    label: String,
}

impl Agent {
    pub async fn new(
        llm: LlmClient,
        tools: ToolRegistry,
        session: SessionStore,
        system_prompt: String,
        max_steps: usize,
        profile_files: ProfileFiles,
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
            step_count: 0,
            profile_files,
            cancel: None,
            label: "main".to_string(),
        })
    }

    /// Set the identity label shown as a prefix on this agent's output lines.
    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }

    /// Format this agent's output prefix, e.g. `[main]` or `[root.1-ch1]`.
    fn tag(&self) -> String {
        format!("{}[{}]{} ", Color::BOLD, self.label, Color::RESET)
    }

    /// Attach a cooperative cancellation flag (used by sub-agents so the parent
    /// can stop them between steps without leaving partial writes).
    pub fn set_cancel(&mut self, cancel: Arc<AtomicBool>) {
        self.cancel = Some(cancel);
    }

    pub fn clear_history_keep_system(&mut self) {
        if let Some(system) = self.messages.first().cloned() {
            self.messages = vec![system];
        }
        self.step_count = 0;
    }

    pub async fn persist_all(&self) -> Result<()> {
        self.session.rewrite(&self.messages).await
    }

    pub fn history_len(&self) -> usize {
        self.messages.len()
    }

    pub fn get_first_response(&self) -> Option<String> {
        self.messages
            .iter()
            .find(|m| m.role == "assistant" && !m.content.is_empty())
            .map(|m| m.content.clone())
    }

    pub fn replace_messages(&mut self, messages: Vec<Message>) {
        self.messages = messages;
    }

    /// Point this agent at a new session store and profile files after its
    /// on-disk agent directory has been renamed (root rename on summary).
    pub fn rebind_storage(&mut self, session: SessionStore, profile_files: ProfileFiles) {
        self.session = session;
        self.profile_files = profile_files;
    }

    pub fn profile_dir(&self) -> PathBuf {
        self.profile_files.persona.parent().unwrap().to_path_buf()
    }

    pub async fn run_task(&mut self, input: &str) -> Result<String> {
        let user = Message::user(input.to_string());
        self.messages.push(user.clone());
        self.session.append(&user).await?;

        for step in 0..self.max_steps {
            if self.is_cancelled() {
                let msg = format!("Task stopped by parent at step {}.", step);
                println!("{}{}cancelled{}: {}", self.tag(), Color::YELLOW, Color::RESET, msg);
                return Ok(msg);
            }

            self.step_count += 1;
            let tag = self.tag();

            println!(
                "{}{}[step {}/{}] thinking{}",
                tag,
                Color::DIM,
                step + 1,
                self.max_steps,
                Color::RESET
            );

            let reply = self
                .llm
                .generate(&self.messages, &self.tools.schemas())
                .await?;

            let assistant = Message::assistant(reply.content.clone(), Some(reply.tool_calls.clone()));
            self.messages.push(assistant.clone());
            self.session.append(&assistant).await?;

            if reply.tool_calls.is_empty() {
                if self.step_count % MEMORY_SUMMARY_INTERVAL == 0 {
                    self.summarize_and_update_behavior().await?;
                }
                return Ok(reply.content);
            }

            if !reply.content.is_empty() {
                println!(
                    "{}{}assistant>{} {}",
                    tag,
                    Color::BLUE,
                    Color::RESET,
                    summarize(&reply.content)
                );
            }

            let tool_futures: Vec<_> = reply.tool_calls.iter().map(|call| {
                let tool_name = call.function.name.clone();
                let args = call.function.arguments.clone();
                let call_id = call.id.clone();
                let tools = self.tools.clone();
                async move {
                    let args_str = serde_json::to_string_pretty(&args).unwrap_or_else(|_| "{}".to_string());
                    let result = tools.execute(&tool_name, args).await;
                    (call_id, tool_name, args_str, result)
                }
            }).collect();

            let results = join_all(tool_futures).await;

            for (call_id, tool_name, args_str, result) in results {
                let text = match &result {
                    Ok(r) => r.as_text(),
                    Err(e) => format!("error: {}", e),
                };

                println!(
                    "{}{}tool>{} {}{}{}",
                    tag,
                    Color::MAGENTA,
                    Color::RESET,
                    Color::BOLD,
                    tool_name,
                    Color::RESET
                );
                println!("{}{}tool.args>{}\n{}", tag, Color::DIM, Color::RESET, args_str);

                let success = result.as_ref().map(|r| r.success).unwrap_or(false);
                let tool_msg = Message::tool(call_id, tool_name.clone(), text.clone());
                self.messages.push(tool_msg.clone());
                self.session.append(&tool_msg).await?;

                if success {
                    println!(
                        "{}{}tool.ok>{} {}",
                        tag,
                        Color::GREEN,
                        Color::RESET,
                        summarize(&text)
                    );
                } else {
                    println!(
                        "{}{}tool.err>{} {}",
                        tag,
                        Color::RED,
                        Color::RESET,
                        summarize(&text)
                    );
                }
            }

            if self.step_count % MEMORY_SUMMARY_INTERVAL == 0 {
                self.summarize_and_update_behavior().await?;
            }
        }

        Ok(format!(
            "Task couldn't be completed after {} steps.",
            self.max_steps
        ))
    }

    async fn summarize_and_update_behavior(&mut self) -> Result<()> {
        let context = self.build_context_summary();
        let summary = self.generate_summary(&context).await?;

        let behavior_path = self.profile_files.behavior.clone();
        let existing = tokio::fs::read_to_string(&behavior_path).await.unwrap_or_default();
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

        let new_entry = format!(
            "\n\n## Memory Entry [{}]\n{}\n---\n",
            timestamp, summary
        );

        let updated = existing + &new_entry;

        tokio::fs::write(&behavior_path, &updated).await?;

        let memory_block = format!(
            "[Memory update at step {}]: {}",
            self.step_count, summary
        );

        let mem_msg = Message::user(memory_block);
        self.messages.push(mem_msg.clone());
        self.session.append(&mem_msg).await?;

        println!(
            "{}{}memory{}: context summarized and stored at step {}",
            self.tag(),
            Color::YELLOW,
            Color::RESET,
            self.step_count
        );

        Ok(())
    }

    fn build_context_summary(&self) -> String {
        let mut context = String::new();
        context.push_str("Recent conversation:\n");

        let recent = self.messages.iter().rev().take(40).collect::<Vec<_>>();
        for msg in recent.iter().rev() {
            match msg.role.as_str() {
                "system" => {
                    if msg.content.contains("## Recent Context Summary") {
                        continue;
                    }
                }
                "user" => context.push_str(&format!("user: {}\n", truncate(&msg.content, 200))),
                "assistant" => {
                    if !msg.content.is_empty() {
                        context.push_str(&format!("assistant: {}\n", truncate(&msg.content, 300)));
                    }
                    if let Some(calls) = &msg.tool_calls {
                        for call in calls {
                            context.push_str(&format!(
                                "  -> tool: {} args: {}\n",
                                call.function.name,
                                truncate(&serde_json::to_string(&call.function.arguments).unwrap_or_default(), 100)
                            ));
                        }
                    }
                }
                "tool" => {
                    context.push_str(&format!(
                        "  <- {}: {}\n",
                        msg.name.as_deref().unwrap_or("tool"),
                        truncate(&msg.content, 200)
                    ));
                }
                _ => {}
            }
        }
        context
    }

    async fn generate_summary(&self, context: &str) -> Result<String> {
        let prompt = format!(
            "Summarize the following conversation context in 3-5 sentences. Focus on:\n\
            - What task was being worked on\n\
            - What has been completed\n\
            - What is the current state\n\
            - Any important decisions or findings\n\n\
            Context:\n{}",
            context
        );

        let temp_messages = vec![
            Message::user(prompt),
        ];

        let reply = self.llm.generate(&temp_messages, &Arc::new(vec![])).await?;
        Ok(reply.content)
    }

    fn is_cancelled(&self) -> bool {
        self.cancel
            .as_ref()
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(false)
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

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "..."
    }
}