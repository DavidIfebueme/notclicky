use std::sync::mpsc;
use std::sync::Arc;

use crate::ai::providers::LlmProvider;
use crate::app::{AppConfig, Secrets};
use crate::overlay::cursor::OverlayCommand;
use crate::voice::push_to_talk::GlobalHotkey;
use crate::voice::tts::TtsProvider;
use crate::voice::transcription::SttProvider;
use crate::screen::capture::ScreenCapture;

pub struct NotClickyApp {
    pub overlay_tx: mpsc::Sender<OverlayCommand>,
    pub llm: Arc<tokio::sync::Mutex<Box<dyn LlmProvider>>>,
    pub tts: Arc<tokio::sync::Mutex<Box<dyn TtsProvider>>>,
    pub screen: Arc<tokio::sync::Mutex<Box<dyn ScreenCapture>>>,
    pub config: AppConfig,
    pub _secrets: Secrets,
}

impl NotClickyApp {
    pub fn new(
        overlay_tx: mpsc::Sender<OverlayCommand>,
        llm: Box<dyn LlmProvider>,
        tts: Box<dyn TtsProvider>,
        screen: Box<dyn ScreenCapture>,
        config: AppConfig,
        secrets: Secrets,
    ) -> Self {
        Self {
            overlay_tx,
            llm: Arc::new(tokio::sync::Mutex::new(llm)),
            tts: Arc::new(tokio::sync::Mutex::new(tts)),
            screen: Arc::new(tokio::sync::Mutex::new(screen)),
            config,
            _secrets: secrets,
        }
    }

    pub fn start_bridge(&self) {
        let state = crate::bridge::server::AppState {
            overlay_tx: self.overlay_tx.clone(),
            screen: self.screen.clone(),
            tts: self.tts.clone(),
            llm: self.llm.clone(),
            _auth_token: self.config.bridge.token.clone(),
            event_tx: tokio::sync::broadcast::channel(256).0,
        };

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("bridge tokio runtime");
            rt.block_on(async {
                if let Err(e) = crate::bridge::server::start_server(state).await {
                    eprintln!("Bridge server error: {}", e);
                }
            });
        });
    }

    pub fn start_voice(
        &mut self,
        hotkey: Box<dyn GlobalHotkey>,
        stt: Box<dyn SttProvider>,
    ) -> Result<(), anyhow::Error> {
        let capture = crate::voice::capture::AudioCapture::new(16000);
        let resources_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");

        let skill_context = crate::skills::context::SkillContext::new(resources_dir);
        let soul_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("SOUL.md");
        let soul = std::fs::read_to_string(&soul_path).unwrap_or_default();
        let skill_prompt = skill_context.build_system_prompt(None);
        let system_prompt = format!("{}\n\n{}", soul, skill_prompt);

        let overlay_tx = self.overlay_tx.clone();
        capture.set_rms_callback(Box::new(move |rms: f32| {
            let _ = overlay_tx.send(OverlayCommand::ShowWaveform(rms as f64));
        }));

        let mut assistant = crate::voice::assistant::VoiceAssistant::new(
            hotkey,
            capture,
            stt,
            {
                let llm = self.llm.clone();
                struct LlmWrapper { inner: Arc<tokio::sync::Mutex<Box<dyn LlmProvider>>> }
                #[async_trait::async_trait]
                impl LlmProvider for LlmWrapper {
                    async fn complete(&self, req: crate::ai::providers::LlmRequest) -> anyhow::Result<crate::ai::providers::LlmResponse> {
                        self.inner.lock().await.complete(req).await
                    }
                    async fn stream(&self, req: crate::ai::providers::LlmRequest) -> anyhow::Result<crate::ai::providers::LlmStream> {
                        self.inner.lock().await.stream(req).await
                    }
                }
                Box::new(LlmWrapper { inner: llm }) as Box<dyn LlmProvider>
            },
            {
                let tts = self.tts.clone();
                struct TtsWrapper { inner: Arc<tokio::sync::Mutex<Box<dyn TtsProvider>>> }
                #[async_trait::async_trait]
                impl TtsProvider for TtsWrapper {
                    async fn synthesize(&self, text: &str) -> anyhow::Result<Vec<u8>> {
                        self.inner.lock().await.synthesize(text).await
                    }
                    async fn synthesize_stream(&self, text_stream: std::pin::Pin<Box<dyn futures::Stream<Item = String> + Send>>) -> anyhow::Result<crate::voice::tts::AudioStream> {
                        self.inner.lock().await.synthesize_stream(text_stream).await
                    }
                }
                Box::new(TtsWrapper { inner: tts }) as Box<dyn TtsProvider>
            },
            {
                let screen = self.screen.clone();
                struct ScreenWrapper { inner: Arc<tokio::sync::Mutex<Box<dyn ScreenCapture>>> }
                #[async_trait::async_trait]
                impl ScreenCapture for ScreenWrapper {
                    async fn capture_all(&self) -> anyhow::Result<Vec<crate::screen::capture::CaptureResult>> {
                        self.inner.lock().await.capture_all().await
                    }
                    async fn capture_cursor_screen(&self) -> anyhow::Result<crate::screen::capture::CaptureResult> {
                        self.inner.lock().await.capture_cursor_screen().await
                    }
                    async fn capture_focused_window(&self) -> anyhow::Result<crate::screen::capture::CaptureResult> {
                        self.inner.lock().await.capture_focused_window().await
                    }
                }
                Box::new(ScreenWrapper { inner: screen }) as Box<dyn ScreenCapture>
            },
            system_prompt,
        );

        let home_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("notclicky")
            .join("agent-home");
        let agent_backend = crate::agent::process::AgentBackend::from_str(&self.config.agent.backend);
        let agent_manager = crate::agent::process::AgentManager::new(home_dir, agent_backend);
        assistant.set_agent_manager(agent_manager);

        if let Some(ref deepgram_key) = self._secrets.deepgram_api_key {
            if !deepgram_key.is_empty() {
                assistant.set_deepgram_api_key(deepgram_key.clone());
                assistant.set_wake_word_enabled(true);
            }
        }

        assistant.start()
    }
}
