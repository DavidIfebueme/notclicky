use anyhow::Result;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

use crate::voice::push_to_talk::GlobalHotkey;

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

fn modifier_index(mask: ModMask) -> usize {
    let bits: u16 = mask.into();
    bits.trailing_zeros() as usize
}

fn get_keycodes_for_mod(conn: &impl Connection, mask: ModMask) -> Result<Vec<u8>> {
    let reply = conn.get_modifier_mapping()?.reply()?;
    let kpm = reply.keycodes_per_modifier() as usize;
    let start = modifier_index(mask) * kpm;
    let keycodes: Vec<u8> = reply.keycodes[start..start + kpm].iter().copied().filter(|&k| k != 0).collect();
    Ok(keycodes)
}

fn is_key_down(keys: &[u8; 32], keycode: u8) -> bool {
    (keys[(keycode / 8) as usize] >> (keycode % 8)) & 1 == 1
}

#[async_trait]
impl GlobalHotkey for X11Hotkey {
    fn register(&self, _modifiers: Vec<&str>, _key: Option<&str>) -> Result<()> {
        let (conn, _screen_num) = RustConnection::connect(None)?;
        let ctrl_keycodes = get_keycodes_for_mod(&conn, ModMask::CONTROL)?;
        let alt_keycodes = get_keycodes_for_mod(&conn, ModMask::M1)?;

        let pressed = self.pressed.clone();
        let running = self.running.clone();

        if ctrl_keycodes.is_empty() || alt_keycodes.is_empty() {
            anyhow::bail!("could not find Control or Alt keycodes via modifier mapping");
        }

        running.store(true, Ordering::SeqCst);

        std::thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                match conn.query_keymap() {
                    Ok(cookie) => match cookie.reply() {
                        Ok(reply) => {
                            let keymap = reply.keys;
                            let ctrl_down = ctrl_keycodes.iter().any(|&kc| is_key_down(&keymap, kc));
                            let alt_down = alt_keycodes.iter().any(|&kc| is_key_down(&keymap, kc));
                            pressed.store(ctrl_down && alt_down, Ordering::SeqCst);
                        }
                        Err(_) => break,
                    },
                    Err(_) => break,
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
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
