use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use std::pin::Pin;

use crate::voice::tts::{AudioChunk, AudioStream, TtsProvider};

pub struct DeepgramTtsProvider {
    api_key: String,
    model: String,
    client: Client,
}

impl DeepgramTtsProvider {
    pub fn with_model(api_key: String, model: String) -> Self {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { api_key, model, client }
    }
}

#[async_trait]
impl TtsProvider for DeepgramTtsProvider {
    async fn synthesize(&self, text: &str) -> Result<AudioChunk> {
        let url = format!(
            "https://api.deepgram.com/v1/speak?model={}&encoding=mp3",
            self.model
        );
        let body = serde_json::json!({ "text": text });
        let resp = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Deepgram TTS error {}: {}", status, body);
        }

        let mut audio = Vec::new();
        let mut stream = resp.bytes_stream();
        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            audio.extend_from_slice(&chunk?);
        }
        Ok(audio)
    }

    async fn synthesize_stream(
        &self,
        _text_stream: Pin<Box<dyn futures::Stream<Item = String> + Send>>,
    ) -> Result<AudioStream> {
        anyhow::bail!("Deepgram TTS streaming not implemented — use sentence-level synthesize instead")
    }
}
