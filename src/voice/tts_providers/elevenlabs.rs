use anyhow::Result;
use async_trait::async_trait;
use std::pin::Pin;

use crate::voice::tts::{AudioChunk, AudioStream, TtsProvider};

pub struct ElevenLabsProvider {
    api_key: String,
    voice_id: String,
}

impl ElevenLabsProvider {
    pub fn new(api_key: String, voice_id: String) -> Self {
        Self { api_key, voice_id }
    }
}

#[async_trait]
impl TtsProvider for ElevenLabsProvider {
    async fn synthesize(&self, text: &str) -> Result<AudioChunk> {
        let client = reqwest::Client::new();
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}", self.voice_id);

        let resp = client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "text": text,
                "model_id": "eleven_monolingual_v1",
                "voice_settings": {
                    "stability": 0.5,
                    "similarity_boost": 0.75
                }
            }))
            .send()
            .await?;

        Ok(resp.bytes().await?.to_vec())
    }

    async fn synthesize_stream(
        &self,
        _text_stream: Pin<Box<dyn futures::Stream<Item = String> + Send>>,
    ) -> Result<AudioStream> {
        anyhow::bail!("ElevenLabs streaming TTS not yet implemented")
    }
}
