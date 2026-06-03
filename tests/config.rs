use notclicky::app::AppConfig;

#[test]
fn parse_sample_config() {
    let sample = r#"
[llm]
provider = "openai-compatible"
base_url = "https://api.zai.com/v1"
model = "glm-4"
max_tokens = 4096

[tts]
provider = "edge"
voice_id = ""

[stt]
provider = "whisper-cpp"
model = "base"
language = "en"

[bridge]
port = 32123
token = ""

[overlay]
backend = "x11"
"#;

    let config: AppConfig = toml::from_str(sample).unwrap();
    assert_eq!(config.llm.provider, "openai-compatible");
    assert_eq!(config.llm.base_url, "https://api.zai.com/v1");
    assert_eq!(config.llm.model, "glm-4");
    assert_eq!(config.llm._max_tokens, 4096);
    assert_eq!(config.tts.provider, "edge");
    assert_eq!(config.stt.provider, "whisper-cpp");
    assert_eq!(config.stt.model, "base");
    assert_eq!(config.stt.language, "en");
    assert_eq!(config.bridge.port, 32123);
    assert_eq!(config.overlay.backend, "x11");
}

#[test]
fn default_config_values() {
    let config = AppConfig::default();
    assert_eq!(config.llm.provider, "openai-compatible");
    assert_eq!(config.tts.provider, "edge");
    assert_eq!(config.stt.provider, "whisper-cpp");
    assert_eq!(config.bridge.port, 32123);
    assert_eq!(config.overlay.backend, "x11");
}

#[test]
fn partial_config_uses_defaults() {
    let sample = r#"
[llm]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
"#;

    let config: AppConfig = toml::from_str(sample).unwrap();
    assert_eq!(config.llm.provider, "anthropic");
    assert_eq!(config.llm.model, "claude-sonnet-4-20250514");
    assert_eq!(config.tts.provider, "edge");
    assert_eq!(config.stt.provider, "whisper-cpp");
    assert_eq!(config.bridge.port, 32123);
}
