use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::ai::providers::LlmProvider;
use crate::ai::streaming_pipeline::SentenceStream;
use crate::agent::process::AgentManager;
use crate::screen::capture::ScreenCapture;
use crate::voice::capture::AudioCapture;
use crate::voice::push_to_talk::GlobalHotkey;
use crate::voice::transcription::{SttProvider, Transcript};
use crate::voice::tts::TtsProvider;
use crate::voice::wake_word::WakeWordDetector;

type TranscriptCallback = Box<dyn Fn(Transcript) + Send + 'static>;

pub struct VoiceAssistant {
    hotkey: Arc<Mutex<Box<dyn GlobalHotkey>>>,
    capture: Arc<Mutex<AudioCapture>>,
    stt: Arc<Mutex<Box<dyn SttProvider>>>,
    llm: Arc<Mutex<Box<dyn LlmProvider>>>,
    tts: Arc<Mutex<Box<dyn TtsProvider>>>,
    screen: Arc<Mutex<Box<dyn ScreenCapture>>>,
    agent_manager: Arc<Mutex<Option<AgentManager>>>,
    system_prompt: String,
    running: Arc<AtomicBool>,
    on_transcript: Arc<Mutex<Option<TranscriptCallback>>>,
    wake_word: Option<WakeWordDetector>,
}

impl VoiceAssistant {
    pub fn new(
        hotkey: Box<dyn GlobalHotkey>,
        capture: AudioCapture,
        stt: Box<dyn SttProvider>,
        llm: Box<dyn LlmProvider>,
        tts: Box<dyn TtsProvider>,
        screen: Box<dyn ScreenCapture>,
        system_prompt: String,
    ) -> Self {
        Self {
            hotkey: Arc::new(Mutex::new(hotkey)),
            capture: Arc::new(Mutex::new(capture)),
            stt: Arc::new(Mutex::new(stt)),
            llm: Arc::new(Mutex::new(llm)),
            tts: Arc::new(Mutex::new(tts)),
            screen: Arc::new(Mutex::new(screen)),
            agent_manager: Arc::new(Mutex::new(None)),
            system_prompt,
            running: Arc::new(AtomicBool::new(false)),
            on_transcript: Arc::new(Mutex::new(None)),
            wake_word: None,
        }
    }

    pub fn set_wake_word(&mut self, detector: WakeWordDetector) {
        self.wake_word = Some(detector);
    }

    #[allow(dead_code)]
    pub fn set_on_transcript(&self, cb: TranscriptCallback) {
        *self.on_transcript.lock().unwrap() = Some(cb);
    }

    pub fn set_agent_manager(&self, manager: AgentManager) {
        *self.agent_manager.lock().unwrap() = Some(manager);
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        self.hotkey.lock().unwrap().register(vec!["Control", "Alt"], None)?;
        self.running.store(true, Ordering::SeqCst);

        let has_wake_word = self.wake_word.is_some();
        if has_wake_word {
            let _ = self.capture.lock().unwrap().start();
            eprintln!("notclicky: wake word listening enabled (say \"hey clicky\")");
        }

        let running = self.running.clone();
        let hotkey = self.hotkey.clone();
        let capture = self.capture.clone();
        let stt = self.stt.clone();
        let llm = self.llm.clone();
        let tts = self.tts.clone();
        let screen = self.screen.clone();
        let system_prompt = self.system_prompt.clone();
        let on_transcript = self.on_transcript.clone();
        let agent_manager = self.agent_manager.clone();
        let sample_rate = capture.lock().unwrap().sample_rate();
        let wake_word = self.wake_word.take();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
            let mut was_pressed = false;
            let mut last_wake_word_time: Option<std::time::Instant> = None;
            let mut wake_word_check_counter: u64 = 0;

            while running.load(Ordering::SeqCst) {
                let pressed = hotkey.lock().unwrap().is_pressed();

                if pressed && !was_pressed {
                    let _ = capture.lock().unwrap().start();
                    was_pressed = true;
                } else if !pressed && was_pressed {
                    let audio = capture.lock().unwrap().stop();
                    let screenshot = rt.block_on(async {
                        screen.lock().unwrap().capture_cursor_screen().await.ok()
                    });

                    if !audio.is_empty() {
                        let _ = process_audio(
                            &audio, sample_rate, &stt, &llm, &tts, &system_prompt,
                            &on_transcript, &agent_manager, screenshot.as_ref(), &rt,
                        );
                    }

                    was_pressed = false;
                    if has_wake_word {
                        let _ = capture.lock().unwrap().start();
                    }
                }

                if !was_pressed && has_wake_word {
                    wake_word_check_counter += 1;
                    let in_cooldown = last_wake_word_time.map_or(false, |t| t.elapsed() < std::time::Duration::from_secs(3));
                    if wake_word_check_counter % 200 == 0 && !in_cooldown {
                        if let Some(ref ww) = wake_word {
                            let chunk_len = (sample_rate as f64 * 3.0) as usize;
                            let recent = {
                                let cap = capture.lock().unwrap();
                                let buf = cap.snapshot();
                                if buf.len() > chunk_len * 2 {
                                    cap.trim_to(chunk_len);
                                }
                                if buf.len() > chunk_len {
                                    buf[buf.len() - chunk_len..].to_vec()
                                } else {
                                    buf
                                }
                            };
                            if !recent.is_empty() && ww.check(&recent) {
                                last_wake_word_time = Some(std::time::Instant::now());
                                let audio = capture.lock().unwrap().stop();
                                eprintln!("notclicky: wake word triggered, processing command...");

                                let screenshot = rt.block_on(async {
                                    screen.lock().unwrap().capture_cursor_screen().await.ok()
                                });

                                if !audio.is_empty() {
                                    let _ = process_audio(
                                        &audio, sample_rate, &stt, &llm, &tts, &system_prompt,
                                        &on_transcript, &agent_manager, screenshot.as_ref(), &rt,
                                    );
                                }

                                let _ = capture.lock().unwrap().start();
                            }
                        }
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });

        Ok(())
    }

    #[allow(dead_code)]
    pub fn stop(&self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        self.hotkey.lock().unwrap().unregister()
    }
}

fn process_audio(
    audio: &[f32],
    sample_rate: u32,
    stt: &Arc<Mutex<Box<dyn SttProvider>>>,
    llm: &Arc<Mutex<Box<dyn LlmProvider>>>,
    tts: &Arc<Mutex<Box<dyn TtsProvider>>>,
    system_prompt: &str,
    on_transcript: &Arc<Mutex<Option<TranscriptCallback>>>,
    agent_manager: &Arc<Mutex<Option<AgentManager>>>,
    screenshot: Option<&crate::screen::capture::CaptureResult>,
    rt: &tokio::runtime::Runtime,
) -> anyhow::Result<()> {
    let transcript_result = rt.block_on(async {
        stt.lock().unwrap().transcribe(audio, sample_rate).await
    });

    if let Ok(transcript) = transcript_result {
        if let Some(ref cb) = *on_transcript.lock().unwrap() {
            cb(Transcript {
                text: transcript.text.clone(),
                _is_final: true,
            });
        }

        if !transcript.text.trim().is_empty() {
            if is_agent_request(&transcript.text) {
                let agent_prompt = strip_agent_keyword(&transcript.text);
                let mgr = agent_manager.lock().unwrap();
                if let Some(ref mgr) = *mgr {
                    let _ = rt.block_on(mgr.spawn(agent_prompt, None, None));
                }
            } else {
                let _ = rt.block_on(process_response(
                    llm, tts, system_prompt, &transcript.text, screenshot,
                ));
            }
        }
    }

    Ok(())
}

async fn process_response(
    llm: &Arc<Mutex<Box<dyn LlmProvider>>>,
    tts: &Arc<Mutex<Box<dyn TtsProvider>>>,
    system_prompt: &str,
    user_text: &str,
    screenshot: Option<&crate::screen::capture::CaptureResult>,
) -> anyhow::Result<()> {
    let user_content = if let Some(screenshot) = screenshot {
        let b64 = base64_encode(&screenshot.image_data);
        format!("{}\n\n[data:image/jpeg;base64,{}]", user_text, b64)
    } else {
        user_text.to_string()
    };

    let req = crate::ai::providers::LlmRequest {
        messages: vec![
            crate::ai::providers::LlmMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            crate::ai::providers::LlmMessage {
                role: "user".to_string(),
                content: user_content,
            },
        ],
        model: None,
        max_tokens: None,
        temperature: None,
    };

    let stream = llm.lock().unwrap().stream(req).await?;
    let mut sentence_stream = SentenceStream::new(stream);
    let tts_provider = tts.lock().unwrap();

    while let Some(sentence) = futures::StreamExt::next(&mut sentence_stream).await {
        if !sentence.is_empty() {
            match tts_provider.synthesize(&sentence).await {
                Ok(audio_data) => {
                    let _ = play_audio(&audio_data);
                }
                Err(e) => eprintln!("TTS error: {}", e),
            }
        }
    }

    Ok(())
}

fn play_audio(data: &[u8]) -> anyhow::Result<()> {
    let cursor = std::io::Cursor::new(data.to_vec());
    let source = rodio::Decoder::new(cursor)?;
    let (_stream, stream_handle) = rodio::OutputStream::try_default()?;
    let sink = rodio::Sink::try_new(&stream_handle)?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}

fn base64_encode(data: &[u8]) -> String {
    use std::fmt::Write;
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.write_char(TABLE[((triple >> 18) & 0x3F) as usize] as char).unwrap();
        out.write_char(TABLE[((triple >> 12) & 0x3F) as usize] as char).unwrap();
        if chunk.len() > 1 {
            out.write_char(TABLE[((triple >> 6) & 0x3F) as usize] as char).unwrap();
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.write_char(TABLE[(triple & 0x3F) as usize] as char).unwrap();
        } else {
            out.push('=');
        }
    }
    out
}

pub fn is_agent_request(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.starts_with("agent ") || lower.starts_with("clicky agent ") || lower.contains(" agent,") || lower.contains(" agent:")
}

pub fn strip_agent_keyword(text: &str) -> String {
    let lower = text.to_lowercase();
    if lower.starts_with("clicky agent ") {
        text["clicky agent ".len()..].to_string()
    } else if lower.starts_with("agent ") {
        text["agent ".len()..].to_string()
    } else {
        text.to_string()
    }
}
