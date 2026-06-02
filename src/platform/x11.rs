use anyhow::Result;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

use crate::voice::push_to_talk::GlobalHotkey;

fn ptt_modifiers() -> ModMask {
    ModMask::CONTROL | ModMask::M1
}

fn grab_mods() -> Vec<ModMask> {
    let base = ptt_modifiers();
    vec![
        base,
        base | ModMask::LOCK,
        base | ModMask::M2,
        base | ModMask::LOCK | ModMask::M2,
    ]
}

pub struct X11Hotkey {
    pressed: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
}

impl X11Hotkey {
    pub fn new() -> Result<Self> {
        Ok(Self {
            pressed: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(false)),
        })
    }
}

#[async_trait]
impl GlobalHotkey for X11Hotkey {
    fn register(&self, _modifiers: Vec<&str>, _key: Option<&str>) -> Result<()> {
        let (conn, screen_num) = RustConnection::connect(None)?;
        let screen = conn.setup().roots[screen_num].clone();
        let root = screen.root;

        for mods in grab_mods() {
            grab_key(&conn, false, root, mods, 0u8, GrabMode::ASYNC, GrabMode::ASYNC)?;
        }
        conn.flush()?;

        let pressed = self.pressed.clone();
        let running = self.running.clone();
        let required = ptt_modifiers();

        running.store(true, Ordering::SeqCst);

        std::thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                match conn.wait_for_event() {
                    Ok(event) => match event {
                        Event::KeyPress(_) => {
                            pressed.store(true, Ordering::SeqCst);
                        }
                        Event::KeyRelease(ev) => {
                            let active = ModMask::from(u16::from(ev.state));
                            if (active & required) != required {
                                pressed.store(false, Ordering::SeqCst);
                            }
                        }
                        _ => {}
                    },
                    Err(_) => break,
                }
            }
            for mods in grab_mods() {
                let _ = ungrab_key(&conn, 0u8, root, mods);
            }
            let _ = conn.flush();
        });

        Ok(())
    }

    fn unregister(&self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_pressed(&self) -> bool {
        self.pressed.load(Ordering::SeqCst)
    }
}

impl Drop for X11Hotkey {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }
}
