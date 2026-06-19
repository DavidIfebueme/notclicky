use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;

use crate::ai::providers::LlmProvider;
use crate::ai::streaming_pipeline::SentenceStream;
use crate::agent::process::AgentManager;
use crate::overlay::cursor::OverlayCommand;
use crate::overlay::integration::process_stream_token;
use crate::screen::capture::ScreenCapture;
use crate::voice::audio_player::AudioPlayer;
use crate::voice::capture::AudioCapture;
use crate::voice::filler::FillerLibrary;
use crate::voice::push_to_talk::GlobalHotkey;
use crate::voice::transcription::SttProvider;
use crate::voice::transcription_deepgram::{DeepgramSttProvider, DeepgramStreamingSession, DeepgramWakeWordSession, encode_wav};
use crate::voice::tts::TtsProvider;
use futures::StreamExt;

type TranscriptCallback = Box<dyn Fn(String) + Send + 'static>;

const FILLER_DELAY_MS: u64 = 400;
const IDLE_GRACE_MS: u64 = 500;

const STATE_IDLE: u8 = 0;
const STATE_LISTENING: u8 = 1;
const STATE_PROCESSING: u8 = 2;
const STATE_RESPONDING: u8 = 3;

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
    voice_state: Arc<AtomicU8>,
    on_transcript: Arc<Mutex<Option<TranscriptCallback>>>,
    wake_word_enabled: bool,
    overlay_tx: mpsc::Sender<OverlayCommand>,
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
        overlay_tx: mpsc::Sender<OverlayCommand>,
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
            voice_state: Arc::new(AtomicU8::new(STATE_IDLE)),
            on_transcript: Arc::new(Mutex::new(None)),
            wake_word_enabled: false,
            overlay_tx,
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
        let voice_state = self.voice_state.clone();
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
        let overlay_tx = self.overlay_tx.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("voice runtime");
            rt.block_on(pipeline_loop(
                running, voice_state, hotkey, capture, stt, deepgram_api_key,
                llm, tts, screen, system_prompt, on_transcript,
                agent_manager, sample_rate, wake_word_enabled, filler_library,
                overlay_tx,
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
    voice_state: Arc<AtomicU8>,
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
    overlay_tx: mpsc::Sender<OverlayCommand>,
) {
    let mut was_pressed = false;
    let mut idle_since: Option<std::time::Instant> = None;
    let mut wake_word_session: Option<DeepgramWakeWordSession> = None;
    let mut wake_word_capture_started = false;
    let response_cancel: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    while running.load(Ordering::SeqCst) {
        let current_state = voice_state.load(Ordering::SeqCst);
        let pressed = hotkey.lock().unwrap().is_pressed();

        if pressed && !was_pressed {
            if current_state == STATE_RESPONDING {
                stop_playback(&response_cancel);
                voice_state.store(STATE_LISTENING, Ordering::SeqCst);
                eprintln!("notclicky: barge-in — interrupted response");
            }
            let _ = capture.lock().unwrap().start();
            voice_state.store(STATE_LISTENING, Ordering::SeqCst);
            was_pressed = true;
            idle_since = None;
        }

        if !pressed && was_pressed {
            was_pressed = false;
            voice_state.store(STATE_PROCESSING, Ordering::SeqCst);

            let audio = capture.lock().unwrap().stop();
            if !audio.is_empty() {
                let (transcript, ptt_interim) = if let Some(ref key) = deepgram_api_key {
                    match DeepgramStreamingSession::connect(key).await {
                        Ok(mut session) => {
                            if session.send_audio(&audio).await.is_err() {
                                let provider = DeepgramSttProvider::new(key.clone());
                                let wav = encode_wav(&audio, sample_rate);
                                let text = provider.transcribe_bytes(&wav).await.unwrap_or_default();
                                (text, String::new())
                            } else {
                                let _ = session.finalize().await;
                                let mut final_text = String::new();
                                let mut first_interim = String::new();
                                while let Some(t) = session.next_transcript().await {
                                    match t {
                                        Ok(t) if t.is_final && !t.text.is_empty() => {
                                            final_text = t.text.clone();
                                        }
                                        Ok(t) => {
                                            if !t.text.is_empty() && first_interim.is_empty() {
                                                first_interim = t.text;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                let _ = session.close().await;
                                if final_text.is_empty() {
                                    let provider = DeepgramSttProvider::new(key.clone());
                                    let wav = encode_wav(&audio, sample_rate);
                                    (provider.transcribe_bytes(&wav).await.unwrap_or_default(), first_interim)
                                } else {
                                    (final_text, first_interim)
                                }
                            }
                        }
                        Err(_) => {
                            let provider = DeepgramSttProvider::new(key.clone());
                            let wav = encode_wav(&audio, sample_rate);
                            (provider.transcribe_bytes(&wav).await.unwrap_or_default(), String::new())
                        }
                    }
                } else {
                    let text = stt.lock().unwrap().transcribe(&audio, sample_rate).await.map(|t| t.text).unwrap_or_default();
                    (text, String::new())
                };

                eprintln!("notclicky: ptt transcript = {:?}", if transcript.is_empty() { "(empty)".to_string() } else { transcript.clone() });

                if !transcript.trim().is_empty() {
                    if let Some(ref cb) = *on_transcript.lock().unwrap() {
                        cb(transcript.clone());
                    }

                    if is_agent_request(&transcript) {
                        let prompt = strip_agent_keyword(&transcript);
                        let mgr_guard = agent_manager.lock().unwrap();
                        if let Some(ref m) = *mgr_guard {
                            let _ = m.spawn(prompt, None, None).await;
                        }
                        voice_state.store(STATE_IDLE, Ordering::SeqCst);
                        idle_since = Some(std::time::Instant::now());
                    } else {
                        let screenshot = if should_attach_screenshot(&transcript) {
                            screen.lock().unwrap().capture_cursor_screen().await.ok()
                        } else {
                            None
                        };
                        let llm_c = llm.clone();
                        let tts_c = tts.clone();
                        let sys = system_prompt.clone();
                        let lib = filler_library.clone();
                        let text = transcript.clone();
                        let vs = voice_state.clone();
                        let cancel = response_cancel.clone();
                        let overlay_tx = overlay_tx.clone();
                        response_cancel.store(false, Ordering::SeqCst);
                        voice_state.store(STATE_RESPONDING, Ordering::SeqCst);
                        std::thread::spawn(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            let _ = rt.block_on(respond_with_pipeline(
                                &llm_c, &tts_c, &sys, &text,
                                screenshot.as_ref(), &lib, &cancel,
                                ptt_interim,
                                overlay_tx,
                            ));
                            vs.compare_exchange(STATE_RESPONDING, STATE_IDLE, Ordering::SeqCst, Ordering::SeqCst).ok();
                        });
                    }
                } else {
                    voice_state.store(STATE_IDLE, Ordering::SeqCst);
                    idle_since = Some(std::time::Instant::now());
                }
            } else {
                voice_state.store(STATE_IDLE, Ordering::SeqCst);
                idle_since = Some(std::time::Instant::now());
            }
        }

        if !wake_word_enabled {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            continue;
        }

        if current_state != STATE_IDLE {
            if let Some(mut session) = wake_word_session.take() {
                let _ = session.close().await;
            }
            wake_word_capture_started = false;
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            continue;
        }

        if !wake_word_capture_started {
            let _ = capture.lock().unwrap().start();
            eprintln!("notclicky: wake word capture started");
            wake_word_capture_started = true;

            if wake_word_session.is_none() {
                if let Some(ref key) = deepgram_api_key {
                    match DeepgramWakeWordSession::connect(key, sample_rate).await {
                        Ok(session) => {
                            eprintln!("notclicky: wake word streaming connected");
                            wake_word_session = Some(session);
                        }
                        Err(e) => {
                            eprintln!("notclicky: wake word connect failed: {}", e);
                        }
                    }
                }
            }
        }

        let grace_ok = idle_since.map_or(true, |t| t.elapsed() >= std::time::Duration::from_millis(IDLE_GRACE_MS));
        if !grace_ok {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            continue;
        }

        if let Some(ref mut session) = wake_word_session {
            let recent = {
                let cap = capture.lock().unwrap();
                cap.snapshot()
            };
            if !recent.is_empty() {
                if session.send_audio(&recent).await.is_err() {
                    eprintln!("notclicky: wake word send_audio failed");
                }

                while let Some(transcript_result) = session.next_transcript().await {
                    match transcript_result {
                        Ok(transcript) => {
                            let lower = transcript.text.to_lowercase();
                            let trimmed = lower.trim();
                            if !trimmed.is_empty() {
                                eprintln!("notclicky: heard \"{}\"", trimmed);
                            }
                            let ww = ["hey clicky", "clicky", "hey clikey", "not clicky", "notclicky"];
                            if ww.iter().any(|w| lower.contains(w)) {
                                eprintln!("notclicky: wake word detected!");
                                voice_state.store(STATE_LISTENING, Ordering::SeqCst);
                                idle_since = None;
                                let _ = capture.lock().unwrap().stop();
                                let _ = capture.lock().unwrap().start();
                                eprintln!("notclicky: listening for command...");

                                let mut interim_transcript = String::new();
                                let mut command_transcript = String::new();
                                let command_start = std::time::Instant::now();

                                while command_start.elapsed() < std::time::Duration::from_secs(4) {
                                    let recent = {
                                        let cap = capture.lock().unwrap();
                                        cap.snapshot()
                                    };
                                    if !recent.is_empty() {
                                        if session.send_audio(&recent).await.is_err() {
                                            eprintln!("notclicky: command send_audio failed");
                                        }

                                        while let Some(transcript_result) = session.next_transcript().await {
                                            match transcript_result {
                                                Ok(t) => {
                                                    if !t.text.trim().is_empty() {
                                                        interim_transcript = t.text.clone();
                                                        eprintln!("notclicky: interim: \"{}\"", t.text);
                                                    }
                                                    if t.is_final {
                                                        command_transcript = t.text.clone();
                                                        eprintln!("notclicky: final transcript: \"{}\"", t.text);
                                                        break;
                                                    }
                                                }
                                                Err(e) => {
                                                    eprintln!("notclicky: command transcript error: {}", e);
                                                }
                                            }
                                        }
                                        if !command_transcript.is_empty() {
                                            break;
                                        }
                                    }
                                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                                }

                                let _ = session.close().await;
                                wake_word_session = None;

                                let interim_for_respond = interim_transcript.clone();
                                let final_text = if command_transcript.is_empty() {
                                    interim_transcript
                                } else {
                                    command_transcript
                                };
                                eprintln!("notclicky: transcript = {:?}", if final_text.is_empty() { "(empty)".to_string() } else { final_text.clone() });

                                if !final_text.trim().is_empty() {
                                    if is_agent_request(&final_text) {
                                        let prompt = strip_agent_keyword(&final_text);
                                        let mgr_guard = agent_manager.lock().unwrap();
                                        if let Some(ref m) = *mgr_guard {
                                            let _ = m.spawn(prompt, None, None).await;
                                        }
                                        voice_state.store(STATE_IDLE, Ordering::SeqCst);
                                        idle_since = Some(std::time::Instant::now());
                                    } else {
                                        voice_state.store(STATE_PROCESSING, Ordering::SeqCst);
                                        let screenshot = if should_attach_screenshot(&final_text) {
                                            screen.lock().unwrap().capture_cursor_screen().await.ok()
                                        } else {
                                            None
                                        };
                                        let llm_c = llm.clone();
                                        let tts_c = tts.clone();
                                        let sys = system_prompt.clone();
                                        let lib = filler_library.clone();
                                        let text = final_text.clone();
                                        let interim = interim_for_respond;
                                        let vs = voice_state.clone();
                                        let cancel = response_cancel.clone();
                                        let overlay_tx = overlay_tx.clone();
                                        response_cancel.store(false, Ordering::SeqCst);
                                        voice_state.store(STATE_RESPONDING, Ordering::SeqCst);
                                        eprintln!("notclicky: sending to LLM...");
                                        std::thread::spawn(move || {
                                            let rt = tokio::runtime::Runtime::new().unwrap();
                                            let result = rt.block_on(respond_with_pipeline(
                                                &llm_c, &tts_c, &sys, &text,
                                                screenshot.as_ref(), &lib, &cancel,
                                                interim,
                                                overlay_tx,
                                            ));
                                            if let Err(e) = result {
                                                eprintln!("notclicky: response error: {}", e);
                                            }
                                            vs.compare_exchange(STATE_RESPONDING, STATE_IDLE, Ordering::SeqCst, Ordering::SeqCst).ok();
                                        });
                                    }
                                } else {
                                    eprintln!("notclicky: no speech detected after wake word");
                                    voice_state.store(STATE_IDLE, Ordering::SeqCst);
                                    idle_since = Some(std::time::Instant::now());
                                }

                                if voice_state.load(Ordering::SeqCst) == STATE_IDLE {
                                    let _ = capture.lock().unwrap().start();
                                }
                                wake_word_capture_started = false;
                                break;
                            }
                        }
                        Err(e) => {
                            eprintln!("notclicky: wake word transcript error: {}", e);
                        }
                    }
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

async fn respond_with_pipeline(
    llm: &Arc<Mutex<Box<dyn LlmProvider>>>,
    tts: &Arc<Mutex<Box<dyn TtsProvider>>>,
    system_prompt: &str,
    user_text: &str,
    screenshot: Option<&crate::screen::capture::CaptureResult>,
    filler_library: &FillerLibrary,
    cancel: &Arc<AtomicBool>,
    interim_text: String,
    overlay_tx: mpsc::Sender<OverlayCommand>,
) -> anyhow::Result<()> {
    use std::sync::mpsc as sync_mpsc;

    let pipeline_future = async {
        let mut player = AudioPlayer::new();
        let (audio_tx, audio_rx) = sync_mpsc::channel::<Vec<u8>>();
        let pending_tts = Arc::new(std::sync::atomic::AtomicU32::new(0));

        if let Some(filler) = filler_library.pick_for_transcript(user_text) {
            let audio = filler.audio.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(FILLER_DELAY_MS));
                let _ = AudioPlayer::play_blocking(&audio);
            });
        }

        let use_speculative = !interim_text.trim().is_empty()
            && crate::ai::prefire::compute_divergence(&interim_text, user_text) <= 0.15;

        let (stream, _is_speculative) = if use_speculative {
            let req = build_prefire_request(&interim_text, system_prompt);
            match llm.lock().unwrap().stream(req).await {
                Ok(s) => {
                    eprintln!("notclicky: using speculative stream (pre-fire hit)");
                    (s, true)
                }
                Err(e) => {
                    eprintln!("notclicky: speculative stream failed: {}, falling back", e);
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
                    (llm.lock().unwrap().stream(req).await?, false)
                }
            }
        } else {
            if !interim_text.trim().is_empty() {
                eprintln!("notclicky: speculative pre-fire skipped (divergence > 15%)");
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
            (llm.lock().unwrap().stream(req).await?, false)
        };

        eprintln!("notclicky: LLM stream connected");

        let overlay_tx_for_points = overlay_tx.clone();
        let point_stream = stream.map(move |token_result| {
            if let Ok(token) = &token_result {
                let _ = process_stream_token(token, &overlay_tx_for_points);
            }
            token_result
        });

        let mut sentence_stream = SentenceStream::new(Box::pin(point_stream));

        while let Some(sentence) = futures::StreamExt::next(&mut sentence_stream).await {
            if cancel.load(Ordering::SeqCst) {
                player.stop();
                break;
            }
            if !sentence.is_empty() {
                pending_tts.fetch_add(1, Ordering::SeqCst);
                let tts_clone = tts.clone();
                let tx = audio_tx.clone();
                let pending_clone = pending_tts.clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let audio_data = {
                        let provider = tts_clone.lock().unwrap();
                        rt.block_on(provider.synthesize(&sentence))
                    };
                    if let Ok(data) = audio_data {
                        let _ = tx.send(data);
                    }
                    pending_clone.fetch_sub(1, Ordering::SeqCst);
                });
            }
        }

        drop(audio_tx);

        loop {
            while let Ok(data) = audio_rx.try_recv() {
                player.enqueue(data);
            }
            if pending_tts.load(Ordering::SeqCst) == 0 && !player.is_playing() {
                break;
            }
            if cancel.load(Ordering::SeqCst) {
                player.stop();
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        Ok(())
    };

    tokio::time::timeout(std::time::Duration::from_secs(120), pipeline_future).await?
}

fn build_prefire_request(text: &str, system_prompt: &str) -> crate::ai::providers::LlmRequest {
    crate::ai::providers::LlmRequest {
        messages: vec![
            crate::ai::providers::LlmMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            crate::ai::providers::LlmMessage {
                role: "user".to_string(),
                content: text.to_string(),
            },
        ],
        model: None,
        max_tokens: None,
        temperature: None,
    }
}

fn stop_playback(cancel: &Arc<AtomicBool>) {
    cancel.store(true, Ordering::SeqCst);
}

pub fn should_attach_screenshot(transcript: &str) -> bool {
    let lower = transcript.to_lowercase();
    let visual_phrases = [
        "my screen", "the screen", "on screen", "on the screen", "this screen",
        "what am i looking", "what's on", "what is on", "what do you see",
        "look at", "take a look", "can you see", "do you see",
        "this window", "that window", "current window", "active window",
        "this app", "that app", "this page", "that page",
        "this button", "that button", "this field", "that field",
        "this menu", "that menu", "where is", "where's",
        "point to", "show me where", "highlight",
        "select", "open this", "open that", "visible", "shown",
        "displayed", "screenshot", "icon", "image", "dialog",
        "sidebar", "toolbar", "tab", "cursor",
    ];
    if visual_phrases.iter().any(|p| contains_word_boundary(&lower, p)) {
        return true;
    }
    let visual_tokens = [
        "screen", "window", "button", "field", "menu", "dialog", "popup",
        "page", "tab", "cursor", "visible", "shown", "displayed", "image",
        "screenshot", "icon", "link", "sidebar", "toolbar", "dock",
        "panel", "notification", "tooltip", "checkbox", "dropdown",
        "click", "press",
    ];
    let words: Vec<&str> = lower.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    words.iter().any(|w| visual_tokens.contains(w))
}

fn contains_word_boundary(text: &str, phrase: &str) -> bool {
    let phrase_words: Vec<&str> = phrase.split_whitespace().collect();
    if phrase_words.is_empty() {
        return false;
    }
    let text_words: Vec<&str> = text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    if phrase_words.len() > text_words.len() {
        return false;
    }
    for i in 0..=text_words.len() - phrase_words.len() {
        if text_words[i..i + phrase_words.len()] == phrase_words[..] {
            return true;
        }
    }
    false
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
