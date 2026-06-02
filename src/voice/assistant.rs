use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::ai::providers::LlmProvider;
use crate::ai::streaming_pipeline::SentenceStream;
use crate::voice::capture::AudioCapture;
use crate::voice::push_to_talk::GlobalHotkey;
use crate::voice::transcription::{SttProvider, Transcript};
use crate::voice::tts::TtsProvider;

type TranscriptCallback = Box<dyn Fn(Transcript) + Send + 'static>;

pub struct VoiceAssistant {
    hotkey: Arc<Mutex<Box<dyn GlobalHotkey>>>,
    capture: Arc<Mutex<AudioCapture>>,
    stt: Arc<Mutex<Box<dyn SttProvider>>>,
    llm: Arc<Mutex<Box<dyn LlmProvider>>>,
    tts: Arc<Mutex<Box<dyn TtsProvider>>>,
    system_prompt: String,
    running: Arc<AtomicBool>,
    on_transcript: Arc<Mutex<Option<TranscriptCallback>>>,
}

impl VoiceAssistant {
    pub fn new(
        hotkey: Box<dyn GlobalHotkey>,
        capture: AudioCapture,
        stt: Box<dyn SttProvider>,
        llm: Box<dyn LlmProvider>,
        tts: Box<dyn TtsProvider>,
        system_prompt: String,
    ) -> Self {
        Self {
            hotkey: Arc::new(Mutex::new(hotkey)),
            capture: Arc::new(Mutex::new(capture)),
            stt: Arc::new(Mutex::new(stt)),
            llm: Arc::new(Mutex::new(llm)),
            tts: Arc::new(Mutex::new(tts)),
            system_prompt,
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
        let llm = self.llm.clone();
        let tts = self.tts.clone();
        let system_prompt = self.system_prompt.clone();
        let on_transcript = self.on_transcript.clone();
        let sample_rate = capture.lock().unwrap().sample_rate();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
            let mut was_pressed = false;

            while running.load(Ordering::SeqCst) {
                let pressed = hotkey.lock().unwrap().is_pressed();

                if pressed && !was_pressed {
                    let _ = capture.lock().unwrap().start();
                    was_pressed = true;
                } else if !pressed && was_pressed {
                    let audio = capture.lock().unwrap().stop();
                    if !audio.is_empty() {
                        let transcript_result = rt.block_on(async {
                            stt.lock().unwrap().transcribe(&audio, sample_rate).await
                        });

                        if let Ok(transcript) = transcript_result {
                            if let Some(ref cb) = *on_transcript.lock().unwrap() {
                                cb(Transcript {
                                    text: transcript.text.clone(),
                                    is_final: true,
                                });
                            }

                            if !transcript.text.trim().is_empty() {
                                let _ = rt.block_on(process_response(
                                    &llm, &tts, &system_prompt, &transcript.text,
                                ));
                            }
                        }
                    }
                    was_pressed = false;
                }

                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });

        Ok(())
    }

    pub fn stop(&self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        self.hotkey.lock().unwrap().unregister()
    }
}

async fn process_response(
    llm: &Arc<Mutex<Box<dyn LlmProvider>>>,
    tts: &Arc<Mutex<Box<dyn TtsProvider>>>,
    system_prompt: &str,
    user_text: &str,
) -> anyhow::Result<()> {
    let req = crate::ai::providers::LlmRequest {
        messages: vec![
            crate::ai::providers::LlmMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            crate::ai::providers::LlmMessage {
                role: "user".to_string(),
                content: user_text.to_string(),
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
