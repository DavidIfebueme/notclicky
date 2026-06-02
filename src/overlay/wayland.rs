use anyhow::Result;

use crate::overlay::cursor::OverlayCommand;

pub struct WaylandOverlay;

impl WaylandOverlay {
    pub fn new() -> Result<Self> {
        anyhow::bail!("Overlay is not supported on Wayland")
    }

    pub fn send(&self, _cmd: OverlayCommand) -> Result<()> {
        anyhow::bail!("Overlay is not supported on Wayland")
    }
}
