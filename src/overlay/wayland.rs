use anyhow::Result;

use crate::overlay::cursor::OverlayCommand;

pub struct WaylandOverlay {
    notify_available: bool,
}

impl WaylandOverlay {
    pub fn new() -> Result<Self> {
        let notify_available = std::process::Command::new("which")
            .arg("notify-send")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        Ok(Self { notify_available })
    }

    pub fn send(&self, cmd: OverlayCommand) -> Result<()> {
        match cmd {
            OverlayCommand::ShowCursor(point, label, _) => {
                self.notify("NotClicky", &format!("Pointing at ({}, {}){}", point.x, point.y, if label.is_empty() { String::new() } else { format!(": {}", label) }));
            }
            OverlayCommand::ShowCursors(points, _, _) => {
                let labels: Vec<String> = points.iter().map(|p| format!("({},{})", p.x, p.y)).collect();
                self.notify("NotClicky", &format!("Showing {} cursors: {}", points.len(), labels.join(", ")));
            }
            OverlayCommand::ShowScribble(_points, _, _) => {
                self.notify("NotClicky", "Drawing scribble overlay (Wayland: notification only)");
            }
            OverlayCommand::ShowHighlight(rect, _, _) => {
                self.notify("NotClicky", &format!("Highlighting area at ({},{}) {}x{}", rect.x, rect.y, rect.width, rect.height));
            }
            OverlayCommand::ShowCaption(text, x, y, _, _) => {
                self.notify("NotClicky", &format!("Caption at ({},{}): {}", x, y, text));
            }
            OverlayCommand::NavigateCursor(x, y, _) => {
                self.notify("NotClicky", &format!("Navigating to ({}, {})", x, y));
            }
            OverlayCommand::ShowWaveform(_rms) => {}
            OverlayCommand::HideWaveform => {}
            OverlayCommand::Clear => {}
        }
        Ok(())
    }

    fn notify(&self, title: &str, body: &str) {
        if !self.notify_available {
            return;
        }
        let _ = std::process::Command::new("notify-send")
            .args([title, body])
            .spawn();
    }
}
