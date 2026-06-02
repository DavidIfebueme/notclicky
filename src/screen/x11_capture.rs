use anyhow::Result;
use async_trait::async_trait;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

use crate::screen::capture::{CaptureResult, ScreenCapture};

const MAX_DIMENSION: u32 = 1280;
const JPEG_QUALITY: u8 = 80;

pub struct X11Capture {
    screen_num: usize,
}

impl X11Capture {
    pub fn new() -> Result<Self> {
        let (_, screen_num) = RustConnection::connect(None)?;
        Ok(Self { screen_num })
    }

    fn connect(&self) -> Result<(RustConnection, usize)> {
        let (conn, screen_num) = RustConnection::connect(None)?;
        Ok((conn, screen_num))
    }

    fn capture_root(&self, conn: &RustConnection, root: u32, width: u16, height: u16, is_cursor: bool) -> Result<CaptureResult> {
        let reply = get_image(conn, ImageFormat::Z_PIXMAP, root, 0, 0, width, height, 0xFFFFFFFF)?.reply()?;
        let jpeg = encode_to_jpeg(&reply.data, width as u32, height as u32)?;
        let (w, h) = dimensions(width as u32, height as u32);
        Ok(CaptureResult {
            image_data: jpeg,
            width: w,
            height: h,
            app_name: None,
            is_cursor_screen: is_cursor,
        })
    }
}

#[async_trait]
impl ScreenCapture for X11Capture {
    async fn capture_all(&self) -> Result<Vec<CaptureResult>> {
        let (conn, _) = self.connect()?;
        let setup = conn.setup();
        let mut results = Vec::new();

        for (i, screen) in setup.roots.iter().enumerate() {
            let result = self.capture_root(&conn, screen.root, screen.width_in_pixels, screen.height_in_pixels, i == self.screen_num)?;
            results.push(result);
        }

        Ok(results)
    }

    async fn capture_cursor_screen(&self) -> Result<CaptureResult> {
        let (conn, _) = self.connect()?;
        let setup = conn.setup();
        let screen = &setup.roots[self.screen_num];
        self.capture_root(&conn, screen.root, screen.width_in_pixels, screen.height_in_pixels, true)
    }

    async fn capture_focused_window(&self) -> Result<CaptureResult> {
        let (conn, _) = self.connect()?;
        let setup = conn.setup();
        let screen = &setup.roots[self.screen_num];

        let active_window = get_active_window(&conn, screen.root)?;

        let (win, width, height) = if let Some(win) = active_window {
            let geo = get_geometry(&conn, win)?.reply()?;
            (win, geo.width, geo.height)
        } else {
            (screen.root, screen.width_in_pixels, screen.height_in_pixels)
        };

        let reply = get_image(&conn, ImageFormat::Z_PIXMAP, win, 0, 0, width, height, 0xFFFFFFFF)?.reply()?;
        let app_name = get_window_name(&conn, win);
        let jpeg = encode_to_jpeg(&reply.data, width as u32, height as u32)?;
        let (w, h) = dimensions(width as u32, height as u32);

        Ok(CaptureResult {
            image_data: jpeg,
            width: w,
            height: h,
            app_name,
            is_cursor_screen: true,
        })
    }
}

fn encode_to_jpeg(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let (new_w, new_h) = dimensions(width, height);

    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for chunk in data.chunks(4) {
        let b = chunk.get(0).copied().unwrap_or(0);
        let g = chunk.get(1).copied().unwrap_or(0);
        let r = chunk.get(2).copied().unwrap_or(0);
        let a = chunk.get(3).copied().unwrap_or(255);
        rgba.extend_from_slice(&[r, g, b, a]);
    }

    let img = image::RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| anyhow::anyhow!("failed to create image from raw data"))?;

    let resized = if new_w != width || new_h != height {
        image::imageops::resize(&img, new_w, new_h, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let rgb = image::DynamicImage::ImageRgba8(resized).to_rgb8();

    let mut buf = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, JPEG_QUALITY);
    encoder.encode(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)?;
    Ok(buf)
}

fn dimensions(width: u32, height: u32) -> (u32, u32) {
    let max = width.max(height);
    if max <= MAX_DIMENSION {
        return (width, height);
    }
    let scale = MAX_DIMENSION as f64 / max as f64;
    ((width as f64 * scale) as u32, (height as f64 * scale) as u32)
}

fn get_active_window(conn: &RustConnection, root: u32) -> Result<Option<u32>> {
    let atom = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW")?.reply()?;
    let prop = get_property(conn, false, root, atom.atom, AtomEnum::WINDOW, 0, 1)?.reply()?;
    if prop.value.len() < 4 {
        return Ok(None);
    }
    let win = u32::from_ne_bytes(prop.value[0..4].try_into()?);
    Ok(Some(win))
}

fn get_window_name(conn: &RustConnection, win: u32) -> Option<String> {
    let atom = conn.intern_atom(false, b"_NET_WM_NAME").ok()?.reply().ok()?;
    let prop = get_property(conn, false, win, atom.atom, AtomEnum::STRING, 0, 1024).ok()?.reply().ok()?;
    if prop.value.is_empty() {
        return None;
    }
    String::from_utf8(prop.value).ok()
}
