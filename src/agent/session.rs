use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

static AGENT_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Starting,
    Running,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: String,
    pub status: AgentStatus,
    pub prompt: String,
    pub output: String,
    pub working_dir: String,
    pub model: Option<String>,
    pub created_at: u64,
}

impl AgentSession {
    pub fn new(prompt: String, working_dir: String, model: Option<String>) -> Self {
        let count = AGENT_COUNTER.fetch_add(1, Ordering::Relaxed);
        let id = format!(
            "agent-{}-{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            count
        );
        Self {
            id,
            status: AgentStatus::Starting,
            prompt,
            output: String::new(),
            working_dir,
            model,
            created_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentEvent {
    pub session_id: String,
    pub kind: AgentEventKind,
}

#[derive(Debug, Clone)]
pub enum AgentEventKind {
    Output { delta: String },
    StatusChanged { status: AgentStatus },
    Done,
    Failed { error: String },
}
