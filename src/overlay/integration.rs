use crate::ai::point_parser;
use crate::overlay::cursor::OverlayCommand;

pub fn process_stream_token(token: &str, overlay_tx: &std::sync::mpsc::Sender<OverlayCommand>) {
    let points = point_parser::parse_points(token);
    for p in points {
        let cmd = OverlayCommand::NavigateCursor(p.x as f64, p.y as f64, "blue".to_string());
        let _ = overlay_tx.send(cmd);
    }
}
