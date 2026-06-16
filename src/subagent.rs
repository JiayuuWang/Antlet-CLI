//! Sub-agent orchestration.
//!
//! A parent agent can `spawn` one or more child agents that each run a task in
//! the background, then later `stop` (and harvest the result of) any child.
//!
//! Design goals:
//! - **Non-blocking spawn**: children run on their own tokio tasks; spawn
//!   returns immediately with their ids.
//! - **No interference**: every agent (parent or child) gets its own session
//!   file and its own isolated profile directory, so memory/persona writes
//!   never collide.
//! - **Unbounded recursion**: children carry the same spawn/stop tools, so they
//!   can spawn their own children to any depth. A soft global cap on the number
//!   of simultaneously-live agents acts as a safety valve only.
//! - **Cooperative cancellation + cascade**: stopping an agent flips a cancel
//!   flag (graceful) or aborts (forceful) and recursively reaps all of its
//!   descendants, cleaning up their temp profile dirs.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::agent::Agent;
use crate::llm::LlmClient;
use crate::profile::{build_system_prompt, init_sub_profile, ProfileFiles, SubProfileInit};
use crate::session_store::SessionStore;
use crate::tools::ToolRegistry;
use crate::ui::Color;

/// Soft default cap on the number of simultaneously-live sub-agents across the
/// whole tree. Override with `ANTLET_MAX_LIVE_SUBAGENTS`; set to 0 to disable.
const DEFAULT_MAX_LIVE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildStatus {
    Running,
    Completed,
    Failed,
    Stopped,
}

impl ChildStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChildStatus::Running => "running",
            ChildStatus::Completed => "completed",
            ChildStatus::Failed => "failed",
            ChildStatus::Stopped => "stopped",
        }
    }
}

/// Immutable configuration shared by the whole agent tree. One `Arc` is cloned
/// down into every manager so children are built exactly like the root.
pub struct AgentFactory {
    pub api_key: String,
    pub api_base: String,
    pub model: String,
    pub base_prompt: String,
    pub workspace: PathBuf,
    pub data_dir: PathBuf,
    pub max_steps: usize,
    /// Global counter of live sub-agents (shared across the entire tree).
    pub live_count: AtomicUsize,
    pub max_live: usize,
}

impl AgentFactory {
    pub fn new(
        api_key: String,
        api_base: String,
        model: String,
        base_prompt: String,
        workspace: PathBuf,
        data_dir: PathBuf,
        max_steps: usize,
    ) -> Self {
        let max_live = std::env::var("ANTLET_MAX_LIVE_SUBAGENTS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX_LIVE);
        Self {
            api_key,
            api_base,
            model,
            base_prompt,
            workspace,
            data_dir,
            max_steps,
            live_count: AtomicUsize::new(0),
            max_live,
        }
    }

    fn sub_profiles_root(&self) -> PathBuf {
        self.data_dir.join("sub_profiles")
    }
}

/// One entry per live child owned by a manager.
struct ChildEntry {
    id: String,
    task: String,
    status: Arc<Mutex<ChildStatus>>,
    result: Arc<Mutex<Option<String>>>,
    handle: Mutex<Option<JoinHandle<()>>>,
    cancel: Arc<AtomicBool>,
    /// The child's own manager, so we can cascade stop/reap its descendants.
    child_manager: Arc<SubAgentManager>,
    profile_dir: PathBuf,
}

/// Specification for a single child to spawn.
#[derive(Debug, Clone)]
pub struct SpawnSpec {
    pub system_prompt: String,
    pub task: Option<String>,
    pub init: SubProfileInit,
}

/// Manages the direct children of a single agent node in the tree.
pub struct SubAgentManager {
    factory: Arc<AgentFactory>,
    /// Hierarchical id prefix, e.g. "root", "root.1", "root.1.2".
    id_prefix: String,
    /// Monotonic counter for assigning child ordinals.
    next_ordinal: AtomicUsize,
    children: Mutex<HashMap<String, Arc<ChildEntry>>>,
}

impl SubAgentManager {
    pub fn new_root(factory: Arc<AgentFactory>) -> Arc<Self> {
        Arc::new(Self {
            factory,
            id_prefix: "root".to_string(),
            next_ordinal: AtomicUsize::new(0),
            children: Mutex::new(HashMap::new()),
        })
    }

    fn new_child_manager(factory: Arc<AgentFactory>, id_prefix: String) -> Arc<Self> {
        Arc::new(Self {
            factory,
            id_prefix,
            next_ordinal: AtomicUsize::new(0),
            children: Mutex::new(HashMap::new()),
        })
    }

    fn default_task(system_prompt: &str) -> String {
        let _ = system_prompt;
        "Begin working on the role and objective described in your system prompt. \
         Use your tools to complete the work. When finished, provide a concise \
         summary of what you accomplished and the final results."
            .to_string()
    }

    /// Spawn one or more children. Returns `(id, task)` pairs immediately; the
    /// children run in the background.
    pub async fn spawn(self: &Arc<Self>, specs: Vec<SpawnSpec>) -> Result<Vec<(String, String)>> {
        if specs.is_empty() {
            return Err(anyhow!("no agents specified (empty array)"));
        }

        let mut spawned = Vec::with_capacity(specs.len());

        for spec in specs {
            // Soft global safety valve.
            let max_live = self.factory.max_live;
            if max_live > 0 {
                let current = self.factory.live_count.load(Ordering::SeqCst);
                if current >= max_live {
                    return Err(anyhow!(
                        "live sub-agent cap reached ({}/{}). Stop some children first or raise ANTLET_MAX_LIVE_SUBAGENTS.",
                        current, max_live
                    ));
                }
            }

            let ordinal = self.next_ordinal.fetch_add(1, Ordering::SeqCst) + 1;
            let id = format!("{}.{}", self.id_prefix, ordinal);
            let task = spec.task.clone().unwrap_or_else(|| Self::default_task(&spec.system_prompt));

            // Isolated profile dir: persona = the provided system_prompt.
            let profile_dir = self.factory.sub_profiles_root().join(id.replace('.', "_"));
            let init = SubProfileInit {
                persona: Some(spec.system_prompt.clone()),
                identities: spec.init.identities.clone(),
                self_knowledge: spec.init.self_knowledge.clone(),
                behavior: spec.init.behavior.clone(),
            };

            let profile = match init_sub_profile(&profile_dir, &init).await {
                Ok(p) => p,
                Err(e) => return Err(anyhow!("failed to init sub-agent profile: {}", e)),
            };

            let system_prompt =
                build_system_prompt(&self.factory.base_prompt, &self.factory.workspace, &profile);

            let llm = LlmClient::new(
                self.factory.api_key.clone(),
                self.factory.api_base.clone(),
                self.factory.model.clone(),
            );

            // Child manager enables recursive spawning.
            let child_manager = Self::new_child_manager(self.factory.clone(), id.clone());

            let tools = ToolRegistry::with_subagents(
                self.factory.workspace.clone(),
                profile_dir.clone(),
                child_manager.clone(),
            );

            let session = SessionStore::new(
                &self.factory.data_dir,
                &format!("subagent-{}", id.replace('.', "_")),
            );

            let profile_files = ProfileFiles::new(&profile_dir);

            let mut agent = Agent::new(
                llm,
                tools,
                session,
                system_prompt,
                self.factory.max_steps,
                profile_files,
            )
            .await?;

            let cancel = Arc::new(AtomicBool::new(false));
            agent.set_cancel(cancel.clone());

            let status = Arc::new(Mutex::new(ChildStatus::Running));
            let result = Arc::new(Mutex::new(None));

            // Bump live counter before spawning.
            self.factory.live_count.fetch_add(1, Ordering::SeqCst);

            let status_bg = status.clone();
            let result_bg = result.clone();
            let factory_bg = self.factory.clone();
            let id_bg = id.clone();
            let task_bg = task.clone();

            eprintln!(
                "{}subagent{}: spawned {} (task: {})",
                Color::CYAN,
                Color::RESET,
                id,
                truncate(&task, 120)
            );

            let handle = tokio::spawn(async move {
                let mut agent = agent;
                let outcome = agent.run_task(&task_bg).await;
                match outcome {
                    Ok(text) => {
                        *result_bg.lock().await = Some(text.clone());
                        let mut st = status_bg.lock().await;
                        // Don't override a Stopped status set by a forceful stop.
                        if *st == ChildStatus::Running {
                            *st = ChildStatus::Completed;
                        }
                        eprintln!(
                            "{}subagent{}: {} completed: {}",
                            Color::GREEN,
                            Color::RESET,
                            id_bg,
                            truncate(&text, 160)
                        );
                    }
                    Err(e) => {
                        *result_bg.lock().await = Some(format!("error: {}", e));
                        let mut st = status_bg.lock().await;
                        if *st == ChildStatus::Running {
                            *st = ChildStatus::Failed;
                        }
                        eprintln!(
                            "{}subagent{}: {} failed: {}",
                            Color::RED,
                            Color::RESET,
                            id_bg,
                            e
                        );
                    }
                }
                factory_bg.live_count.fetch_sub(1, Ordering::SeqCst);
            });

            let entry = Arc::new(ChildEntry {
                id: id.clone(),
                task: task.clone(),
                status,
                result,
                handle: Mutex::new(Some(handle)),
                cancel,
                child_manager,
                profile_dir,
            });

            self.children.lock().await.insert(id.clone(), entry);
            spawned.push((id, task));
        }

        Ok(spawned)
    }

    /// Snapshot the status of all direct children.
    pub async fn list(&self) -> Vec<ChildSnapshot> {
        let children = self.children.lock().await;
        let mut out = Vec::with_capacity(children.len());
        for entry in children.values() {
            let status = *entry.status.lock().await;
            let result = entry.result.lock().await.clone();
            out.push(ChildSnapshot {
                id: entry.id.clone(),
                task: entry.task.clone(),
                status,
                result,
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    /// Stop a single child by id.
    ///
    /// - `wait = true`: graceful — wait for the child to finish its current
    ///   work and return its full result (harvest).
    /// - `wait = false`: forceful — flip cancel + abort, return current status.
    ///
    /// In both cases all descendants are recursively reaped and temp profile
    /// directories are cleaned up. The child is removed from the registry.
    pub async fn stop(&self, id: &str, wait: bool) -> Result<StopOutcome> {
        let entry = {
            let mut children = self.children.lock().await;
            children
                .remove(id)
                .ok_or_else(|| anyhow!("no such sub-agent: {}", id))?
        };

        // First, recursively stop all of this child's own descendants.
        Box::pin(entry.child_manager.stop_all(wait)).await?;

        let outcome = if wait {
            // Graceful: let the child reach a natural stopping point.
            let handle = entry.handle.lock().await.take();
            if let Some(h) = handle {
                let _ = h.await;
            }
            let status = *entry.status.lock().await;
            let result = entry.result.lock().await.clone();
            StopOutcome {
                id: entry.id.clone(),
                status,
                result,
                forced: false,
            }
        } else {
            // Forceful: request cancellation, then abort the task.
            entry.cancel.store(true, Ordering::SeqCst);
            *entry.status.lock().await = ChildStatus::Stopped;
            let handle = entry.handle.lock().await.take();
            if let Some(h) = handle {
                h.abort();
                let _ = h.await;
            }
            let result = entry.result.lock().await.clone();
            StopOutcome {
                id: entry.id.clone(),
                status: ChildStatus::Stopped,
                result,
                forced: true,
            }
        };

        // Clean up the isolated profile directory.
        let _ = tokio::fs::remove_dir_all(&entry.profile_dir).await;

        eprintln!(
            "{}subagent{}: {} stopped ({})",
            Color::YELLOW,
            Color::RESET,
            entry.id,
            if outcome.forced { "forced" } else { "harvested" }
        );

        Ok(outcome)
    }

    /// Stop every direct child (cascades into descendants).
    pub async fn stop_all(&self, wait: bool) -> Result<Vec<StopOutcome>> {
        let ids: Vec<String> = {
            let children = self.children.lock().await;
            children.keys().cloned().collect()
        };
        let mut outcomes = Vec::with_capacity(ids.len());
        for id in ids {
            match self.stop(&id, wait).await {
                Ok(o) => outcomes.push(o),
                Err(e) => eprintln!("subagent: stop_all error for {}: {}", id, e),
            }
        }
        Ok(outcomes)
    }
}

#[derive(Debug, Clone)]
pub struct ChildSnapshot {
    pub id: String,
    pub task: String,
    pub status: ChildStatus,
    pub result: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StopOutcome {
    pub id: String,
    pub status: ChildStatus,
    pub result: Option<String>,
    pub forced: bool,
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "..."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_factory() -> Arc<AgentFactory> {
        Arc::new(AgentFactory::new(
            "key".into(),
            "https://example.invalid/v1".into(),
            "model".into(),
            "base".into(),
            PathBuf::from("/tmp/ws"),
            std::env::temp_dir().join("antlet_test_data"),
            5,
        ))
    }

    #[tokio::test]
    async fn spawn_empty_specs_errors() {
        let mgr = SubAgentManager::new_root(test_factory());
        let res = mgr.spawn(vec![]).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn list_is_empty_initially() {
        let mgr = SubAgentManager::new_root(test_factory());
        assert!(mgr.list().await.is_empty());
    }

    #[tokio::test]
    async fn stop_unknown_id_errors() {
        let mgr = SubAgentManager::new_root(test_factory());
        assert!(mgr.stop("root.99", true).await.is_err());
    }

    #[tokio::test]
    async fn stop_all_empty_ok() {
        let mgr = SubAgentManager::new_root(test_factory());
        let outcomes = mgr.stop_all(false).await.unwrap();
        assert!(outcomes.is_empty());
    }

    #[test]
    fn child_status_strings() {
        assert_eq!(ChildStatus::Running.as_str(), "running");
        assert_eq!(ChildStatus::Completed.as_str(), "completed");
        assert_eq!(ChildStatus::Failed.as_str(), "failed");
        assert_eq!(ChildStatus::Stopped.as_str(), "stopped");
    }
}
