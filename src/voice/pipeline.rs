use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::voice::capture::AudioCapture;
use crate::voice::push_to_talk::GlobalHotkey;
use crate::voice::transcription::{SttProvider, Transcript};

type TranscriptCallback = Box<dyn Fn(Transcript) + Send + 'static>;

const INTERIM_INTERVAL: Duration = Duration::from_millis(500);

pub struct VoicePipeline {
    hotkey: Arc<Mutex<Box<dyn GlobalHotkey>>>,
    capture: Arc<Mutex<AudioCapture>>,
    stt: Arc<Mutex<Box<dyn SttProvider>>>,
    running: Arc<AtomicBool>,
    on_transcript: Arc<Mutex<Option<TranscriptCallback>>>,
}

impl VoicePipeline {
    pub fn new(
        hotkey: Box<dyn GlobalHotkey>,
        capture: AudioCapture,
        stt: Box<dyn SttProvider>,
    ) -> Self {
        Self {
            hotkey: Arc::new(Mutex::new(hotkey)),
            capture: Arc::new(Mutex::new(capture)),
            stt: Arc::new(Mutex::new(stt)),
            running: Arc::new(AtomicBool::new(false)),
            on_transcript: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_on_transcript(&self, cb: TranscriptCallback) {
        *self.on_transcript.lock().unwrap() = Some(cb);
    }

    pub fn start(&self) -> anyhow::Result<()> {
        self.hotkey.lock().unwrap().register(vec!["Control", "Alt"], None)?;
        self.running.store(true, Ordering::SeqCst);

        let running = self.running.clone();
        let hotkey = self.hotkey.clone();
        let capture = self.capture.clone();
        let stt = self.stt.clone();
        let on_transcript = self.on_transcript.clone();
        let sample_rate = {
            let cap = capture.lock().unwrap();
            cap.sample_rate()
        };

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
            let mut was_pressed = false;
            let mut last_interim = Instant::now();

            while running.load(Ordering::SeqCst) {
                let pressed = hotkey.lock().unwrap().is_pressed();

                if pressed && !was_pressed {
                    let _ = capture.lock().unwrap().start();
                    last_interim = Instant::now();
                    was_pressed = true;
                } else if pressed && was_pressed {
                    if last_interim.elapsed() >= INTERIM_INTERVAL {
                        let audio = capture.lock().unwrap().snapshot();
                        if !audio.is_empty() {
                            let result = rt.block_on(async {
                                stt.lock().unwrap().transcribe(&audio, sample_rate).await
                            });
                            if let Ok(transcript) = result {
                                if let Some(ref cb) = *on_transcript.lock().unwrap() {
                                    cb(Transcript {
                                        text: transcript.text,
                                        is_final: false,
                                    });
                                }
                            }
                        }
                        last_interim = Instant::now();
                    }
                } else if !pressed && was_pressed {
                    let audio = capture.lock().unwrap().stop();
                    if !audio.is_empty() {
                        let result = rt.block_on(async {
                            stt.lock().unwrap().transcribe(&audio, sample_rate).await
                        });
                        if let Ok(transcript) = result {
                            if let Some(ref cb) = *on_transcript.lock().unwrap() {
                                cb(Transcript {
                                    text: transcript.text,
                                    is_final: true,
                                });
                            }
                        }
                    }
                    was_pressed = false;
                }

                std::thread::sleep(Duration::from_millis(10));
            }
        });

        Ok(())
    }

    pub fn stop(&self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        self.hotkey.lock().unwrap().unregister()
    }
}
