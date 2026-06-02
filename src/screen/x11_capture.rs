use anyhow::Result;
use async_trait::async_trait;

use crate::screen::capture::{CaptureResult, ScreenCapture};

pub struct X11Capture;

impl X11Capture {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

#[async_trait]
impl ScreenCapture for X11Capture {
    async fn capture_all(&self) -> Result<Vec<CaptureResult>> {
        todo!()
    }

    async fn capture_cursor_screen(&self) -> Result<CaptureResult> {
        todo!()
    }

    async fn capture_focused_window(&self) -> Result<CaptureResult> {
        todo!()
    }
}
