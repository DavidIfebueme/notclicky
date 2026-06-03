use anyhow::Result;
use async_trait::async_trait;
use std::pin::Pin;

use crate::voice::tts::{AudioChunk, AudioStream, TtsProvider};

pub struct CartesiaProvider {
    _api_key: String,
}

impl CartesiaProvider {
    pub fn new(api_key: String) -> Self {
        Self { _api_key: api_key }
    }
}

#[async_trait]
impl TtsProvider for CartesiaProvider {
    async fn synthesize(&self, _text: &str) -> Result<AudioChunk> {
        anyhow::bail!("Cartesia TTS not yet implemented")
    }

    async fn synthesize_stream(
        &self,
        _text_stream: Pin<Box<dyn futures::Stream<Item = String> + Send>>,
    ) -> Result<AudioStream> {
        anyhow::bail!("Cartesia TTS streaming not yet implemented")
    }
}
