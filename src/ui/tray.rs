use gtk4::prelude::*;
use libadwaita as adw;
use ksni::Tray;
use std::sync::mpsc;

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

pub fn setup(app: &adw::Application) {
    let panel_window = crate::ui::panel::build_panel(app);
    let mini_window = crate::ui::panel::build_mini_panel(app);
    let settings_window = crate::ui::settings::build_settings_window(app);

    let (tx, rx) = mpsc::channel::<TrayEvent>();

    let tray = TrayIcon { tx };
    ksni::TrayService::new(tray).spawn();

    let panel = panel_window.clone();
    let mini = mini_window.clone();
    let settings = settings_window.clone();
    gtk4::glib::MainContext::default().spawn_local(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
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
        }
    });
}
