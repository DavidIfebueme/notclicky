use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

pub type AudioChunk = Vec<u8>;
pub type AudioStream = Pin<Box<dyn Stream<Item = Result<AudioChunk>> + Send>>;

#[async_trait]
pub trait TtsProvider: Send + Sync {
    async fn synthesize(&self, text: &str) -> Result<AudioChunk>;
    async fn synthesize_stream(
        &self,
        text_stream: Pin<Box<dyn Stream<Item = String> + Send>>,
    ) -> Result<AudioStream>;
}
