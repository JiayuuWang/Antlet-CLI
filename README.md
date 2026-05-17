# Antlet Agent

> A nano coding agent built with Rust, supporting multi-agent scheduling, memory management, and cron-triggered tasks.

[English](./README.md) | [中文](./README_zh.md)

---

## Quick Start

### 1. Build

```bash
cargo build
```

### 2. Configure

**Option A: Environment variables (legacy)**

```bash
export ANTLET_API_KEY="your_api_key"
export ANTLET_API_BASE="https://api.minimaxi.com/v1"   # optional, defaults to MiniMax
export ANTLET_MODEL="MiniMax-M2.5"                      # optional
export TAVILY_API_KEY="your_tavily_key"                 # optional, for search tool
```

**Option B: Persistent config file (recommended)**

Create `~/.antlet/config.toml`:

```toml
ANTLET_API_KEY = "your_api_key"
ANTLET_API_BASE = "https://api.minimaxi.com/v1"  # optional
ANTLET_MODEL = "MiniMax-M2.5"                    # optional
TAVILY_API_KEY = "your_tavily_key"               # optional
```

Environment variables take precedence over config file values.

### 3. Run

```bash
# Interactive mode
cargo run -- --workspace /path/to/your/project --session demo

# One-shot task mode
cargo run -- --workspace /path/to/your/project --task "Fix the compile error in src/main.rs"
```

---

## Configuration

Configuration can be set via environment variables (highest priority) or `~/.antlet/config.toml`.

| Variable | Config File | Required | Default | Description |
|----------|-------------|----------|---------|-------------|
| `ANTLET_API_KEY` | Yes | Yes | - | API key for LLM |
| `ANTLET_API_BASE` | Yes | No | `https://api.minimaxi.com/v1` | OpenAI-compatible endpoint |
| `ANTLET_MODEL` | Yes | No | `MiniMax-M2.5` | Model name |
| `ANTLET_HOME` | No | No | `~/.antlet` | Data directory |
| `ANTLET_PROFILE_DIR` | No | No | `~/.antlet/profile` | System prompt templates directory |
| `ANTLET_PROFILE_RESET` | No | No | `0` | Set to `1` to reset profile templates |
| `TAVILY_API_KEY` | Yes | No | - | Required for `search` tool |

---

## Scheduled Tasks

Schedule tasks to run automatically at specified times. Both cron expressions and one-shot timestamps are supported.

```bash
# Schedule a cron task (every 10 minutes)
cargo run -- --schedule "*/10 * * * *" --schedule-name "health check" --workspace . --task "check system health"

# Schedule a one-shot task (unix timestamp)
cargo run -- --schedule "1747500000" --schedule-name "deploy" --workspace . --task "run deploy script"
```

**In interactive mode**, the scheduler runs in the background and fires tasks automatically. Use `/schedule` to list all scheduled tasks.

### Schedule Format

- **Cron**: `min hour day month dow` — e.g. `0 9 * * *` (9am daily), `*/10 * * * *` (every 10 min)
- **One-shot**: unix timestamp in seconds — e.g. `1747500000`

### Interactive Commands

- `/history` - show message count
- `/clear` - clear history (keeps system prompt)
- `/schedule` - list scheduled tasks
- `/exit` - quit

---

## Built-in Tools

| Tool | Description |
|------|-------------|
| `read` | Read file contents with line numbers |
| `write` | Write file or edit via text replacement |
| `grep` | Search for regex pattern in files |
| `glob` | Find files by name pattern |
| `ls` | List directory contents |
| `bash` | Execute shell commands |
| `search` | Web search via Tavily |
| `write_profile` | Write to profile files (identities.md, self_knowledge.md, behavior.md) |

---

## Session

Sessions are stored as JSONL in `~/.antlet/sessions/<session>.jsonl`. Scheduled tasks are stored in `~/.antlet/scheduled_tasks.json`. Config is stored in `~/.antlet/config.toml`. Sessions persist across restarts and are independent of workspace location.

---

## CLI Options

```bash
--workspace PATH    Working directory (default: .)
--session NAME      Session name (default: default)
--task TEXT         One-shot task mode
--max-steps N       Max loop iterations (default: 20)
--api-base URL      Override API endpoint
--model NAME        Override model name
--schedule CRON     Schedule a recurring task (cron expression)
--schedule-name     Name for the scheduled task
```

---

## System Prompt

On startup, Antlet reads markdown files from `~/.antlet/profile/` (or `ANTLET_PROFILE_DIR`) to build the system prompt:

| File | Description | Access |
|------|-------------|--------|
| `persona.md` | Agent identity and values | read-only |
| `identities.md` | Credentials, API keys, accounts | read/write |
| `self_knowledge.md` | Agent capabilities and knowledge base links | read/write |
| `behavior.md` | Execution rules, response style, memory entries | read/write |

Templates are created automatically on first run. Edit these files to customize behavior.

**identities.md** - Store credentials, API keys, passwords, and account information organized by service.

**self_knowledge.md** - Contains links to local knowledge files (stored in tree structure). The agent manages this knowledge base.

---

## Memory

Every 20 steps, Antlet automatically summarizes the current context:

1. **Summary generation**: Recent conversation (last 40 messages) is sent to LLM for summarization
2. **Persistent storage**: Summary is appended to `behavior.md` as a Memory Entry with timestamp
3. **Context injection**: Summary is inserted into current message history as a user message

This ensures long-running sessions maintain context without token bloat. Memory entries in `behavior.md` provide a searchable history of task progress.

---

## Architecture

```
src/
├── main.rs          # entry point, CLI parsing, interactive loop
├── agent.rs         # Agent struct, run_task loop
├── llm.rs           # LlmClient, OpenAI-compatible /chat/completions
├── schema.rs        # Message, ToolCall, FunctionCall types
├── scheduler.rs     # task scheduler with cron and one-shot support
├── schedule_store.rs # JSON persistence for scheduled tasks
├── tools/
│   ├── mod.rs       # ToolRegistry, Tool trait
│   ├── read.rs      # read files with line numbers
│   ├── write.rs     # write file or text replacement
│   ├── grep.rs      # regex search in files
│   ├── glob.rs      # find files by pattern
│   ├── ls.rs        # list directory contents
│   ├── bash.rs      # execute shell commands
│   └── search.rs    # web search via Tavily
├── config.rs        # AppConfig, environment variable loading
├── profile.rs       # system prompt building from .md files
├── session_store.rs # JSONL session persistence
└── ui.rs            # colored terminal output
```

Core loop in `Agent::run_task()`: send messages + tool schemas → LLM → if tool calls, execute and repeat → else return text.

---

## License

MIT