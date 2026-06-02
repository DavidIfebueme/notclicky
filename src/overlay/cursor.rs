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

pub enum OverlayCommand {
    ShowCursor(Point, String, u32),
    ShowCursors(Vec<Point>, String, u32),
    ShowCaption(String, f64, f64, String, u32),
    ShowHighlight(Rect, String, u32),
    ShowScribble(Vec<Point>, String, u32),
    NavigateCursor(f64, f64, String),
    Clear,
}
