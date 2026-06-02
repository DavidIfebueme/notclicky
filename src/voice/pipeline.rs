use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::voice::capture::AudioCapture;
use crate::voice::push_to_talk::GlobalHotkey;

type AudioCallback = Box<dyn Fn(Vec<f32>) + Send + 'static>;

pub struct VoicePipeline {
    hotkey: Arc<Mutex<Box<dyn GlobalHotkey>>>,
    capture: Arc<Mutex<AudioCapture>>,
    running: Arc<AtomicBool>,
    on_audio: Arc<Mutex<Option<AudioCallback>>>,
}

impl VoicePipeline {
    pub fn new(hotkey: Box<dyn GlobalHotkey>, capture: AudioCapture) -> Self {
        Self {
            hotkey: Arc::new(Mutex::new(hotkey)),
            capture: Arc::new(Mutex::new(capture)),
            running: Arc::new(AtomicBool::new(false)),
            on_audio: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_on_audio(&self, cb: AudioCallback) {
        *self.on_audio.lock().unwrap() = Some(cb);
    }

    pub fn start(&self) -> anyhow::Result<()> {
        self.hotkey.lock().unwrap().register(vec!["Control", "Alt"], None)?;
        self.running.store(true, Ordering::SeqCst);

        let running = self.running.clone();
        let hotkey = self.hotkey.clone();
        let capture = self.capture.clone();
        let on_audio = self.on_audio.clone();

        std::thread::spawn(move || {
            let mut was_pressed = false;

            while running.load(Ordering::SeqCst) {
                let pressed = hotkey.lock().unwrap().is_pressed();

                if pressed && !was_pressed {
                    let _ = capture.lock().unwrap().start();
                    was_pressed = true;
                } else if !pressed && was_pressed {
                    let audio = capture.lock().unwrap().stop();
                    if !audio.is_empty() {
                        if let Some(ref cb) = *on_audio.lock().unwrap() {
                            cb(audio);
                        }
                    }
                    was_pressed = false;
                }

                std::thread::sleep(Duration::from_millis(10));
            }
        });

        Ok(())
    }

    pub fn stop(&self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        self.hotkey.lock().unwrap().unregister()
    }
}
