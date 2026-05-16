# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build

```bash
cargo build
```

Requires Rust 1.94+.

## Run

```bash
# Interactive mode
cargo run -- --workspace /path/to/project --session mysession

# One-shot task mode
cargo run -- --workspace /path/to/project --task "Summarize the architecture"
```

## Test

```bash
cargo test
```

## Architecture

**Core loop**: `Agent::run_task()` in `src/agent.rs` implements the agent loop:
1. Send messages + tool schemas to LLM
2. If LLM returns tool calls → execute tools and loop back
3. If LLM returns text → return as final answer

**LLM client**: `src/llm.rs` - `LlmClient` calls OpenAI-compatible `/chat/completions` API. Handles message conversion for system/user/assistant/tool roles and tool call parsing.

**Tool system**: `src/tools/mod.rs` - `ToolRegistry` holds all tools. Each tool implements `Tool` trait (`name`, `description`, `parameters`, `execute`). Built-in tools:
- `read` - read files with line numbers
- `write` - write file or edit via text replacement (old/new, or full overwrite via content)
- `grep` - search for regex pattern in files
- `glob` - find files by name pattern
- `ls` - list directory contents
- `bash` - command execution
- `search` - Tavily web search (requires `TAVILY_API_KEY`)

**Scheduler**: `src/scheduler.rs` + `src/schedule_store.rs` - background task scheduler. Supports cron expressions and one-shot timestamps. Tasks stored in `{data_dir}/scheduled_tasks.json`. In interactive mode, scheduler runs as background async task calling `agent.run_task()` when tasks fire.

**Session persistence**: `src/session_store.rs` - sessions stored as JSONL in `~/.antlet/sessions/<session>.jsonl`. Messages appended on each turn.

**System prompt**: `src/profile.rs` - builds system prompt from markdown files in `~/.antlet/profile/` (`persona.md`, `capabilities.md`, `self_knowledge.md`, `behavior.md`). Template created on first run if missing.

**Configuration**: `src/config.rs` - `AppConfig` loads from env vars (`ANTLET_API_KEY`, `ANTLET_API_BASE`, `ANTLET_MODEL`, `ANTLET_HOME`, `ANTLET_PROFILE_DIR`, etc.) and CLI args.

**CLI entry**: `src/cli.rs` defines `CliArgs` with `--workspace`, `--session`, `--task`, `--max-steps`, `--api-base`, `--model`, `--schedule`, `--schedule-name`.

## Environment Variables

- `ANTLET_API_KEY` - required
- `ANTLET_API_BASE` - defaults to `https://api.minimaxi.com/v1`
- `ANTLET_MODEL` - defaults to `MiniMax-M2.5`
- `ANTLET_HOME` - data directory, defaults to `~/.antlet`
- `TAVILY_API_KEY` - for web search tool