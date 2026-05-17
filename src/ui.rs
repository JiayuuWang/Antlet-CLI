use std::path::Path;

use crate::config::AppConfig;

pub struct Color;

impl Color {
    pub const RESET: &'static str = "\x1b[0m";
    pub const DIM: &'static str = "\x1b[2m";
    pub const BOLD: &'static str = "\x1b[1m";
    pub const CYAN: &'static str = "\x1b[36m";
    pub const BLUE: &'static str = "\x1b[34m";
    pub const GREEN: &'static str = "\x1b[32m";
    pub const YELLOW: &'static str = "\x1b[33m";
    pub const MAGENTA: &'static str = "\x1b[35m";
    pub const RED: &'static str = "\x1b[31m";
    pub const WHITE: &'static str = "\x1b[97m";
    pub const THINK: &'static str = "\x1b[90m";
}

pub fn print_banner(
    config: &AppConfig,
    workspace: &Path,
    tool_names: &[String],
    profile_files: &[String],
) {
    println!(
        "{}{}\n █████╗ ███╗   ██╗████████╗██╗     ███████╗████████╗\n██╔══██╗████╗  ██║╚══██╔══╝██║     ██╔════╝╚══██╔══╝\n███████║██╔██╗ ██║   ██║   ██║     █████╗     ██║   \n██╔══██║██║╚██╗██║   ██║   ██║     ██╔══╝     ██║   \n██║  ██║██║ ╚████║   ██║   ███████╗███████╗   ██║   \n╚═╝  ╚═╝╚═╝  ╚═══╝   ╚═╝   ╚══════╝╚══════╝   ╚═╝   {}",
        Color::BOLD,
        Color::CYAN,
        Color::RESET
    );

    println!(
        "{}mode{}: {}{}{}",
        Color::DIM,
        Color::RESET,
        Color::WHITE,
        if config.max_steps > 0 {
            "agent"
        } else {
            "unknown"
        },
        Color::RESET
    );
    println!(
        "{}workspace{}: {}{}{}",
        Color::DIM,
        Color::RESET,
        Color::GREEN,
        workspace.display(),
        Color::RESET
    );
    println!(
        "{}session{}: {}{}{}",
        Color::DIM,
        Color::RESET,
        Color::MAGENTA,
        config.session,
        Color::RESET
    );
    println!(
        "{}model{}: {}{}{}",
        Color::DIM,
        Color::RESET,
        Color::BLUE,
        config.model,
        Color::RESET
    );
    println!(
        "{}api_base{}: {}{}{}",
        Color::DIM,
        Color::RESET,
        Color::BLUE,
        config.api_base,
        Color::RESET
    );
    println!(
        "{}profile_dir{}: {}{}{}",
        Color::DIM,
        Color::RESET,
        Color::YELLOW,
        config.profile_dir.display(),
        Color::RESET
    );
    println!(
        "{}profile_files{}: {}{}{}",
        Color::DIM,
        Color::RESET,
        Color::WHITE,
        profile_files.join(", "),
        Color::RESET
    );
    println!(
        "{}max_steps{}: {}{}{}",
        Color::DIM,
        Color::RESET,
        Color::YELLOW,
        config.max_steps,
        Color::RESET
    );
    println!(
        "{}tools{}: {}{}{}\n",
        Color::DIM,
        Color::RESET,
        Color::WHITE,
        tool_names.join(", "),
        Color::RESET
    );
}
