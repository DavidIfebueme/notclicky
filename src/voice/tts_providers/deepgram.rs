use anyhow::Result;
use async_trait::async_trait;
use std::pin::Pin;

use crate::voice::tts::{AudioChunk, AudioStream, TtsProvider};

pub struct DeepgramTtsProvider {
    _api_key: String,
}

impl DeepgramTtsProvider {
    pub fn new(api_key: String) -> Self {
        Self { _api_key: api_key }
    }
}

#[async_trait]
impl TtsProvider for DeepgramTtsProvider {
    async fn synthesize(&self, _text: &str) -> Result<AudioChunk> {
        anyhow::bail!("Deepgram TTS not yet implemented")
    }

    async fn synthesize_stream(
        &self,
        _text_stream: Pin<Box<dyn futures::Stream<Item = String> + Send>>,
    ) -> Result<AudioStream> {
        anyhow::bail!("Deepgram TTS streaming not yet implemented")
    }
}
