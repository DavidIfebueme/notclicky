use anyhow::Result;
use async_trait::async_trait;

use crate::voice::push_to_talk::GlobalHotkey;

pub struct WaylandHotkey;

impl WaylandHotkey {
    pub fn new() -> Result<Self> {
        anyhow::bail!("Global hotkeys are not supported on Wayland")
    }
}

#[async_trait]
impl GlobalHotkey for WaylandHotkey {
    fn register(&self, _modifiers: Vec<&str>, _key: Option<&str>) -> Result<()> {
        anyhow::bail!("Global hotkeys are not supported on Wayland")
    }

    fn unregister(&self) -> Result<()> {
        anyhow::bail!("Global hotkeys are not supported on Wayland")
    }
}
