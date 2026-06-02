use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub tts: TtsConfig,
    #[serde(default)]
    pub stt: SttConfig,
    #[serde(default)]
    pub bridge: BridgeConfig,
    #[serde(default)]
    pub overlay: OverlayConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_llm_provider")]
    pub provider: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default = "default_llm_model")]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TtsConfig {
    #[serde(default = "default_tts_provider")]
    pub provider: String,
    #[serde(default)]
    pub voice_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SttConfig {
    #[serde(default = "default_stt_provider")]
    pub provider: String,
    #[serde(default = "default_stt_model")]
    pub model: String,
    #[serde(default = "default_stt_language")]
    pub language: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BridgeConfig {
    #[serde(default = "default_bridge_port")]
    pub port: u16,
    #[serde(default)]
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OverlayConfig {
    #[serde(default = "default_overlay_backend")]
    pub backend: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            llm: LlmConfig::default(),
            tts: TtsConfig::default(),
            stt: SttConfig::default(),
            bridge: BridgeConfig::default(),
            overlay: OverlayConfig::default(),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_llm_provider(),
            base_url: String::new(),
            model: default_llm_model(),
            max_tokens: default_max_tokens(),
        }
    }
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            provider: default_tts_provider(),
            voice_id: String::new(),
        }
    }
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            provider: default_stt_provider(),
            model: default_stt_model(),
            language: default_stt_language(),
        }
    }
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            port: default_bridge_port(),
            token: String::new(),
        }
    }
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            backend: default_overlay_backend(),
        }
    }
}

fn default_llm_provider() -> String { "openai-compatible".into() }
fn default_llm_model() -> String { "glm-4".into() }
fn default_max_tokens() -> u32 { 4096 }
fn default_tts_provider() -> String { "edge".into() }
fn default_stt_provider() -> String { "whisper-cpp".into() }
fn default_stt_model() -> String { "base".into() }
fn default_stt_language() -> String { "en".into() }
fn default_bridge_port() -> u16 { 32123 }
fn default_overlay_backend() -> String { "x11".into() }

pub fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join("notclicky")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn load() -> Result<AppConfig> {
    let path = config_path();
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let contents = fs::read_to_string(&path)?;
    let config: AppConfig = toml::from_str(&contents)?;
    Ok(config)
}
