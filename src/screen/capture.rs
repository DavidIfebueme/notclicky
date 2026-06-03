use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct CaptureResult {
    pub image_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub app_name: Option<String>,
    pub is_cursor_screen: bool,
}

#[async_trait]
pub trait ScreenCapture: Send + Sync {
    async fn capture_all(&self) -> Result<Vec<CaptureResult>>;
    async fn capture_cursor_screen(&self) -> Result<CaptureResult>;
    #[allow(dead_code)]
    async fn capture_focused_window(&self) -> Result<CaptureResult>;
}
