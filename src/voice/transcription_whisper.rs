use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::transcription::{SttProvider, Transcript};

pub struct WhisperSttProvider {
    model_path: PathBuf,
}

impl WhisperSttProvider {
    pub fn new(model_path: PathBuf) -> Self {
        Self { model_path }
    }

    pub fn transcribe_sync(&self, audio: &[f32]) -> Result<String> {
        let ctx = WhisperContext::new_with_params(&self.model_path, WhisperContextParameters::default())?;
        let mut state = ctx.create_state()?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_print_progress(false);
        params.set_print_timestamps(false);
        params.set_print_special(false);
        state.full(params, audio)?;

        let text: String = state
            .as_iter()
            .filter_map(|seg| seg.to_str().ok())
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string();

        Ok(text)
    }
}

#[async_trait]
impl SttProvider for WhisperSttProvider {
    async fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<Transcript> {
        let audio = if sample_rate != 16000 {
            crate::voice::resample::resample(audio, sample_rate, 16000)
        } else {
            audio.to_vec()
        };

        let text = self.transcribe_sync(&audio)?;
        Ok(Transcript {
            text,
            is_final: true,
        })
    }
}
