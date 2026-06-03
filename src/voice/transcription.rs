use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct Transcript {
    pub text: String,
    pub _is_final: bool,
}

#[allow(dead_code)]
pub type TranscriptStream = Pin<Box<dyn Stream<Item = Result<Transcript>> + Send>>;
#[allow(dead_code)]
pub type AudioInputStream = Pin<Box<dyn Stream<Item = Vec<f32>> + Send>>;

#[async_trait]
pub trait SttProvider: Send + Sync {
    async fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<Transcript>;
    #[allow(dead_code)]
    fn supports_streaming(&self) -> bool {
        false
    }
    #[allow(dead_code)]
    async fn transcribe_stream(
        &self,
        _audio_stream: AudioInputStream,
    ) -> Result<TranscriptStream> {
        unimplemented!()
    }
}
