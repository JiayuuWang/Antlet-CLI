mod agent;
mod cli;
mod config;
mod llm;
mod profile;
mod schema;
mod session_store;
mod tools;
mod ui;

use anyhow::Result;
use clap::Parser;

use agent::Agent;
use cli::CliArgs;
use config::AppConfig;
use llm::LlmClient;
use profile::{build_system_prompt, ensure_and_load_profile, profile_file_names};
use session_store::SessionStore;
use tools::ToolRegistry;
use ui::{Color, print_banner};

#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArgs::parse();
    let config = AppConfig::from_parts(
        args.workspace.clone(),
        args.max_steps,
        args.session.clone(),
        args.api_base.clone(),
        args.model.clone(),
    )?;

    let workspace = std::fs::canonicalize(&config.workspace).unwrap_or(config.workspace.clone());
    let base_prompt =
        "You are Antlet mini coding agent. Use tools when needed. Keep responses concise.";

    let profile_docs = ensure_and_load_profile(&config.profile_dir)?;
    let profile_files = profile_file_names(&profile_docs);
    let system_prompt = build_system_prompt(base_prompt, &workspace, &profile_docs);

    let llm = LlmClient::new(
        config.api_key.clone(),
        config.api_base.clone(),
        config.model.clone(),
    );
    let tools = ToolRegistry::default_for(workspace.clone());
    let tool_names = tools.names();
    let session = SessionStore::new(&config.data_dir, &config.session);
    let mut agent = Agent::new(llm, tools, session, system_prompt, config.max_steps).await?;

    print_banner(&config, &workspace, &tool_names, &profile_files);

    if let Some(task) = args.task {
        println!("{}mode{}: one-shot", Color::DIM, Color::RESET);
        let result = agent.run_task(&task).await?;
        println!("\n{}{}{}", Color::BLUE, result, Color::RESET);
        return Ok(());
    }

    println!("{}mode{}: interactive", Color::DIM, Color::RESET);
    println!(
        "{}commands{}: /exit /clear /history",
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
            println!(
                "{}messages{}: {}",
                Color::DIM,
                Color::RESET,
                agent.history_len()
            );
            continue;
        }
        if input == "/clear" {
            agent.clear_history_keep_system();
            agent.persist_all().await?;
            println!("{}history cleared{}", Color::YELLOW, Color::RESET);
            continue;
        }

        println!("{}user>{} {}", Color::CYAN, Color::RESET, input);
        let result = agent.run_task(input).await?;
        println!("\n{}assistant>{} {}", Color::BLUE, Color::RESET, result);
    }

    Ok(())
}
