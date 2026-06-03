use anyhow::Result;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::voice::push_to_talk::GlobalHotkey;

pub struct WaylandHotkey {
    pressed: Arc<AtomicBool>,
    registered: Arc<AtomicBool>,
}

impl WaylandHotkey {
    pub fn new() -> Result<Self> {
        Ok(Self {
            pressed: Arc::new(AtomicBool::new(false)),
            registered: Arc::new(AtomicBool::new(false)),
        })
    }
}

#[async_trait]
impl GlobalHotkey for WaylandHotkey {
    fn register(&self, _modifiers: Vec<&str>, _key: Option<&str>) -> Result<()> {
        let dbus_available = std::process::Command::new("which")
            .arg("gdbus")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !dbus_available {
            anyhow::bail!("gdbus not available for Wayland GlobalShortcuts. Install gdbus or use X11.");
        }

        self.registered.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn unregister(&self) -> Result<()> {
        self.registered.store(false, Ordering::SeqCst);
        self.pressed.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_pressed(&self) -> bool {
        if !self.registered.load(Ordering::SeqCst) {
            return false;
        }

        let result = std::process::Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest", "org.freedesktop.portal.Desktop",
                "--object-path", "/org/freedesktop/portal/desktop",
                "--method", "org.freedesktop.portal.GlobalShortcuts.ListShortcuts",
                "",
                "{}",
            ])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                self.pressed.load(Ordering::SeqCst)
            }
            _ => false,
        }
    }
}
