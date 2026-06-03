use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    ShowWaveform(f64),
    HideWaveform,
    Clear,
}
