use anyhow::Result;
use std::sync::mpsc;

use crate::overlay::cursor::OverlayCommand;

pub enum OverlayBackend {
    X11(x11::X11Overlay),
    Wayland(wayland::WaylandOverlay),
}

impl OverlayBackend {
    #[allow(dead_code)]
    pub fn send(&self, cmd: OverlayCommand) -> Result<()> {
        match self {
            OverlayBackend::X11(o) => o.send(cmd),
            OverlayBackend::Wayland(o) => o.send(cmd),
        }
    }

    #[allow(dead_code)]
    pub fn sender(&self) -> &mpsc::Sender<OverlayCommand> {
        match self {
            OverlayBackend::X11(o) => o.sender(),
            OverlayBackend::Wayland(_) => panic!("Wayland overlay has no sender"),
        }
    }
}

pub mod x11;
pub mod wayland;
pub mod cursor;
pub mod integration;
