use gtk4::prelude::*;
use libadwaita as adw;
use ksni::Tray;
use std::sync::mpsc;

use crate::notclicky_app::NotClickyApp;

enum TrayEvent {
    Chat,
    MiniChat,
    Settings,
    Quit,
}

struct TrayIcon {
    tx: mpsc::Sender<TrayEvent>,
}

impl Tray for TrayIcon {
    fn id(&self) -> String {
        "notclicky".into()
    }

    fn title(&self) -> String {
        "NotClicky".into()
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::MenuItem;
        let tx_chat = self.tx.clone();
        let tx_mini = self.tx.clone();
        let tx_settings = self.tx.clone();
        let tx_quit = self.tx.clone();
        vec![
            MenuItem::Standard(ksni::menu::StandardItem {
                label: "Chat".into(),
                activate: Box::new(move |_: &mut TrayIcon| {
                    let _ = tx_chat.send(TrayEvent::Chat);
                }),
                ..Default::default()
            }),
            MenuItem::Standard(ksni::menu::StandardItem {
                label: "Mini Chat".into(),
                activate: Box::new(move |_: &mut TrayIcon| {
                    let _ = tx_mini.send(TrayEvent::MiniChat);
                }),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(ksni::menu::StandardItem {
                label: "Settings".into(),
                activate: Box::new(move |_: &mut TrayIcon| {
                    let _ = tx_settings.send(TrayEvent::Settings);
                }),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(ksni::menu::StandardItem {
                label: "Quit".into(),
                activate: Box::new(move |_: &mut TrayIcon| {
                    let _ = tx_quit.send(TrayEvent::Quit);
                }),
                ..Default::default()
            }),
        ]
    }
}

pub fn setup_with_app(app: &adw::Application, nc_app: &NotClickyApp) {
    let panel_window = crate::ui::panel::build_panel(app, nc_app);
    let mini_window = crate::ui::panel::build_mini_panel(app);
    let settings_window = crate::ui::settings::build_settings_window(app);

    let (tx, rx) = mpsc::channel::<TrayEvent>();

    let tray = TrayIcon { tx };
    ksni::TrayService::new(tray).spawn();

    let panel = panel_window.clone();
    let mini = mini_window.clone();
    let settings = settings_window.clone();
    gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        while let Ok(event) = rx.try_recv() {
            match event {
                TrayEvent::Chat => panel.present(),
                TrayEvent::MiniChat => mini.present(),
                TrayEvent::Settings => settings.present(),
                TrayEvent::Quit => {
                    panel.close();
                    mini.close();
                    settings.close();
                }
            }
        }
        gtk4::glib::ControlFlow::Continue
    });
}

pub fn setup(app: &adw::Application) {
    let config = crate::app::load().unwrap_or_default();
    let secrets = crate::app::Secrets::load().unwrap_or_else(|_| {
        crate::app::Secrets { values: std::collections::HashMap::new() }
    });
    let (overlay_tx, _) = mpsc::channel();
    let llm = Box::new(crate::ai::providers::openai_compat::OpenAiCompatProvider::new(
        String::new(), String::new(), String::new(),
    ));
    let tts = Box::new(crate::voice::tts_providers::edge::EdgeTtsProvider::new(String::new()));
    struct NoopCapture;
    #[async_trait::async_trait]
    impl crate::screen::capture::ScreenCapture for NoopCapture {
        async fn capture_all(&self) -> anyhow::Result<Vec<crate::screen::capture::CaptureResult>> {
            anyhow::bail!("No screen capture")
        }
        async fn capture_cursor_screen(&self) -> anyhow::Result<crate::screen::capture::CaptureResult> {
            anyhow::bail!("No screen capture")
        }
        async fn capture_focused_window(&self) -> anyhow::Result<crate::screen::capture::CaptureResult> {
            anyhow::bail!("No screen capture")
        }
    }
    let screen: Box<dyn crate::screen::capture::ScreenCapture> = Box::new(NoopCapture);
    let nc_app = NotClickyApp::new(overlay_tx, llm, tts, screen, config, secrets);
    setup_with_app(app, &nc_app);
}
