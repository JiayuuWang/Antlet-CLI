mod agent;
mod cli;
mod config;
mod llm;
mod profile;
mod schedule_store;
mod scheduler;
mod schema;
mod session_store;
mod tools;
mod ui;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use tokio::sync::Mutex;

use agent::Agent;
use cli::CliArgs;
use config::AppConfig;
use llm::LlmClient;
use profile::{build_system_prompt, init_profile, profile_dir, ProfileFiles};
use scheduler::Scheduler;
use session_store::SessionStore;
use tools::ToolRegistry;
use ui::{Color, print_banner};

fn data_dir_from_env() -> PathBuf {
    std::env::var("ANTLET_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join(".antlet"))
        })
        .unwrap_or_else(|| PathBuf::from(".antlet"))
}

fn profile_dir_from_env(data_dir: &PathBuf) -> PathBuf {
    std::env::var("ANTLET_PROFILE_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("profile"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArgs::parse();

    // Handle --schedule: add a scheduled task and exit (no agent needed)
    if let Some(schedule) = args.schedule.clone() {
        let data_dir = data_dir_from_env();
        let profile_dir = profile_dir_from_env(&data_dir);
        let workspace = tokio::fs::canonicalize(&args.workspace).await.unwrap_or(args.workspace.clone());
        let scheduler = Scheduler::new(data_dir.clone());
        let name = args.schedule_name.as_deref().unwrap_or("scheduled task");
        let task_text = args.task.clone().unwrap_or_default();

        // Ensure profile dir exists
        tokio::fs::create_dir_all(&profile_dir).await.ok();

        let task = scheduler.add_from_cli(
            &schedule,
            name,
            &task_text,
            &args.session,
            workspace.to_str().unwrap_or("."),
        ).await?;

        println!(
            "{}scheduler{}: task '{}' scheduled (id={})",
            Color::GREEN,
            Color::RESET,
            task.name,
            task.id
        );
        if let Some(next) = task.next_run {
            let dt = chrono::DateTime::from_timestamp(next, 0)
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| next.to_string());
            println!("{}scheduler{}: next run at {}", Color::DIM, Color::RESET, dt);
        }
        return Ok(());
    }

    // Agent modes (interactive/one-shot) require config
    let explicit_session = args.session != "default" && !args.session.starts_with("temp-");
    let session_name = if explicit_session {
        args.session.clone()
    } else {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("temp-{}", ts)
    };

    let data_dir = data_dir_from_env();
    let config = AppConfig::from_parts(
        args.workspace.clone(),
        args.max_steps,
        session_name.clone(),
        args.api_base.clone(),
        args.model.clone(),
        &data_dir,
    )?;

    let workspace = tokio::fs::canonicalize(&config.workspace).await.unwrap_or(config.workspace.clone());
    let base_prompt =
        "You are Antlet mini coding agent. Use tools when needed. Keep responses concise.";

    let profile_dir = profile_dir(&data_dir);
    let reset_profile = std::env::var("ANTLET_PROFILE_RESET")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let profile = init_profile(&profile_dir, reset_profile).await?;
    let profile_files = ProfileFiles::new(&profile_dir);
    let profile_file_names = profile_files.names();
    let system_prompt = build_system_prompt(base_prompt, &workspace, &profile);

    let llm = LlmClient::new(
        config.api_key.clone(),
        config.api_base.clone(),
        config.model.clone(),
    );
    let tools = ToolRegistry::with_profile(workspace.clone(), profile_dir.clone());
    let tool_names = tools.names();
    let session = SessionStore::new(&config.data_dir, &config.session);
    let mut agent = Agent::new(llm, tools, session, system_prompt, config.max_steps, profile_files).await?;

    print_banner(&config, &workspace, &tool_names, &profile_file_names);

    let mut session_renamed = false;

    if let Some(task) = args.task {
        println!("{}mode{}: one-shot", Color::DIM, Color::RESET);
        let result = agent.run_task(&task).await?;
        println!("\n{}{}{}", Color::BLUE, result, Color::RESET);
        return Ok(());
    }

    println!("{}mode{}: interactive", Color::DIM, Color::RESET);

    // Spawn scheduler in background
    let scheduler = Scheduler::new(config.data_dir.clone());
    let agent_arc = Arc::new(Mutex::new(Box::new(agent)));
    let agent_for_sched = agent_arc.clone();
    let scheduler_handle = tokio::spawn(async move {
        if let Err(e) = scheduler.run(agent_for_sched).await {
            eprintln!("scheduler error: {}", e);
        }
    });

    println!(
        "{}commands{}: /exit /clear /history /schedule",
        Color::DIM,
        Color::RESET
    );

    let stdin = tokio::io::stdin();
    let mut reader = tokio::io::BufReader::new(stdin);

    loop {
        use tokio::io::AsyncBufReadExt;
        use tokio::io::AsyncWriteExt;

        let mut stdout = tokio::io::stdout();
        stdout
            .write_all(format!("\n{}>{} ", Color::WHITE, Color::RESET).as_bytes())
            .await?;
        stdout.flush().await?;

        let mut line = String::new();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if matches!(input, "/exit" | "exit" | "quit" | "q") {
            break;
        }
        if input == "/history" {
            let agent = agent_arc.lock().await;
            println!(
                "{}messages{}: {}",
                Color::DIM,
                Color::RESET,
                agent.history_len()
            );
            continue;
        }
        if input == "/clear" {
            {
                let mut agent = agent_arc.lock().await;
                agent.clear_history_keep_system();
                agent.persist_all().await?;
            }
            println!("{}history cleared{}", Color::YELLOW, Color::RESET);
            continue;
        }
        if input == "/schedule" {
            let scheduler = Scheduler::new(config.data_dir.clone());
            let tasks = scheduler.load_tasks().await.unwrap_or_default();
            if tasks.is_empty() {
                println!("{}schedule{}: no scheduled tasks", Color::DIM, Color::RESET);
            } else {
                println!("{}schedule{}: {} task(s)", Color::DIM, Color::RESET, tasks.len());
                for t in &tasks {
                    let next = t.next_run.map(|ts| {
                        chrono::DateTime::from_timestamp(ts, 0)
                            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_else(|| ts.to_string())
                    }).unwrap_or_else(|| "disabled".to_string());
                    println!(
                        "  {} - {} [{}] next:{}",
                        t.id,
                        t.name,
                        if t.enabled { "enabled" } else { "disabled" },
                        next
                    );
                }
            }
            continue;
        }

        println!("{}user>{} {}", Color::CYAN, Color::RESET, input);
        let result = {
            let mut agent = agent_arc.lock().await;
            let r = agent.run_task(input).await;

            if !session_renamed && !explicit_session {
                if r.is_ok() {
                    if let Some(first) = agent.get_first_response() {
                        let summary = summarize_for_session(&first);
                        if !summary.is_empty() {
                            if let Ok(new_store) = SessionStore::new(&config.data_dir, &config.session).rename_to(&summary).await {
                                session_renamed = true;
                                if let Ok(msgs) = new_store.load().await {
                                    agent.replace_messages(msgs);
                                }
                                println!(
                                    "{}session{}: renamed to '{}'",
                                    Color::YELLOW,
                                    Color::RESET,
                                    summary
                                );
                            }
                        }
                    }
                }
            }
            r?
        };
        println!("\n{}assistant>{} {}", Color::BLUE, Color::RESET, result);
    }

    scheduler_handle.abort();
    Ok(())
}

fn summarize_for_session(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut result = String::new();
    for w in words {
        if result.len() >= 48 {
            break;
        }
        if !result.is_empty() {
            result.push('-');
        }
        let clean: String = w
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect();
        if !clean.is_empty() {
            result.push_str(&clean);
        }
    }
    result
}