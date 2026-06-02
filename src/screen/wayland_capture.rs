use anyhow::Result;
use async_trait::async_trait;

use crate::screen::capture::{CaptureResult, ScreenCapture};

pub struct WaylandCapture;

impl WaylandCapture {
    pub fn new() -> Result<Self> {
        anyhow::bail!("Screen capture is not supported on Wayland")
    }
}

#[async_trait]
impl ScreenCapture for WaylandCapture {
    async fn capture_all(&self) -> Result<Vec<CaptureResult>> {
        anyhow::bail!("Screen capture is not supported on Wayland")
    }

    async fn capture_cursor_screen(&self) -> Result<CaptureResult> {
        anyhow::bail!("Screen capture is not supported on Wayland")
    }

    async fn capture_focused_window(&self) -> Result<CaptureResult> {
        anyhow::bail!("Screen capture is not supported on Wayland")
    }
}
