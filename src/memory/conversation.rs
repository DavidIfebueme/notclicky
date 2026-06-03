use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_EXCHANGES: usize = 8;
const ARCHIVE_THRESHOLD: usize = 2400;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exchange {
    pub user: String,
    pub assistant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationHistory {
    exchanges: Vec<Exchange>,
    archive: String,
}

impl ConversationHistory {
    pub fn new() -> Self {
        Self {
            exchanges: Vec::new(),
            archive: String::new(),
        }
    }

    pub fn add(&mut self, user: String, assistant: String) {
        self.exchanges.push(Exchange { user, assistant });
        self.compact_if_needed();
    }

    pub fn exchanges(&self) -> &[Exchange] {
        &self.exchanges
    }

    pub fn archive(&self) -> &str {
        &self.archive
    }

    pub fn to_prompt_context(&self) -> String {
        let mut parts = Vec::new();

        if !self.archive.is_empty() {
            parts.push(format!("[Earlier conversation summary]\n{}", self.archive));
        }

        for exchange in &self.exchanges {
            parts.push(format!("User: {}", exchange.user));
            parts.push(format!("Assistant: {}", exchange.assistant));
        }

        parts.join("\n\n")
    }

    fn compact_if_needed(&mut self) {
        if self.exchanges.len() <= MAX_EXCHANGES {
            return;
        }

        let current_chars: usize = self.exchanges.iter().map(|e| e.user.len() + e.assistant.len()).sum();

        if current_chars <= ARCHIVE_THRESHOLD {
            return;
        }

        while self.exchanges.len() > MAX_EXCHANGES / 2 {
            let oldest = self.exchanges.remove(0);
            if !self.archive.is_empty() {
                self.archive.push('\n');
            }
            self.archive.push_str(&format!("User: {} | Assistant: {}", oldest.user, oldest.assistant));
        }

        if self.archive.len() > ARCHIVE_THRESHOLD * 2 {
            self.archive = self.archive[self.archive.len() - ARCHIVE_THRESHOLD..].to_string();
            if let Some(pos) = self.archive.find('\n') {
                self.archive = self.archive[pos + 1..].to_string();
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentMemory {
    content: String,
    path: PathBuf,
}

impl PersistentMemory {
    pub fn new(path: PathBuf) -> Self {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        Self { content, path }
    }

    pub fn get(&self) -> &str {
        &self.content
    }

    pub fn set(&mut self, content: String) -> Result<()> {
        self.content = content;
        self.save()
    }

    pub fn append(&mut self, text: &str) -> Result<()> {
        if !self.content.is_empty() {
            self.content.push('\n');
        }
        self.content.push_str(text);
        self.save()
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, &self.content)?;
        Ok(())
    }
}
