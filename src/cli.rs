use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "antlet-agent")]
#[command(about = "A simple Rust coding agent for Antlet")]
pub struct CliArgs {
    #[arg(long)]
    pub task: Option<String>,

    #[arg(long, default_value = ".")]
    pub workspace: PathBuf,

    #[arg(long, default_value_t = 20)]
    pub max_steps: usize,

    #[arg(long, default_value = "default")]
    pub session: String,

    #[arg(long)]
    pub api_base: Option<String>,

    #[arg(long)]
    pub model: Option<String>,

    #[arg(long)]
    pub schedule: Option<String>,

    #[arg(long)]
    pub schedule_name: Option<String>,

    #[arg(long)]
    pub session_hint: Option<String>,
}