mod agent;
mod cli;
mod config;
mod llm;
mod schema;
mod session_store;
mod tools;
mod leetcode_hot10;

use anyhow::Result;
use clap::Parser;

use agent::Agent;
use cli::CliArgs;
use config::AppConfig;
use llm::LlmClient;
use session_store::SessionStore;
use tools::ToolRegistry;

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
    let system_prompt = format!(
        "You are Antlet mini coding agent. Work in workspace: {}. Use tools when needed. Keep responses concise.",
        workspace.display()
    );

    let llm = LlmClient::new(config.api_key, config.api_base, config.model);
    let tools = ToolRegistry::default_for(workspace.clone());
    let session = SessionStore::new(&config.session);
    let mut agent = Agent::new(llm, tools, session, system_prompt, config.max_steps).await?;

    if let Some(task) = args.task {
        let result = agent.run_task(&task).await?;
        println!("\n{}", result);
        return Ok(());
    }

    println!("antlet-agent interactive mode");
    println!("workspace: {}", workspace.display());
    println!("commands: /exit /clear /history");

    let stdin = tokio::io::stdin();
    let mut reader = tokio::io::BufReader::new(stdin);

    loop {
        use tokio::io::AsyncBufReadExt;
        use tokio::io::AsyncWriteExt;

        let mut stdout = tokio::io::stdout();
        stdout.write_all(b"\n> ").await?;
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
            println!("messages: {}", agent.history_len());
            continue;
        }
        if input == "/clear" {
            agent.clear_history_keep_system();
            agent.persist_all().await?;
            println!("history cleared");
            continue;
        }

        let result = agent.run_task(input).await?;
        println!("\nassistant> {}", result);
    }

    Ok(())
}
