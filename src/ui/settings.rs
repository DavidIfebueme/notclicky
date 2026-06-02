use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::app::{AppConfig, Secrets};

pub fn build_settings_window(app: &adw::Application) -> adw::PreferencesWindow {
    let config = crate::app::load().unwrap_or_default();
    let secrets = Rc::new(RefCell::new(Secrets::load().unwrap_or_else(|_| {
        Secrets { values: std::collections::HashMap::new() }
    })));

    let window = adw::PreferencesWindow::builder()
        .application(app)
        .title("NotClicky Settings")
        .default_width(640)
        .default_height(520)
        .build();

    window.add(&build_llm_page(&config, &secrets));
    window.add(&build_tts_page(&config, &secrets));
    window.add(&build_stt_page(&config, &secrets));
    window.add(&build_bridge_page(&config));
    window.add(&build_overlay_page(&config));

    window
}

fn build_llm_page(config: &AppConfig, secrets: &Rc<RefCell<Secrets>>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::new();
    page.set_title("LLM");
    page.set_icon_name(Some("text-x-generic-symbolic"));

    let group = adw::PreferencesGroup::new();
    group.set_title("Language Model");

    let providers = ["openai-compatible", "anthropic", "openai", "ollama"];
    let provider_row = combo_row("Provider", &providers, &config.llm.provider);
    let base_url_row = entry_row("Base URL", &config.llm.base_url);
    let model_row = entry_row("Model", &config.llm.model);
    let api_key_row = secret_entry_row("API Key", secrets.clone(), "ZAI_API_KEY");

    group.add(&provider_row);
    group.add(&base_url_row);
    group.add(&model_row);
    group.add(&api_key_row);

    if secrets.borrow().get("ZAI_API_KEY").is_none() {
        let warning = adw::ActionRow::builder()
            .title("ZAI_API_KEY not found")
            .subtitle("Add it to ~/.config/notclicky/secrets.env")
            .build();
        warning.add_css_class("error");
        group.add(&warning);
    }

    page.add(&group);
    page
}

fn build_tts_page(config: &AppConfig, secrets: &Rc<RefCell<Secrets>>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::new();
    page.set_title("TTS");
    page.set_icon_name(Some("audio-speakers-symbolic"));

    let group = adw::PreferencesGroup::new();
    group.set_title("Text-to-Speech");

    let providers = ["edge", "elevenlabs", "deepgram", "cartesia"];
    let provider_row = combo_row("Provider", &providers, &config.tts.provider);
    let voice_id_row = secret_entry_row("Voice ID", secrets.clone(), "ELEVENLABS_VOICE_ID");

    group.add(&provider_row);
    group.add(&voice_id_row);

    page.add(&group);
    page
}

fn build_stt_page(config: &AppConfig, secrets: &Rc<RefCell<Secrets>>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::new();
    page.set_title("STT");
    page.set_icon_name(Some("microphone-symbolic"));

    let group = adw::PreferencesGroup::new();
    group.set_title("Speech-to-Text");

    let providers = ["whisper-cpp", "deepgram", "openai-whisper", "assemblyai"];
    let provider_row = combo_row("Provider", &providers, &config.stt.provider);
    let model_row = combo_row("Model", &["tiny", "base", "small", "medium", "large"], &config.stt.model);
    let language_row = entry_row("Language", &config.stt.language);
    let api_key_row = secret_entry_row("Deepgram API Key", secrets.clone(), "DEEPGRAM_API_KEY");

    group.add(&provider_row);
    group.add(&model_row);
    group.add(&language_row);
    group.add(&api_key_row);

    page.add(&group);
    page
}

fn build_bridge_page(config: &AppConfig) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::new();
    page.set_title("Bridge");
    page.set_icon_name(Some("network-workgroup-symbolic"));

    let group = adw::PreferencesGroup::new();
    group.set_title("External Control Bridge");

    let port_row = entry_row("Port", &config.bridge.port.to_string());
    let token_row = entry_row("Auth Token", &config.bridge.token);

    group.add(&port_row);
    group.add(&token_row);

    page.add(&group);
    page
}

fn build_overlay_page(config: &AppConfig) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::new();
    page.set_title("Overlay");
    page.set_icon_name(Some("cursor-symbolic"));

    let group = adw::PreferencesGroup::new();
    group.set_title("Cursor Overlay");

    let backends = ["x11", "wayland"];
    let backend_row = combo_row("Backend", &backends, &config.overlay.backend);

    group.add(&backend_row);

    page.add(&group);
    page
}

fn combo_row(title: &str, options: &[&str], current: &str) -> adw::ComboRow {
    let row = adw::ComboRow::builder()
        .title(title)
        .build();

    let model = gtk4::StringList::new(options);
    row.set_model(Some(&model));

    if let Some(idx) = options.iter().position(|o| o == &current) {
        row.set_selected(idx as u32);
    }

    row
}

fn entry_row(title: &str, value: &str) -> adw::EntryRow {
    adw::EntryRow::builder()
        .title(title)
        .text(value)
        .build()
}

fn secret_entry_row(title: &str, secrets: Rc<RefCell<Secrets>>, key: &'static str) -> adw::EntryRow {
    let current = secrets.borrow().get(key).unwrap_or("").to_string();
    let display = if current.is_empty() { String::new() } else { "••••••••".to_string() };
    let row = adw::EntryRow::builder()
        .title(title)
        .text(&display)
        .build();

    row.connect_changed(move |row| {
        let text = row.text().to_string();
        if !text.is_empty() && text != "••••••••" {
            if let Ok(mut s) = secrets.try_borrow_mut() {
                let _ = s.set(key, &text);
            }
        }
    });

    row
}
