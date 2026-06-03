use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::voice::transcription_whisper::WhisperSttProvider;
use whisper_rs::WhisperContext;

const WAKE_WORDS: &[&str] = &["hey clicky", "clicky", "hey clikey", "notclicky"];
const LISTEN_CHUNK_SECS: f64 = 3.0;
const RMS_THRESHOLD: f32 = 0.02;

pub struct WakeWordDetector {
    whisper: Arc<Mutex<WhisperSttProvider>>,
    sample_rate: u32,
    enabled: Arc<AtomicBool>,
    ctx_cache: Arc<Mutex<Option<Arc<WhisperContext>>>>,
}

impl WakeWordDetector {
    pub fn new(model_path: std::path::PathBuf, sample_rate: u32) -> Self {
        let whisper = WhisperSttProvider::new(model_path);
        Self {
            whisper: Arc::new(Mutex::new(whisper)),
            sample_rate,
            enabled: Arc::new(AtomicBool::new(true)),
            ctx_cache: Arc::new(Mutex::new(None)),
        }
    }

    pub fn check(&self, audio: &[f32]) -> bool {
        if !self.enabled.load(Ordering::SeqCst) {
            return false;
        }

        let rms = compute_rms(audio);
        if rms < RMS_THRESHOLD {
            return false;
        }

        let chunk_len = (self.sample_rate as f64 * LISTEN_CHUNK_SECS) as usize;
        let audio_chunk = if audio.len() > chunk_len {
            &audio[audio.len() - chunk_len..]
        } else {
            audio
        };

        let ctx = {
            let mut cache = self.ctx_cache.lock().unwrap();
            if cache.is_none() {
                let whisper = self.whisper.lock().unwrap();
                match whisper.create_context() {
                    Ok(ctx) => { *cache = Some(ctx); }
                    Err(_) => return false,
                }
            }
            cache.clone().unwrap()
        };

        match self.whisper.lock().unwrap().transcribe_sync_with_ctx(&ctx, audio_chunk) {
            Ok(text) => {
                let lower = text.to_lowercase();
                let trimmed = lower.trim();
                if trimmed.is_empty() || trimmed == "[blanks_audio]" || trimmed.contains("blank_audio") {
                    return false;
                }
                eprintln!("notclicky: heard \"{}\"", trimmed);
                for wake_word in WAKE_WORDS {
                    if lower.contains(wake_word) {
                        eprintln!("notclicky: wake word detected!");
                        return true;
                    }
                }
                false
            }
            Err(_) => false,
        }
    }

    #[allow(dead_code)]
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }
}

fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}
