use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::ai::providers::LlmProvider;
use crate::ai::streaming_pipeline::SentenceStream;
use crate::agent::process::AgentManager;
use crate::screen::capture::ScreenCapture;
use crate::voice::audio_player::AudioPlayer;
use crate::voice::capture::AudioCapture;
use crate::voice::filler::FillerLibrary;
use crate::voice::push_to_talk::GlobalHotkey;
use crate::voice::transcription::SttProvider;
use crate::voice::transcription_deepgram::{DeepgramSttProvider, DeepgramStreamingSession};
use crate::voice::tts::TtsProvider;

type TranscriptCallback = Box<dyn Fn(String) + Send + 'static>;

const FILLER_DELAY_MS: u64 = 400;

pub struct VoiceAssistant {
    hotkey: Arc<Mutex<Box<dyn GlobalHotkey>>>,
    capture: Arc<Mutex<AudioCapture>>,
    stt: Arc<Mutex<Box<dyn SttProvider>>>,
    deepgram_api_key: Option<String>,
    llm: Arc<Mutex<Box<dyn LlmProvider>>>,
    tts: Arc<Mutex<Box<dyn TtsProvider>>>,
    screen: Arc<Mutex<Box<dyn ScreenCapture>>>,
    agent_manager: Arc<Mutex<Option<AgentManager>>>,
    system_prompt: String,
    running: Arc<AtomicBool>,
    on_transcript: Arc<Mutex<Option<TranscriptCallback>>>,
    wake_word_enabled: bool,
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
            deepgram_api_key: None,
            llm: Arc::new(Mutex::new(llm)),
            tts: Arc::new(Mutex::new(tts)),
            screen: Arc::new(Mutex::new(screen)),
            agent_manager: Arc::new(Mutex::new(None)),
            system_prompt,
            running: Arc::new(AtomicBool::new(false)),
            on_transcript: Arc::new(Mutex::new(None)),
            wake_word_enabled: false,
        }
    }

    pub fn set_deepgram_api_key(&mut self, key: String) {
        self.deepgram_api_key = Some(key);
    }

    pub fn set_wake_word_enabled(&mut self, enabled: bool) {
        self.wake_word_enabled = enabled;
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

        if self.wake_word_enabled {
            let _ = self.capture.lock().unwrap().start();
            eprintln!("notclicky: wake word listening enabled (say \"hey clicky\")");
        }

        let running = self.running.clone();
        let hotkey = self.hotkey.clone();
        let capture = self.capture.clone();
        let stt = self.stt.clone();
        let deepgram_api_key = self.deepgram_api_key.clone();
        let llm = self.llm.clone();
        let tts = self.tts.clone();
        let screen = self.screen.clone();
        let system_prompt = self.system_prompt.clone();
        let on_transcript = self.on_transcript.clone();
        let agent_manager = self.agent_manager.clone();
        let sample_rate = capture.lock().unwrap().sample_rate();
        let wake_word_enabled = self.wake_word_enabled;
        let filler_library = FillerLibrary::load();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("voice runtime");
            rt.block_on(pipeline_loop(
                running, hotkey, capture, stt, deepgram_api_key,
                llm, tts, screen, system_prompt, on_transcript,
                agent_manager, sample_rate, wake_word_enabled, filler_library,
            ));
        });

        Ok(())
    }

    #[allow(dead_code)]
    pub fn stop(&self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        self.hotkey.lock().unwrap().unregister()
    }
}

async fn pipeline_loop(
    running: Arc<AtomicBool>,
    hotkey: Arc<Mutex<Box<dyn GlobalHotkey>>>,
    capture: Arc<Mutex<AudioCapture>>,
    stt: Arc<Mutex<Box<dyn SttProvider>>>,
    deepgram_api_key: Option<String>,
    llm: Arc<Mutex<Box<dyn LlmProvider>>>,
    tts: Arc<Mutex<Box<dyn TtsProvider>>>,
    screen: Arc<Mutex<Box<dyn ScreenCapture>>>,
    system_prompt: String,
    on_transcript: Arc<Mutex<Option<TranscriptCallback>>>,
    agent_manager: Arc<Mutex<Option<AgentManager>>>,
    sample_rate: u32,
    wake_word_enabled: bool,
    filler_library: FillerLibrary,
) {
    let mut was_pressed = false;
    let mut last_wake_word_time: Option<std::time::Instant> = None;
    let mut wake_word_counter: u64 = 0;

    while running.load(Ordering::SeqCst) {
        let pressed = hotkey.lock().unwrap().is_pressed();

        if pressed && !was_pressed {
            let _ = capture.lock().unwrap().start();
            was_pressed = true;

            let screen_c = screen.clone();
            tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let _ = rt.block_on(screen_c.lock().unwrap().capture_cursor_screen());
            });
        }

        if !pressed && was_pressed {
            was_pressed = false;

            let audio = capture.lock().unwrap().stop();
            if !audio.is_empty() {
                let transcript = transcribe(&audio, sample_rate, &stt, &deepgram_api_key).await;

                if !transcript.trim().is_empty() {
                    if let Some(ref cb) = *on_transcript.lock().unwrap() {
                        cb(transcript.clone());
                    }

                    if is_agent_request(&transcript) {
                        let prompt = strip_agent_keyword(&transcript);
                        let mgr = agent_manager.lock().unwrap();
                        if let Some(ref m) = *mgr {
                            let _ = m.spawn(prompt, None, None).await;
                        }
                    } else {
                        let screen_c = screen.clone();
                        let screenshot = tokio::task::spawn_blocking(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(screen_c.lock().unwrap().capture_cursor_screen())
                        }).await.unwrap().ok();
                        let llm_c = llm.clone();
                        let tts_c = tts.clone();
                        let sys = system_prompt.clone();
                        let lib = filler_library.clone();
                        let text = transcript.clone();

                        std::thread::spawn(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            let _ = rt.block_on(respond_with_pipeline(
                                &llm_c, &tts_c, &sys, &text,
                                screenshot.as_ref(), &lib,
                            ));
                        });
                    }
                }
            }

            if wake_word_enabled {
                let _ = capture.lock().unwrap().start();
            }
        }

        if !was_pressed && wake_word_enabled {
            wake_word_counter += 1;
            let in_cooldown = last_wake_word_time
                .map_or(false, |t| t.elapsed() < std::time::Duration::from_secs(3));
            if wake_word_counter % 200 == 0 && !in_cooldown {
                if let Some(ref key) = deepgram_api_key {
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
                    if !recent.is_empty() {
                        let provider = DeepgramSttProvider::new(key.clone());
                        let ds = downsample(&recent, sample_rate, 8000);
                        match provider.transcribe_sync(&ds, 8000) {
                            Ok(text) => {
                                let lower = text.to_lowercase();
                                let trimmed = lower.trim();
                                if !trimmed.is_empty() {
                                    eprintln!("notclicky: heard \"{}\"", trimmed);
                                }
                                let ww = ["hey clicky", "clicky", "hey clikey", "not clicky", "notclicky"];
                                if ww.iter().any(|w| lower.contains(w)) {
                                    eprintln!("notclicky: wake word detected!");
                                    last_wake_word_time = Some(std::time::Instant::now());
                                    let audio = capture.lock().unwrap().stop();
                                    eprintln!("notclicky: wake word triggered, processing command...");

                                    let screen_c = screen.clone();
                                    let screenshot = tokio::task::spawn_blocking(move || {
                                        let rt = tokio::runtime::Runtime::new().unwrap();
                                        rt.block_on(screen_c.lock().unwrap().capture_cursor_screen())
                                    }).await.unwrap().ok();
                                    let transcript = transcribe(&audio, sample_rate, &stt, &deepgram_api_key).await;

                                    if !transcript.trim().is_empty() {
                                        let llm_c = llm.clone();
                                        let tts_c = tts.clone();
                                        let sys = system_prompt.clone();
                                        let lib = filler_library.clone();
                                        std::thread::spawn(move || {
                                            let rt = tokio::runtime::Runtime::new().unwrap();
                                            let _ = rt.block_on(respond_with_pipeline(
                                                &llm_c, &tts_c, &sys, &transcript,
                                                screenshot.as_ref(), &lib,
                                            ));
                                        });
                                    }

                                    let _ = capture.lock().unwrap().start();
                                }
                            }
                            Err(_) => {}
                        }
                    }
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

async fn transcribe(
    audio: &[f32],
    sample_rate: u32,
    stt: &Arc<Mutex<Box<dyn SttProvider>>>,
    deepgram_api_key: &Option<String>,
) -> String {
    if audio.is_empty() {
        return String::new();
    }

    if let Some(key) = deepgram_api_key {
        match DeepgramStreamingSession::connect(key).await {
            Ok(mut session) => {
                if session.send_audio(audio).await.is_err() {
                    return DeepgramSttProvider::new(key.clone())
                        .transcribe_sync(audio, sample_rate)
                        .unwrap_or_default();
                }
                let _ = session.finalize().await;
                let mut result = String::new();
                while let Some(t) = session.next_transcript().await {
                    match t {
                        Ok(t) if t.is_final && !t.text.is_empty() => {
                            result = t.text;
                        }
                        Ok(t) if t.is_utterance_end => break,
                        _ => {}
                    }
                }
                let _ = session.close().await;
                if result.is_empty() {
                    DeepgramSttProvider::new(key.clone())
                        .transcribe_sync(audio, sample_rate)
                        .unwrap_or_default()
                } else {
                    result
                }
            }
            Err(_) => {
                DeepgramSttProvider::new(deepgram_api_key.clone().unwrap())
                    .transcribe_sync(audio, sample_rate)
                    .unwrap_or_default()
            }
        }
    } else {
        stt.lock().unwrap().transcribe(audio, sample_rate)
            .await
            .map(|t| t.text)
            .unwrap_or_default()
    }
}

async fn respond_with_pipeline(
    llm: &Arc<Mutex<Box<dyn LlmProvider>>>,
    tts: &Arc<Mutex<Box<dyn TtsProvider>>>,
    system_prompt: &str,
    user_text: &str,
    screenshot: Option<&crate::screen::capture::CaptureResult>,
    filler_library: &FillerLibrary,
) -> anyhow::Result<()> {
    let mut player = AudioPlayer::new();

    if let Some(filler) = filler_library.pick_for_transcript(user_text) {
        let audio = filler.audio.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(FILLER_DELAY_MS));
            let _ = AudioPlayer::play_blocking(&audio);
        });
    }

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
                    player.enqueue(audio_data);
                }
                Err(e) => eprintln!("TTS error: {}", e),
            }
        }
    }

    while player.is_playing() {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    Ok(())
}

fn downsample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let new_len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(new_len);
    for i in 0..new_len {
        let src_idx = (i as f64 * ratio) as usize;
        if src_idx < samples.len() {
            out.push(samples[src_idx]);
        }
    }
    out
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
