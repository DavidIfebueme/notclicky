use gtk4::prelude::*;
use libadwaita as adw;
use ksni::Tray;
use std::sync::mpsc;

enum TrayEvent {
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
        let tx_settings = self.tx.clone();
        let tx_quit = self.tx.clone();
        vec![
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
    let settings_window = crate::ui::settings::build_settings_window(app);
    let window = settings_window.upcast::<gtk4::Window>();

    let (tx, rx) = mpsc::channel::<TrayEvent>();

    let tray = TrayIcon { tx };
    ksni::TrayService::new(tray).spawn();

    let win = window.clone();
    gtk4::glib::MainContext::default().spawn_local(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            while let Ok(event) = rx.try_recv() {
                match event {
                    TrayEvent::Settings => win.present(),
                    TrayEvent::Quit => win.close(),
                }
            }
        }
    });

    window.present();
}
