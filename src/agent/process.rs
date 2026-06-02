use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, watch};

use super::session::{AgentEvent, AgentEventKind, AgentSession, AgentStatus};

pub struct AgentProcess {
    child: Child,
    session_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum OpencodeEvent {
    #[serde(rename = "message")]
    Message { content: Option<String> },
    #[serde(rename = "text")]
    Text { content: String },
    #[serde(rename = "assistant")]
    Assistant { content: Option<String> },
    #[serde(rename = "tool")]
    Tool { name: Option<String> },
    #[serde(rename = "result")]
    Result { content: Option<String> },
    #[serde(rename = "error")]
    Error { message: Option<String> },
    #[serde(rename = "done")]
    Done,
}

pub struct AgentManager {
    sessions: Arc<Mutex<HashMap<String, AgentSession>>>,
    processes: Arc<Mutex<HashMap<String, AgentProcess>>>,
    event_tx: watch::Sender<Option<AgentEvent>>,
    event_rx: watch::Receiver<Option<AgentEvent>>,
    opencode_path: String,
    home_dir: PathBuf,
}

impl AgentManager {
    pub fn new(home_dir: PathBuf) -> Self {
        let (event_tx, event_rx) = watch::channel(None);
        let opencode_path = std::env::var("OPENCODE_PATH").unwrap_or_else(|_| "opencode".to_string());
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            processes: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            event_rx,
            opencode_path,
            home_dir,
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<Option<AgentEvent>> {
        self.event_rx.clone()
    }

    pub async fn list_sessions(&self) -> Vec<AgentSession> {
        let sessions = self.sessions.lock().await;
        sessions.values().cloned().collect()
    }

    pub async fn get_session(&self, id: &str) -> Option<AgentSession> {
        let sessions = self.sessions.lock().await;
        sessions.get(id).cloned()
    }

    pub async fn spawn(&self, prompt: String, working_dir: Option<String>, model: Option<String>) -> Result<String> {
        let dir = working_dir.unwrap_or_else(|| self.home_dir.display().to_string());
        self.setup_agent_home(&dir)?;

        let mut session = AgentSession::new(prompt.clone(), dir.clone(), model.clone());
        let session_id = session.id.clone();

        self.sessions.lock().await.insert(session_id.clone(), session);

        let mut cmd = Command::new(&self.opencode_path);
        cmd.arg("run")
            .arg("--format").arg("json")
            .arg("--dir").arg(&dir)
            .arg(&prompt)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if let Some(ref m) = model {
            cmd.arg("--model").arg(m);
        }

        let child = cmd.spawn()?;
        let process = AgentProcess {
            child,
            session_id: session_id.clone(),
        };
        self.processes.lock().await.insert(session_id.clone(), process);

        self.update_status(&session_id, AgentStatus::Running).await;
        self.spawn_reader(session_id.clone());

        Ok(session_id)
    }

    fn setup_agent_home(&self, dir: &str) -> Result<()> {
        let dir_path = std::path::Path::new(dir);
        std::fs::create_dir_all(dir_path)?;

        let resources_dir = self.home_dir.join("resources");
        if resources_dir.exists() {
            for entry in std::fs::read_dir(&resources_dir)? {
                let entry = entry?;
                let dest = dir_path.join(entry.file_name());
                if !dest.exists() {
                    std::fs::copy(entry.path(), dest)?;
                }
            }
        }

        let soul_md = self.home_dir.join("SOUL.md");
        if soul_md.exists() {
            let dest = dir_path.join("SOUL.md");
            if !dest.exists() {
                std::fs::copy(&soul_md, &dest)?;
            }
        }

        let instructions = self.home_dir.join("ModelInstructions.md");
        if instructions.exists() {
            let dest = dir_path.join("ModelInstructions.md");
            if !dest.exists() {
                std::fs::copy(&instructions, &dest)?;
            }
        }

        let agents_md = self.home_dir.join("AGENTS.md");
        if agents_md.exists() {
            let dest = dir_path.join("AGENTS.md");
            if !dest.exists() {
                std::fs::copy(&agents_md, &dest)?;
            }
        }

        Ok(())
    }

    fn spawn_reader(&self, session_id: String) {
        let sessions = self.sessions.clone();
        let event_tx = self.event_tx.clone();
        let processes = self.processes.clone();
        let home_dir = self.home_dir.clone();

        tokio::spawn(async move {
            let stdout = {
                let mut procs = processes.lock().await;
                if let Some(proc) = procs.get_mut(&session_id) {
                    match proc.child.stdout.take() {
                        Some(out) => out,
                        None => return,
                    }
                } else {
                    return;
                }
            };

            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                let delta = match serde_json::from_str::<serde_json::Value>(&line) {
                    Ok(val) => extract_text(&val),
                    Err(_) => line,
                };

                if !delta.is_empty() {
                    {
                        let mut sessions = sessions.lock().await;
                        if let Some(session) = sessions.get_mut(&session_id) {
                            session.output.push_str(&delta);
                        }
                    }
                    let _ = event_tx.send(Some(AgentEvent {
                        session_id: session_id.clone(),
                        kind: AgentEventKind::Output { delta },
                    }));
                }
            }

            let status = {
                let mut procs = processes.lock().await;
                if let Some(mut proc) = procs.remove(&session_id) {
                    match proc.child.wait().await {
                        Ok(exit) if exit.success() => AgentStatus::Done,
                        _ => AgentStatus::Failed,
                    }
                } else {
                    AgentStatus::Done
                }
            };

            {
                let mut sessions = sessions.lock().await;
                if let Some(session) = sessions.get_mut(&session_id) {
                    session.status = status;
                }
            }

            let kind = match status {
                AgentStatus::Done => {
                    play_agent_done_sound(&home_dir).await;
                    AgentEventKind::Done
                }
                AgentStatus::Failed => AgentEventKind::Failed { error: "Process exited with error".to_string() },
                _ => AgentEventKind::Done,
            };
            let _ = event_tx.send(Some(AgentEvent {
                session_id: session_id.clone(),
                kind,
            }));
        });
    }

    async fn update_status(&self, session_id: &str, status: AgentStatus) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = status;
        }
        let _ = self.event_tx.send(Some(AgentEvent {
            session_id: session_id.to_string(),
            kind: AgentEventKind::StatusChanged { status },
        }));
    }

    pub async fn kill(&self, session_id: &str) -> Result<()> {
        let mut procs = self.processes.lock().await;
        if let Some(mut proc) = procs.remove(session_id) {
            proc.child.kill().await?;
        }
        self.update_status(session_id, AgentStatus::Failed).await;
        Ok(())
    }
}

fn extract_text(val: &serde_json::Value) -> String {
    if let Some(content) = val.get("content").and_then(|c| c.as_str()) {
        return content.to_string();
    }
    if let Some(text) = val.get("text").and_then(|t| t.as_str()) {
        return text.to_string();
    }
    if let Some(delta) = val.get("delta").and_then(|d| d.as_str()) {
        return delta.to_string();
    }
    String::new()
}

pub async fn play_agent_done_sound(home_dir: &std::path::Path) {
    let candidates = vec![
        home_dir.join("resources").join("agent-done.mp3"),
        std::path::PathBuf::from("resources/agent-done.mp3"),
        std::path::PathBuf::from("/usr/share/notclicky/agent-done.mp3"),
    ];

    let sound_path = match candidates.into_iter().find(|p| p.exists()) {
        Some(p) => p,
        None => return,
    };

    let _ = std::process::Command::new("paplay")
        .arg(&sound_path)
        .spawn();
}
