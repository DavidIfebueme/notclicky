use anyhow::Result;
use async_trait::async_trait;

use crate::overlay::cursor::{Overlay, Point, Rect};

pub struct WaylandOverlay;

impl WaylandOverlay {
    pub fn new() -> Result<Self> {
        anyhow::bail!("Overlay is not supported on Wayland")
    }
}

#[async_trait]
impl Overlay for WaylandOverlay {
    fn show_cursor(&self, _point: Point, _accent: &str, _duration_ms: u32) -> Result<()> {
        anyhow::bail!("Overlay is not supported on Wayland")
    }

    fn show_cursors(&self, _points: Vec<Point>, _accent: &str, _duration_ms: u32) -> Result<()> {
        anyhow::bail!("Overlay is not supported on Wayland")
    }

    fn show_caption(&self, _text: &str, _x: f64, _y: f64, _accent: &str, _duration_ms: u32) -> Result<()> {
        anyhow::bail!("Overlay is not supported on Wayland")
    }

    fn show_highlight(&self, _rect: Rect, _accent: &str, _duration_ms: u32) -> Result<()> {
        anyhow::bail!("Overlay is not supported on Wayland")
    }

    fn show_scribble(&self, _points: Vec<Point>, _accent: &str, _duration_ms: u32) -> Result<()> {
        anyhow::bail!("Overlay is not supported on Wayland")
    }

    fn clear(&self) -> Result<()> {
        anyhow::bail!("Overlay is not supported on Wayland")
    }
}
