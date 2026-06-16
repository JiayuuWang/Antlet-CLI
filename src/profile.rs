use std::path::{Path, PathBuf};

use anyhow::Result;
use tokio::fs;

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

pub async fn init_profile(profile_dir: &Path, reset: bool) -> Result<Profile> {
    fs::create_dir_all(profile_dir).await?;

    let files = ProfileFiles::new(profile_dir);

    if reset {
        fs::write(&files.persona, DEFAULT_PERSONA).await?;
        fs::write(&files.identities, DEFAULT_IDENTITIES).await?;
        fs::write(&files.self_knowledge, DEFAULT_SELF_KNOWLEDGE).await?;
        fs::write(&files.behavior, DEFAULT_BEHAVIOR).await?;
    } else {
        if !files.persona.exists() {
            fs::write(&files.persona, DEFAULT_PERSONA).await?;
        }
        if !files.identities.exists() {
            fs::write(&files.identities, DEFAULT_IDENTITIES).await?;
        }
        if !files.self_knowledge.exists() {
            fs::write(&files.self_knowledge, DEFAULT_SELF_KNOWLEDGE).await?;
        }
        if !files.behavior.exists() {
            fs::write(&files.behavior, DEFAULT_BEHAVIOR).await?;
        }
    }

    load_profile(profile_dir).await
}

/// Optional initial overrides for a sub-agent's writable profile files.
/// Any field left `None` falls back to the built-in default template.
#[derive(Debug, Clone, Default)]
pub struct SubProfileInit {
    pub persona: Option<String>,
    pub identities: Option<String>,
    pub self_knowledge: Option<String>,
    pub behavior: Option<String>,
}

/// Materialize an isolated profile directory for a sub-agent. Each writable
/// file is seeded from the provided override or the built-in default. This
/// keeps every agent's memory/persona writes fully separated from the parent.
pub async fn init_sub_profile(profile_dir: &Path, init: &SubProfileInit) -> Result<Profile> {
    fs::create_dir_all(profile_dir).await?;
    let files = ProfileFiles::new(profile_dir);

    let persona = init.persona.clone().unwrap_or_else(|| DEFAULT_PERSONA.to_string());
    let identities = init.identities.clone().unwrap_or_else(|| DEFAULT_IDENTITIES.to_string());
    let self_knowledge = init
        .self_knowledge
        .clone()
        .unwrap_or_else(|| DEFAULT_SELF_KNOWLEDGE.to_string());
    let behavior = init.behavior.clone().unwrap_or_else(|| DEFAULT_BEHAVIOR.to_string());

    fs::write(&files.persona, &persona).await?;
    fs::write(&files.identities, &identities).await?;
    fs::write(&files.self_knowledge, &self_knowledge).await?;
    fs::write(&files.behavior, &behavior).await?;

    Ok(Profile {
        persona,
        identities,
        self_knowledge,
        behavior,
    })
}

pub async fn load_profile(profile_dir: &Path) -> Result<Profile> {
    let files = ProfileFiles::new(profile_dir);
    Ok(Profile {
        persona: read_file(&files.persona).await?,
        identities: read_file(&files.identities).await?,
        self_knowledge: read_file(&files.self_knowledge).await?,
        behavior: read_file(&files.behavior).await?,
    })
}

async fn read_file(path: &Path) -> Result<String> {
    if path.exists() {
        let contents = fs::read_to_string(path).await?;
        Ok(contents)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn sub_profile_uses_overrides_and_defaults() {
        let dir = tempdir().unwrap();
        let pd = dir.path().join("sub_root_1");
        let init = SubProfileInit {
            persona: Some("CUSTOM PERSONA".to_string()),
            identities: None,
            self_knowledge: Some("CUSTOM SK".to_string()),
            behavior: None,
        };
        let profile = init_sub_profile(&pd, &init).await.unwrap();
        // overrides applied
        assert_eq!(profile.persona, "CUSTOM PERSONA");
        assert_eq!(profile.self_knowledge, "CUSTOM SK");
        // defaults fall through
        assert_eq!(profile.identities, DEFAULT_IDENTITIES);
        assert_eq!(profile.behavior, DEFAULT_BEHAVIOR);

        // files actually written to the isolated dir
        let files = ProfileFiles::new(&pd);
        let persona_on_disk = tokio::fs::read_to_string(&files.persona).await.unwrap();
        assert_eq!(persona_on_disk, "CUSTOM PERSONA");
        assert!(files.behavior.exists());
    }

    #[tokio::test]
    async fn sub_profiles_are_isolated() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        init_sub_profile(&a, &SubProfileInit { persona: Some("A".into()), ..Default::default() }).await.unwrap();
        init_sub_profile(&b, &SubProfileInit { persona: Some("B".into()), ..Default::default() }).await.unwrap();
        let pa = tokio::fs::read_to_string(ProfileFiles::new(&a).persona).await.unwrap();
        let pb = tokio::fs::read_to_string(ProfileFiles::new(&b).persona).await.unwrap();
        assert_eq!(pa, "A");
        assert_eq!(pb, "B");
    }
}
