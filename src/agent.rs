use std::path::PathBuf;

use anyhow::Result;

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
        })
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

    pub fn profile_dir(&self) -> PathBuf {
        self.profile_files.persona.parent().unwrap().to_path_buf()
    }

    pub async fn run_task(&mut self, input: &str) -> Result<String> {
        let user = Message::user(input.to_string());
        self.messages.push(user.clone());
        self.session.append(&user).await?;

        for step in 0..self.max_steps {
            self.step_count += 1;

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

            let assistant = Message::assistant(reply.content.clone(), Some(reply.tool_calls.clone()));
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
                if self.step_count % MEMORY_SUMMARY_INTERVAL == 0 {
                    self.summarize_and_update_behavior().await?;
                }
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

        let behavior_path = &self.profile_files.behavior;
        let existing = std::fs::read_to_string(behavior_path).unwrap_or_default();
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

        let new_entry = format!(
            "\n\n## Memory Entry [{}]\n{}\n---\n",
            timestamp, summary
        );

        let updated = if existing.contains("## Memory Entry") {
            existing
        } else {
            existing
        } + &new_entry;

        std::fs::write(behavior_path, &updated)?;

        let memory_block = format!(
            "[Memory update at step {}]: {}",
            self.step_count, summary
        );

        let mem_msg = Message::user(memory_block);
        self.messages.push(mem_msg.clone());
        self.session.append(&mem_msg).await?;

        println!(
            "{}memory{}: context summarized and stored at step {}",
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

        let reply = self.llm.generate(&temp_messages, &[]).await?;
        Ok(reply.content)
    }

    fn find_system_prompt_end(&self) -> Option<usize> {
        for (i, msg) in self.messages.iter().enumerate() {
            if msg.role == "system" {
                if i == 0 {
                    let content = &msg.content;
                    if let Some(pos) = content.find("\n## Profile Files\n") {
                        return Some(pos);
                    }
                    if let Some(pos) = content.find("\n## Workspace\n") {
                        return Some(pos);
                    }
                }
            }
        }
        None
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