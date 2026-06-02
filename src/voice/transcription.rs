use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct Transcript {
    pub text: String,
    pub is_final: bool,
}

pub type TranscriptStream = Pin<Box<dyn Stream<Item = Result<Transcript>> + Send>>;
pub type AudioInputStream = Pin<Box<dyn Stream<Item = Vec<f32>> + Send>>;

#[async_trait]
pub trait SttProvider: Send + Sync {
    async fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<Transcript>;
    fn supports_streaming(&self) -> bool {
        false
    }
    async fn transcribe_stream(
        &self,
        _audio_stream: AudioInputStream,
    ) -> Result<TranscriptStream> {
        unimplemented!()
    }
}
