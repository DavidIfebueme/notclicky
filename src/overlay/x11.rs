use anyhow::Result;
use gtk4::gdk::{Display, Monitor};
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, DrawingArea};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use crate::overlay::cursor::{OverlayCommand, Point, Rect};

const ANIM_DURATION_MS: u64 = 400;
const TICK_MS: u64 = 16;

struct OverlayState {
    cursors: Vec<CursorState>,
    highlights: Vec<HighlightState>,
    captions: Vec<CaptionState>,
    scribbles: Vec<ScribbleState>,
}

struct CursorState {
    x: f64,
    y: f64,
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    label: Option<String>,
    accent: String,
    scale: f64,
    anim_progress: f64,
    animating: bool,
}

struct HighlightState {
    rect: Rect,
    accent: String,
}

struct CaptionState {
    text: String,
    x: f64,
    y: f64,
    accent: String,
}

struct ScribbleState {
    points: Vec<Point>,
    accent: String,
}

pub struct X11Overlay {
    tx: mpsc::Sender<OverlayCommand>,
}

impl X11Overlay {
    pub fn new(app: &gtk4::Application) -> Result<Self> {
        let (tx, rx) = mpsc::channel::<OverlayCommand>();

        let window = ApplicationWindow::builder()
            .application(app)
            .title("notclicky-overlay")
            .decorated(false)
            .resizable(false)
            .build();

        let display = Display::default();
        let (max_w, max_h) = if let Some(d) = display {
            let monitors = d.monitors();
            let n = monitors.n_items();
            let mut mw = 0i32;
            let mut mh = 0i32;
            for i in 0..n {
                if let Some(monitor) = monitors.item(i).and_downcast::<Monitor>() {
                    let geo = monitor.geometry();
                    mw = mw.max(geo.x() + geo.width());
                    mh = mh.max(geo.y() + geo.height());
                }
            }
            (mw.max(1920), mh.max(1080))
        } else {
            (1920, 1080)
        };

        window.set_default_size(max_w, max_h);

        let state = Rc::new(RefCell::new(OverlayState {
            cursors: Vec::new(),
            highlights: Vec::new(),
            captions: Vec::new(),
            scribbles: Vec::new(),
        }));

        let drawing_area = DrawingArea::new();
        let state_clone = state.clone();
        drawing_area.set_draw_func(move |_area, cr, _w, _h| {
            let s = state_clone.borrow();
            for highlight in &s.highlights {
                let (r, g, b) = parse_accent(&highlight.accent);
                cr.set_source_rgba(r, g, b, 0.15);
                cr.rectangle(highlight.rect.x, highlight.rect.y, highlight.rect.width, highlight.rect.height);
                let _ = cr.fill();
                cr.set_source_rgba(r, g, b, 0.6);
                cr.set_line_width(2.0);
                cr.rectangle(highlight.rect.x, highlight.rect.y, highlight.rect.width, highlight.rect.height);
                let _ = cr.stroke();
            }

            for scribble in &s.scribbles {
                if scribble.points.len() < 2 {
                    continue;
                }
                let (r, g, b) = parse_accent(&scribble.accent);
                cr.set_source_rgba(r, g, b, 0.8);
                cr.set_line_width(2.0);
                let first = &scribble.points[0];
                cr.move_to(first.x, first.y);
                for pt in &scribble.points[1..] {
                    cr.line_to(pt.x, pt.y);
                }
                let _ = cr.stroke();
            }

            for cursor in &s.cursors {
                let (r, g, b) = parse_accent(&cursor.accent);
                let x = cursor.x;
                let y = cursor.y;
                let size = 20.0 * cursor.scale;

                cr.set_source_rgba(r, g, b, 0.9);
                cr.move_to(x, y - size);
                cr.line_to(x - size * 0.6, y + size * 0.4);
                cr.line_to(x + size * 0.6, y + size * 0.4);
                cr.close_path();
                let _ = cr.fill();

                if let Some(ref label) = cursor.label {
                    cr.set_source_rgba(r, g, b, 0.9);
                    cr.set_font_size(12.0);
                    let _ = cr.move_to(x, y - size - 10.0);
                    let _ = cr.show_text(label);
                }
            }

            for caption in &s.captions {
                let (r, g, b) = parse_accent(&caption.accent);
                cr.set_source_rgba(r, g, b, 0.9);
                cr.set_font_size(14.0);
                let _ = cr.move_to(caption.x, caption.y);
                let _ = cr.show_text(&caption.text);
            }
        });

        window.set_child(Some(&drawing_area));
        window.set_visible(false);

        let window_clone = window.clone();
        let drawing_area_clone = drawing_area.clone();
        let state_clone2 = state.clone();
        gtk4::glib::idle_add_local(move || {
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    OverlayCommand::ShowCursor(point, accent, _duration) => {
                        state_clone2.borrow_mut().cursors.push(CursorState {
                            x: point.x,
                            y: point.y,
                            start_x: point.x,
                            start_y: point.y,
                            target_x: point.x,
                            target_y: point.y,
                            label: point.label,
                            accent,
                            scale: 1.0,
                            anim_progress: 1.0,
                            animating: false,
                        });
                        window_clone.set_visible(true);
                        window_clone.present();
                        drawing_area_clone.queue_draw();
                    }
                    OverlayCommand::ShowCursors(points, accent, _duration) => {
                        for p in points {
                            state_clone2.borrow_mut().cursors.push(CursorState {
                                x: p.x,
                                y: p.y,
                                start_x: p.x,
                                start_y: p.y,
                                target_x: p.x,
                                target_y: p.y,
                                label: p.label,
                                accent: accent.clone(),
                                scale: 1.0,
                                anim_progress: 1.0,
                                animating: false,
                            });
                        }
                        window_clone.set_visible(true);
                        window_clone.present();
                        drawing_area_clone.queue_draw();
                    }
                    OverlayCommand::NavigateCursor(tx, ty, _accent) => {
                        let mut s = state_clone2.borrow_mut();
                        if let Some(cursor) = s.cursors.last_mut() {
                            cursor.start_x = cursor.x;
                            cursor.start_y = cursor.y;
                            cursor.target_x = tx;
                            cursor.target_y = ty;
                            cursor.anim_progress = 0.0;
                            cursor.animating = true;
                        }
                        drop(s);
                    }
                    OverlayCommand::ShowCaption(text, x, y, accent, _duration) => {
                        state_clone2.borrow_mut().captions.push(CaptionState {
                            text, x, y, accent,
                        });
                        window_clone.set_visible(true);
                        window_clone.present();
                        drawing_area_clone.queue_draw();
                    }
                    OverlayCommand::ShowHighlight(rect, accent, _duration) => {
                        state_clone2.borrow_mut().highlights.push(HighlightState {
                            rect, accent,
                        });
                        window_clone.set_visible(true);
                        window_clone.present();
                        drawing_area_clone.queue_draw();
                    }
                    OverlayCommand::ShowScribble(points, accent, _duration) => {
                        state_clone2.borrow_mut().scribbles.push(ScribbleState {
                            points, accent,
                        });
                        window_clone.set_visible(true);
                        window_clone.present();
                        drawing_area_clone.queue_draw();
                    }
                    OverlayCommand::Clear => {
                        state_clone2.borrow_mut().cursors.clear();
                        state_clone2.borrow_mut().highlights.clear();
                        state_clone2.borrow_mut().captions.clear();
                        state_clone2.borrow_mut().scribbles.clear();
                        window_clone.set_visible(false);
                        drawing_area_clone.queue_draw();
                    }
                }
            }
            gtk4::glib::ControlFlow::Continue
        });

        let state_anim = state.clone();
        let drawing_area_anim = drawing_area.clone();
        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(TICK_MS), move || {
            let mut s = state_anim.borrow_mut();
            let mut any_animating = false;
            for cursor in s.cursors.iter_mut() {
                if cursor.animating {
                    cursor.anim_progress += TICK_MS as f64 / ANIM_DURATION_MS as f64;
                    if cursor.anim_progress >= 1.0 {
                        cursor.anim_progress = 1.0;
                        cursor.animating = false;
                        cursor.x = cursor.target_x;
                        cursor.y = cursor.target_y;
                        cursor.scale = 1.0;
                    } else {
                        any_animating = true;
                        let t = ease_out_cubic(cursor.anim_progress);
                        let arc_height = -80.0;
                        cursor.x = cursor.start_x + (cursor.target_x - cursor.start_x) * t;
                        cursor.y = cursor.start_y + (cursor.target_y - cursor.start_y) * t + arc_height * (4.0 * t * (1.0 - t));
                        cursor.scale = 1.0 + 0.3 * (1.0 - (2.0 * cursor.anim_progress - 1.0).abs());
                    }
                }
            }
            drop(s);
            if any_animating {
                drawing_area_anim.queue_draw();
            }
            gtk4::glib::ControlFlow::Continue
        });

        Ok(Self { tx })
    }

    pub fn send(&self, cmd: OverlayCommand) -> Result<()> {
        self.tx.send(cmd)?;
        Ok(())
    }
}

fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(3)
}

fn parse_accent(accent: &str) -> (f64, f64, f64) {
    match accent.to_lowercase().as_str() {
        "blue" => (0.2, 0.6, 1.0),
        "green" => (0.3, 0.9, 0.4),
        "orange" => (1.0, 0.6, 0.2),
        "purple" => (0.7, 0.3, 0.9),
        "red" => (1.0, 0.3, 0.3),
        _ => (0.2, 0.6, 1.0),
    }
}
