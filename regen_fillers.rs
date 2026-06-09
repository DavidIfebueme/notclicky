// Run with: cargo run --example regen_fillers
// Requires DEEPGRAM_API_KEY in ~/.config/notclicky/secrets.env

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const DEFINITIONS: &[(&str, &str)] = &[
    ("one_moment", "One moment."),
    ("let_me_check", "Let me check."),
    ("checking_now", "Checking now."),
    ("sure_thing", "Sure thing."),
    ("right_away", "Right away."),
];

fn load_secrets() -> HashMap<String, String> {
    let mut secrets = HashMap::new();
    let path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("notclicky")
        .join("secrets.env");
    if let Ok(content) = fs::read_to_string(&path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                secrets.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }
    secrets
}

async fn synthesize_deepgram(api_key: &str, model: &str, text: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let url = format!("https://api.deepgram.com/v1/speak?model={}&encoding=mp3", model);
    let body = serde_json::json!({ "text": text });
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Token {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Deepgram TTS error {}: {}", status, body).into());
    }

    let bytes = resp.bytes().await?;
    Ok(bytes.to_vec())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let secrets = load_secrets();
    let api_key = secrets.get("DEEPGRAM_API_KEY")
        .ok_or("DEEPGRAM_API_KEY not found in secrets.env")?;

    let sounds_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join("sounds");

    for (name, text) in DEFINITIONS {
        println!("Generating {}: \"{}\"", name, text);
        let audio = synthesize_deepgram(api_key, "aura-2-arcas-en", text).await?;
        let path = sounds_dir.join(format!("{}.mp3", name));
        fs::write(&path, audio)?;
        println!("  Saved to {}", path.display());
    }

    println!("All filler phrases regenerated!");
    Ok(())
}