use anyhow::Result;

use crate::voice::push_to_talk::GlobalHotkey;
use crate::screen::capture::ScreenCapture;

pub enum Backend {
    X11,
    Wayland,
}

impl Backend {
    pub fn detect() -> Self {
        match std::env::var("XDG_SESSION_TYPE").as_deref() {
            Ok("wayland") => Backend::Wayland,
            _ => Backend::X11,
        }
    }
}

pub fn create_capture(backend: &Backend) -> Result<Box<dyn ScreenCapture>> {
    match backend {
        Backend::X11 => Ok(Box::new(crate::screen::x11_capture::X11Capture::new()?)),
        Backend::Wayland => Ok(Box::new(crate::screen::wayland_capture::WaylandCapture::new()?)),
    }
}

pub fn create_hotkey(backend: &Backend) -> Result<Box<dyn GlobalHotkey>> {
    match backend {
        Backend::X11 => Ok(Box::new(crate::platform::x11::X11Hotkey::new()?)),
        Backend::Wayland => Ok(Box::new(crate::platform::wayland::WaylandHotkey::new()?)),
    }
}
