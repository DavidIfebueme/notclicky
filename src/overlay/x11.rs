use anyhow::Result;
use async_trait::async_trait;

use crate::overlay::cursor::{Overlay, Point, Rect};

pub struct X11Overlay;

impl X11Overlay {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

#[async_trait]
impl Overlay for X11Overlay {
    fn show_cursor(&self, _point: Point, _accent: &str, _duration_ms: u32) -> Result<()> {
        todo!()
    }

    fn show_cursors(&self, _points: Vec<Point>, _accent: &str, _duration_ms: u32) -> Result<()> {
        todo!()
    }

    fn show_caption(&self, _text: &str, _x: f64, _y: f64, _accent: &str, _duration_ms: u32) -> Result<()> {
        todo!()
    }

    fn show_highlight(&self, _rect: Rect, _accent: &str, _duration_ms: u32) -> Result<()> {
        todo!()
    }

    fn show_scribble(&self, _points: Vec<Point>, _accent: &str, _duration_ms: u32) -> Result<()> {
        todo!()
    }

    fn clear(&self) -> Result<()> {
        todo!()
    }
}
