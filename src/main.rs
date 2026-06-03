use gtk4::prelude::*;
use libadwaita as adw;

mod app;
mod voice;
mod ai;
mod overlay;
mod screen;
mod agent;
mod bridge;
mod skills;
mod memory;
mod ui;
mod platform;
mod notclicky_app;

fn main() {
    let app = adw::Application::builder()
        .application_id("com.notclicky.app")
        .build();

    app.connect_activate(|gtk_app| {
        let config = app::load().unwrap_or_default();
        let secrets = app::Secrets::load().unwrap_or_else(|_| {
            app::Secrets { values: std::collections::HashMap::new() }
        });

        let backend = platform::linux::Backend::detect();

        let overlay = create_overlay(gtk_app, &backend);

        let overlay_tx = match &overlay {
            Some(o) => match o {
                overlay::OverlayBackend::X11(x) => x.sender().clone(),
                overlay::OverlayBackend::Wayland(_) => {
                    let (tx, _) = std::sync::mpsc::channel();
                    tx
                }
            },
            None => {
                let (tx, _) = std::sync::mpsc::channel();
                tx
            }
        };

        let llm = create_llm(&config, &secrets);
        let tts = create_tts(&config, &secrets);
        let screen = create_screen_capture(&backend);

        let nc_app = notclicky_app::NotClickyApp::new(
            overlay_tx,
            llm,
            tts,
            screen,
            config.clone(),
            secrets,
        );

        nc_app.start_bridge();

        let hotkey = platform::linux::create_hotkey(&backend).ok();
        let stt = create_stt(&config);

        if let (Some(hotkey), Some(stt)) = (hotkey, stt) {
            if let Err(e) = nc_app.start_voice(hotkey, stt) {
                eprintln!("Voice pipeline error: {}", e);
            }
        } else {
            eprintln!("Voice pipeline disabled: hotkey or STT not available");
        }

        ui::tray::setup_with_app(gtk_app, &nc_app);
    });

    app.run();
}

fn create_overlay(app: &adw::Application, backend: &platform::linux::Backend) -> Option<overlay::OverlayBackend> {
    match backend {
        platform::linux::Backend::X11 => {
            match overlay::x11::X11Overlay::new(app) {
                Ok(o) => Some(overlay::OverlayBackend::X11(o)),
                Err(e) => {
                    eprintln!("X11 overlay failed: {}", e);
                    None
                }
            }
        }
        platform::linux::Backend::Wayland => {
            match overlay::wayland::WaylandOverlay::new() {
                Ok(o) => Some(overlay::OverlayBackend::Wayland(o)),
                Err(e) => {
                    eprintln!("Wayland overlay failed: {}", e);
                    None
                }
            }
        }
    }
}

fn create_llm(config: &app::AppConfig, secrets: &app::Secrets) -> Box<dyn ai::providers::LlmProvider> {
    match config.llm.provider.as_str() {
        "anthropic" => {
            let key = secrets.get("ANTHROPIC_API_KEY").unwrap_or("").to_string();
            Box::new(ai::providers::anthropic::AnthropicProvider::new(key, config.llm.model.clone()))
        }
        "openai" => {
            let key = secrets.get("OPENAI_API_KEY").unwrap_or("").to_string();
            Box::new(ai::providers::openai::OpenAiProvider::new(key, config.llm.model.clone()))
        }
        "ollama" => {
            Box::new(ai::providers::ollama::OllamaProvider::new(config.llm.model.clone()))
        }
        _ => {
            let key = secrets.get("ZAI_API_KEY").unwrap_or("").to_string();
            let base_url = if config.llm.base_url.is_empty() {
                "https://api.zai.chat/v1".to_string()
            } else {
                config.llm.base_url.clone()
            };
            Box::new(ai::providers::openai_compat::OpenAiCompatProvider::new(
                base_url, key, config.llm.model.clone(),
            ))
        }
    }
}

fn create_tts(config: &app::AppConfig, secrets: &app::Secrets) -> Box<dyn voice::tts::TtsProvider> {
    match config.tts.provider.as_str() {
        "elevenlabs" => {
            let key = secrets.get("ELEVENLABS_API_KEY").unwrap_or("").to_string();
            let voice_id = secrets.get("ELEVENLABS_VOICE_ID").unwrap_or("").to_string();
            Box::new(voice::tts_providers::elevenlabs::ElevenLabsProvider::new(key, voice_id))
        }
        "deepgram" => {
            let key = secrets.get("DEEPGRAM_API_KEY").unwrap_or("").to_string();
            Box::new(voice::tts_providers::deepgram::DeepgramTtsProvider::new(key))
        }
        "cartesia" => {
            let key = secrets.get("CARTESIA_API_KEY").unwrap_or("").to_string();
            Box::new(voice::tts_providers::cartesia::CartesiaProvider::new(key))
        }
        _ => {
            Box::new(voice::tts_providers::edge::EdgeTtsProvider::new(config.tts.voice_id.clone()))
        }
    }
}

fn create_stt(config: &app::AppConfig) -> Option<Box<dyn voice::transcription::SttProvider>> {
    match config.stt.provider.as_str() {
        "whisper-cpp" => {
            let model_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                .join("notclicky")
                .join("whisper-models");
            let model_path = model_dir.join(format!("ggml-{}.bin", config.stt.model));
            if model_path.exists() {
                Some(Box::new(voice::transcription_whisper::WhisperSttProvider::new(model_path)))
            } else {
                eprintln!("Whisper model not found at {:?}. Download it or change STT provider.", model_path);
                None
            }
        }
        _ => None,
    }
}

fn create_screen_capture(backend: &platform::linux::Backend) -> Box<dyn screen::capture::ScreenCapture> {
    match platform::linux::create_capture(backend) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Screen capture unavailable: {}", e);
            struct NoopCapture;
            #[async_trait::async_trait]
            impl screen::capture::ScreenCapture for NoopCapture {
                async fn capture_all(&self) -> anyhow::Result<Vec<screen::capture::CaptureResult>> {
                    anyhow::bail!("No screen capture available")
                }
                async fn capture_cursor_screen(&self) -> anyhow::Result<screen::capture::CaptureResult> {
                    anyhow::bail!("No screen capture available")
                }
                async fn capture_focused_window(&self) -> anyhow::Result<screen::capture::CaptureResult> {
                    anyhow::bail!("No screen capture available")
                }
            }
            Box::new(NoopCapture)
        }
    }
}
