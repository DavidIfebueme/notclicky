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

fn main() {
    let app = adw::Application::builder()
        .application_id("com.notclicky.app")
        .build();

    app.connect_activate(|app| {
        ui::tray::setup(app);
    });

    app.run();
}
