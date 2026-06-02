use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[async_trait]
pub trait Overlay: Send + Sync {
    fn show_cursor(&self, point: Point, accent: &str, duration_ms: u32) -> Result<()>;
    fn show_cursors(&self, points: Vec<Point>, accent: &str, duration_ms: u32) -> Result<()>;
    fn show_caption(&self, text: &str, x: f64, y: f64, accent: &str, duration_ms: u32) -> Result<()>;
    fn show_highlight(&self, rect: Rect, accent: &str, duration_ms: u32) -> Result<()>;
    fn show_scribble(&self, points: Vec<Point>, accent: &str, duration_ms: u32) -> Result<()>;
    fn clear(&self) -> Result<()>;
}
