use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::voice::transcription_deepgram::DeepgramSttProvider;

const WAKE_WORDS: &[&str] = &["hey clicky", "clicky", "hey clikey", "notclicky"];
const LISTEN_CHUNK_SECS: f64 = 3.0;
const RMS_THRESHOLD: f32 = 0.02;

pub struct WakeWordDetector {
    deepgram: Arc<Mutex<DeepgramSttProvider>>,
    sample_rate: u32,
    enabled: Arc<AtomicBool>,
}

impl WakeWordDetector {
    pub fn new(api_key: String, sample_rate: u32) -> Self {
        let deepgram = DeepgramSttProvider::new(api_key);
        Self {
            deepgram: Arc::new(Mutex::new(deepgram)),
            sample_rate,
            enabled: Arc::new(AtomicBool::new(true)),
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

        let deepgram = self.deepgram.lock().unwrap();
        match deepgram.transcribe_sync(audio_chunk, self.sample_rate) {
            Ok(text) => {
                let lower = text.to_lowercase();
                let trimmed = lower.trim();
                if trimmed.is_empty() {
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
            Err(e) => {
                eprintln!("notclicky: deepgram stt error: {}", e);
                false
            }
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
