use std::sync::{Arc, Mutex};
use rodio::{OutputStream, OutputStreamHandle, Sink, Source};
use std::io::Cursor;

pub struct AudioPlayer {
    _stream: Option<OutputStream>,
    _stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    #[allow(dead_code)]
    queue: Arc<Mutex<Vec<Vec<u8>>>>,
    #[allow(dead_code)]
    playing: Arc<Mutex<bool>>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        Self {
            _stream: None,
            _stream_handle: None,
            sink: None,
            queue: Arc::new(Mutex::new(Vec::new())),
            playing: Arc::new(Mutex::new(false)),
        }
    }

    pub fn enqueue(&mut self, data: Vec<u8>) {
        self.ensure_output();
        if let Some(ref sink) = self.sink {
            if let Ok(source) = decode_mp3(&data) {
                sink.append(source);
            }
        }
    }

    pub fn play_blocking(data: &[u8]) -> anyhow::Result<()> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        let source = decode_mp3(data)?;
        sink.append(source);
        sink.sleep_until_end();
        Ok(())
    }

    pub fn is_playing(&self) -> bool {
        self.sink.as_ref().map_or(false, |s| !s.empty())
    }

    #[allow(dead_code)]
    pub fn stop(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.stop();
        }
    }

    fn ensure_output(&mut self) {
        if self._stream.is_none() {
            if let Ok((stream, handle)) = OutputStream::try_default() {
                if let Ok(sink) = Sink::try_new(&handle) {
                    self._stream = Some(stream);
                    self._stream_handle = Some(handle);
                    self.sink = Some(sink);
                }
            }
        }
    }
}

fn decode_mp3(data: &[u8]) -> anyhow::Result<Box<dyn Source<Item = i16> + Send>> {
    let cursor = Cursor::new(data.to_vec());
    let source = rodio::Decoder::new(cursor)?;
    Ok(Box::new(source.convert_samples()))
}
