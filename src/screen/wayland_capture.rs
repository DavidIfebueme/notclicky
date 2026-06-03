use anyhow::Result;
use async_trait::async_trait;

use crate::screen::capture::{CaptureResult, ScreenCapture};

pub struct WaylandCapture {
    gdbus_available: bool,
}

impl WaylandCapture {
    pub fn new() -> Result<Self> {
        let gdbus_available = std::process::Command::new("which")
            .arg("gdbus")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        Ok(Self { gdbus_available })
    }
}

#[async_trait]
impl ScreenCapture for WaylandCapture {
    async fn capture_all(&self) -> Result<Vec<CaptureResult>> {
        let path = self.request_screenshot()?;
        let data = tokio::fs::read(&path).await?;
        let _ = std::fs::remove_file(&path);
        Ok(vec![CaptureResult {
            image_data: data,
            width: 0,
            height: 0,
            app_name: None,
            is_cursor_screen: true,
        }])
    }

    async fn capture_cursor_screen(&self) -> Result<CaptureResult> {
        let results = self.capture_all().await?;
        results.into_iter().next().ok_or_else(|| anyhow::anyhow!("No screenshot captured"))
    }

    async fn capture_focused_window(&self) -> Result<CaptureResult> {
        self.capture_cursor_screen().await
    }
}

impl WaylandCapture {
    fn request_screenshot(&self) -> Result<String> {
        if !self.gdbus_available {
            anyhow::bail!("gdbus not available for Wayland screenshot. Install gdbus or use X11.");
        }

        let output = std::process::Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest", "org.freedesktop.portal.Desktop",
                "--object-path", "/org/freedesktop/portal/desktop",
                "--method", "org.freedesktop.portal.Screenshot.Screenshot",
                "",
                "{}",
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Portal screenshot failed: {}", stderr);
        }

        let response = String::from_utf8_lossy(&output.stdout);
        let path = parse_portal_response(&response)?;
        wait_for_file(&path, 5000)?;
        Ok(path)
    }
}

fn parse_portal_response(response: &str) -> Result<String> {
    let response = response.trim();

    if let Some(start) = response.find("'/") {
        let path = &response[start + 1..];
        let end = path.find('\'').unwrap_or(path.len());
        return Ok(format!("/{}", &path[..end]));
    }

    if let Some(start) = response.find("file://") {
        let path = &response[start + 7..];
        let end = path.find('\'').unwrap_or(path.len());
        return Ok(path[..end].to_string());
    }

    if let Some(start) = response.find('"') {
        let rest = &response[start + 1..];
        let end = rest.find('"').unwrap_or(rest.len());
        return Ok(rest[..end].to_string());
    }

    anyhow::bail!("Could not parse portal response: {}", response)
}

fn wait_for_file(path: &str, timeout_ms: u64) -> Result<()> {
    let start = std::time::Instant::now();
    while start.elapsed().as_millis() < u128::from(timeout_ms) {
        if std::path::Path::new(path).exists() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    anyhow::bail!("Timed out waiting for screenshot file: {}", path)
}
