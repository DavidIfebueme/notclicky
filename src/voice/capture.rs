use anyhow::Result;
use pipewire as pw;
use pw::spa;
use pw::spa::pod::Pod;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct AudioCapture {
    buffer: Arc<Mutex<Vec<f32>>>,
    capturing: Arc<AtomicBool>,
    sample_rate: u32,
    rms_callback: Arc<Mutex<Option<Box<dyn Fn(f32) + Send + 'static>>>>,
}

struct UserData {
    format: spa::param::audio::AudioInfoRaw,
}

impl AudioCapture {
    pub fn new(sample_rate: u32) -> Self {
        let capturing = Arc::new(AtomicBool::new(false));
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let rms_callback = Arc::new(Mutex::new(None));

        let cap = capturing.clone();
        let buf = buffer.clone();
        let rms = rms_callback.clone();
        let rate = sample_rate;

        std::thread::spawn(move || {
            if let Err(e) = run_pipewire_loop(buf, cap, rms, rate) {
                eprintln!("PipeWire capture error: {}", e);
            }
        });

        Self {
            buffer,
            capturing,
            sample_rate,
            rms_callback,
        }
    }

    pub fn start(&self) -> Result<()> {
        self.capturing.store(true, Ordering::SeqCst);
        self.buffer.lock().unwrap().clear();
        Ok(())
    }

    pub fn stop(&self) -> Vec<f32> {
        self.capturing.store(false, Ordering::SeqCst);
        std::mem::take(&mut self.buffer.lock().unwrap())
    }

    #[allow(dead_code)]
    pub fn snapshot(&self) -> Vec<f32> {
        self.buffer.lock().unwrap().clone()
    }

    #[allow(dead_code)]
    pub fn is_capturing(&self) -> bool {
        self.capturing.load(Ordering::SeqCst)
    }

    pub fn set_rms_callback(&self, cb: Box<dyn Fn(f32) + Send + 'static>) {
        *self.rms_callback.lock().unwrap() = Some(cb);
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

fn run_pipewire_loop(
    buffer: Arc<Mutex<Vec<f32>>>,
    capturing: Arc<AtomicBool>,
    rms_callback: Arc<Mutex<Option<Box<dyn Fn(f32) + Send + 'static>>>>,
    target_sample_rate: u32,
) -> Result<()> {
    pw::init();

    let mainloop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None)?;

    let data = UserData {
        format: Default::default(),
    };

    let props = pw::properties::properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Voice",
    };

    let stream = pw::stream::StreamBox::new(&core, "notclicky-capture", props)?;

    let buffer_clone = buffer.clone();
    let capturing_clone = capturing.clone();
    let rms_clone = rms_callback.clone();

    let _listener = stream
        .add_local_listener_with_user_data(data)
        .param_changed(|_, user_data, id, param| {
            let Some(param) = param else { return };
            if id != spa::param::ParamType::Format.as_raw() {
                return;
            }
            let (media_type, media_subtype) = match pw::spa::param::format_utils::parse_format(param) {
                Ok(v) => v,
                Err(_) => return,
            };
            if media_type != spa::param::format::MediaType::Audio
                || media_subtype != spa::param::format::MediaSubtype::Raw
            {
                return;
            }
            let _ = user_data.format.parse(param);
        })
        .process(move |stream, user_data| {
            if !capturing_clone.load(Ordering::SeqCst) {
                return;
            }

            if let Some(mut buf) = stream.dequeue_buffer() {
                let datas = buf.datas_mut();
                if datas.is_empty() {
                    return;
                }
                let data = &mut datas[0];
                let n_samples = data.chunk().size() / (std::mem::size_of::<f32>() as u32);

                if let Some(samples) = data.data() {
                    let floats: Vec<f32> = (0..n_samples)
                        .map(|i| {
                            let start = i as usize * std::mem::size_of::<f32>();
                            let end = start + std::mem::size_of::<f32>();
                            f32::from_le_bytes(samples[start..end].try_into().unwrap())
                        })
                        .collect();

                    if !floats.is_empty() {
                        let rms = compute_rms(&floats);
                        if let Some(ref cb) = *rms_clone.lock().unwrap() {
                            cb(rms);
                        }

                        let n_channels = user_data.format.channels().max(1) as usize;
                        let mono: Vec<f32> = if n_channels > 1 {
                            floats
                                .chunks(n_channels)
                                .map(|ch| ch.iter().sum::<f32>() / n_channels as f32)
                                .collect()
                        } else {
                            floats
                        };

                        buffer_clone.lock().unwrap().extend_from_slice(&mono);
                    }
                }
            }
        })
        .register()?;

    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
    audio_info.set_rate(target_sample_rate);
    audio_info.set_channels(1);

    let obj = spa::pod::Object {
        type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: spa::param::ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };

    let values: Vec<u8> = spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(obj),
    )
    .unwrap()
    .0
    .into_inner();

    let mut params = [Pod::from_bytes(&values).unwrap()];

    stream.connect(
        spa::utils::Direction::Input,
        None,
        pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
        &mut params,
    )?;

    mainloop.run();

    Ok(())
}

pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}
