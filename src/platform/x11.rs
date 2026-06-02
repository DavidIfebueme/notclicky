use anyhow::Result;
use async_trait::async_trait;

use crate::voice::push_to_talk::GlobalHotkey;

pub struct X11Hotkey;

impl X11Hotkey {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

#[async_trait]
impl GlobalHotkey for X11Hotkey {
    fn register(&self, _modifiers: Vec<&str>, _key: Option<&str>) -> Result<()> {
        todo!()
    }

    fn unregister(&self) -> Result<()> {
        todo!()
    }
}
