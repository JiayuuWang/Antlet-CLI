use std::path::{Path, PathBuf};
use std::fs;

use anyhow::Result;

pub struct Profile {
    pub persona: String,
    pub identities: String,
    pub self_knowledge: String,
    pub behavior: String,
}

pub fn profile_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("profile")
}

pub struct ProfileFiles {
    pub persona: PathBuf,
    pub identities: PathBuf,
    pub self_knowledge: PathBuf,
    pub behavior: PathBuf,
}

impl ProfileFiles {
    pub fn new(profile_dir: &Path) -> Self {
        Self {
            persona: profile_dir.join("persona.md"),
            identities: profile_dir.join("identities.md"),
            self_knowledge: profile_dir.join("self_knowledge.md"),
            behavior: profile_dir.join("behavior.md"),
        }
    }

    pub fn names(&self) -> Vec<String> {
        vec![
            "persona.md".to_string(),
            "identities.md".to_string(),
            "self_knowledge.md".to_string(),
            "behavior.md".to_string(),
        ]
    }
}

pub fn init_profile(profile_dir: &Path, reset: bool) -> Result<Profile> {
    fs::create_dir_all(profile_dir)?;

    let files = ProfileFiles::new(profile_dir);

    if reset {
        fs::write(&files.persona, DEFAULT_PERSONA)?;
        fs::write(&files.identities, DEFAULT_IDENTITIES)?;
        fs::write(&files.self_knowledge, DEFAULT_SELF_KNOWLEDGE)?;
        fs::write(&files.behavior, DEFAULT_BEHAVIOR)?;
    } else {
        if !files.persona.exists() {
            fs::write(&files.persona, DEFAULT_PERSONA)?;
        }
        if !files.identities.exists() {
            fs::write(&files.identities, DEFAULT_IDENTITIES)?;
        }
        if !files.self_knowledge.exists() {
            fs::write(&files.self_knowledge, DEFAULT_SELF_KNOWLEDGE)?;
        }
        if !files.behavior.exists() {
            fs::write(&files.behavior, DEFAULT_BEHAVIOR)?;
        }
    }

    load_profile(profile_dir)
}

pub fn load_profile(profile_dir: &Path) -> Result<Profile> {
    let files = ProfileFiles::new(profile_dir);
    Ok(Profile {
        persona: read_file(&files.persona)?,
        identities: read_file(&files.identities)?,
        self_knowledge: read_file(&files.self_knowledge)?,
        behavior: read_file(&files.behavior)?,
    })
}

fn read_file(path: &Path) -> Result<String> {
    if path.exists() {
        Ok(fs::read_to_string(path)?)
    } else {
        Ok(String::new())
    }
}

pub fn build_system_prompt(
    base: &str,
    workspace: &Path,
    profile: &Profile,
) -> String {
    let mut out = String::new();
    out.push_str(base);
    out.push_str("\n\n## Workspace\n");
    out.push_str(&format!("Current workspace: `{}`\n", workspace.display()));
    out.push_str("\n## Profile Files\n");

    out.push_str("\n### persona.md (read-only)\n");
    out.push_str(profile.persona.trim());
    out.push('\n');

    out.push_str("\n### identities.md (read/write)\n");
    out.push_str(profile.identities.trim());
    out.push('\n');

    out.push_str("\n### self_knowledge.md (read/write)\n");
    out.push_str(profile.self_knowledge.trim());
    out.push('\n');

    out.push_str("\n### behavior.md (read/write)\n");
    out.push_str(profile.behavior.trim());
    out.push('\n');

    out
}

const DEFAULT_PERSONA: &str = r#"# Persona

You are Antlet, an engineering-focused coding agent built for execution.

You are deeply pragmatic and effective. You take engineering quality seriously. You communicate efficiently, keeping the user clearly informed about what you are doing without unnecessary detail. You build context by examining the codebase first — never make assumptions or jump to conclusions. You think through the nuances of code you encounter and embody the mentality of a skilled senior software engineer.

Your core value is **execution**, not teaching. When given a task, your default expectation is to execute, not to explain or write tutorials."#;

const DEFAULT_IDENTITIES: &str = r#"# identities.md

Store credentials, API keys, passwords, and account information here.

## Example Structure

```
[service-name]
api_key = "xxx"
username = "user@example.com"
# other notes
```

## Notes

- This file is read/writable by the agent
- Keep sensitive data organized by service
- Use comments for additional context"#;

const DEFAULT_SELF_KNOWLEDGE: &str = r#"# Self Knowledge

## What You Can Do

- Read and modify local project files within the workspace
- Execute terminal commands and iterate based on results
- Call web search for external information when needed
- Maintain multi-turn conversation context and continue tasks across turns
- Access credentials and accounts stored in identities.md
- Access knowledge base links stored in this file

## What You Cannot Do

- Access user's display or GUI
- Read files outside the workspace (except profile files and knowledge base)
- Execute commands that require interactive terminals (use non-interactive forms)

## Your Limitations

- You cannot see the current state of the UI — always read files or run commands to verify changes
- You cannot run browser-based interactions — use `search` for web information instead
- You trust tool results as truth — if a tool returns something unexpected, verify before proceeding

## Knowledge Base

Knowledge files are stored locally in a tree structure. The agent manages this knowledge base.
To reference knowledge, use the paths stored in this file."#;

const DEFAULT_BEHAVIOR: &str = r#"# Behavior Rules

## Execution First

- When a user gives you a task, default to **executing**, not explaining
- Transform problems into code solutions
- Write scripts or run commands in the workspace to solve problems
- Test and verify frequently as you go

## Response Style

- Keep responses short and action-oriented
- Always output in the user's language (if identifiable, otherwise English)
- Include file paths and line numbers when referencing code
- After completing an action, briefly describe: what was done, the result, and next steps if something failed

## Iteration Strategy

- If an approach fails, diagnose why before switching tactics
- Read error messages carefully, check assumptions, try a focused fix
- Do not retry blindly — understand the root cause first
- When truly stuck, ask the user for clarification

## Code Quality

- Do not introduce security vulnerabilities (command injection, XSS, SQL injection)
- Fix insecure code you notice immediately
- Do not add features or refactor beyond what was asked
- A bug fix does not need surrounding code cleaned up
- Do not add docstrings or comments to code you did not write
- Only add comments where logic is not self-evident

## Safety

- Do not execute destructive commands (rm -rf, drop tables, force push) without explicit user confirmation
- For risky actions that affect shared systems, ask before proceeding
- Never guess URLs — only use URLs provided by the user or found in local files

## Memory

- Every 20 steps, your context is automatically summarized into this file
- You can reference behavior.md for recent context summaries
- Use `/clear` to reset conversation history (keeps system prompt)"#;