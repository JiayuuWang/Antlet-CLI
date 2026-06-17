//! Centralized on-disk layout for `~/.antlet`.
//!
//! The backend is organized around the **agent** as the top-level unit — a
//! layer above the session. Every agent, whether it is the root started from
//! the CLI or a sub-agent spawned by another agent, is a *peer* directory under
//! `agents/`. There is no special place for sub-agents: parent and child are
//! organized identically.
//!
//! ```text
//! ~/.antlet/
//! ├── config.toml
//! ├── scheduled_tasks.json
//! └── agents/
//!     └── <agent-id>/
//!         ├── profile/        persona.md, identities.md, self_knowledge.md, behavior.md
//!         └── sessions/
//!             └── <session>.jsonl
//! ```
//!
//! An `agent-id` encodes lineage (see `subagent` for the naming rule), e.g.
//! `translate-book`, `translate-book.1-ch1`, `translate-book.1-ch1.2-sec2`.
//! Because the id is also the directory name, the on-disk tree mirrors the
//! live agent tree.

use std::path::{Path, PathBuf};

/// Default session name inside an agent that has a single conversation.
pub const DEFAULT_SESSION: &str = "main";

/// `~/.antlet/agents`
pub fn agents_root(data_dir: &Path) -> PathBuf {
    data_dir.join("agents")
}

/// `~/.antlet/agents/<agent-id>`
pub fn agent_dir(data_dir: &Path, agent_id: &str) -> PathBuf {
    agents_root(data_dir).join(sanitize(agent_id))
}

/// `~/.antlet/agents/<agent-id>/profile`
pub fn agent_profile_dir(data_dir: &Path, agent_id: &str) -> PathBuf {
    agent_dir(data_dir, agent_id).join("profile")
}

/// `~/.antlet/agents/<agent-id>/sessions`
pub fn agent_sessions_dir(data_dir: &Path, agent_id: &str) -> PathBuf {
    agent_dir(data_dir, agent_id).join("sessions")
}

/// `~/.antlet/agents/<agent-id>/sessions/<session>.jsonl`
pub fn agent_session_file(data_dir: &Path, agent_id: &str, session: &str) -> PathBuf {
    agent_sessions_dir(data_dir, agent_id).join(format!("{}.jsonl", sanitize(session)))
}

/// Make an id safe to use as a single path component. Agent ids use `.` to
/// separate lineage levels (e.g. `root.1-label`), which is fine as a directory
/// name, but we still strip path separators and other risky characters.
pub fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '\0' => '_',
            c => c,
        })
        .collect()
}

/// Rename an agent's entire directory (its profile + sessions move with it).
/// Used when the root agent's id is upgraded from `temp-<ts>` to a summary.
/// Returns the new agent directory path.
pub async fn rename_agent(data_dir: &Path, old_id: &str, new_id: &str) -> std::io::Result<PathBuf> {
    let old = agent_dir(data_dir, old_id);
    let new = agent_dir(data_dir, new_id);
    if let Some(parent) = new.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::rename(&old, &new).await?;
    Ok(new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn layout_is_agent_centric() {
        let d = Path::new("/home/u/.antlet");
        assert_eq!(agents_root(d), Path::new("/home/u/.antlet/agents"));
        assert_eq!(
            agent_dir(d, "root.1-ch1"),
            Path::new("/home/u/.antlet/agents/root.1-ch1")
        );
        assert_eq!(
            agent_profile_dir(d, "root.1-ch1"),
            Path::new("/home/u/.antlet/agents/root.1-ch1/profile")
        );
        assert_eq!(
            agent_session_file(d, "root.1-ch1", "main"),
            Path::new("/home/u/.antlet/agents/root.1-ch1/sessions/main.jsonl")
        );
    }

    #[test]
    fn sanitize_strips_separators() {
        assert_eq!(sanitize("a/b:c"), "a_b_c");
        assert_eq!(sanitize("root.1-label"), "root.1-label");
    }
}
