use anyhow::Result;
use async_trait::async_trait;
use futures::{SinkExt, Stream, StreamExt};
use rand::Rng;
use serde::Deserialize;
use std::pin::Pin;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::voice::tts::{AudioChunk, AudioStream, TtsProvider};

const EDGE_TTS_URL: &str = "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1";

pub struct EdgeTtsProvider {
    voice: String,
    rate: String,
}

impl EdgeTtsProvider {
    pub fn new(voice: String) -> Self {
        Self { voice, rate: "+0%".to_string() }
    }

    pub fn with_rate(voice: String, rate: String) -> Self {
        Self { voice, rate }
    }

    fn token(&self) -> String {
        std::env::var("EDGE_TTS_TOKEN").unwrap_or_else(|_| String::new())
    }

    fn connection_id() -> String {
        let mut rng = rand::rng();
        format!("{:08x}{:04x}{:04x}{:04x}{:08x}{:04x}",
            rng.random::<u32>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u32>(),
            rng.random::<u16>(),
        )
    }

    fn build_ssml(&self, text: &str) -> String {
        format!(
            "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'>\
             <voice name='{}'>\
             <prosody pitch='+0Hz' rate='{}' volume='+0%'>\
             {}\
             </prosody>\
             </voice>\
             </speak>",
            self.voice, self.rate, text
        )
    }
}

#[derive(Deserialize)]
struct ConfigMessage {
    #[serde(rename = "X-RequestId")]
    request_id: String,
}

#[async_trait]
impl TtsProvider for EdgeTtsProvider {
    async fn synthesize(&self, text: &str) -> Result<AudioChunk> {
        let conn_id = Self::connection_id();
        let url = format!("{}?TrustedClientToken={}&ConnectionId={}", EDGE_TTS_URL, self.token(), conn_id);
        let request_id = Self::connection_id();

        let (mut ws_stream, _) = connect_async(&url).await?;

        let config_msg = format!(
            "X-Timestamp:{}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n{{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"true\"}},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"}}}}}}}}",
            chrono_like_timestamp()
        );
        ws_stream.send(Message::Text(config_msg.into())).await?;

        let ssml = self.build_ssml(text);
        let ssml_msg = format!(
            "X-RequestId:{}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{}\r\nPath:ssml\r\n\r\n{}",
            request_id,
            chrono_like_timestamp(),
            ssml
        );
        ws_stream.send(Message::Text(ssml_msg.into())).await?;

        let mut audio_data = Vec::new();

        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    if data.len() > 2 {
                        let header_len = u16::from_be_bytes([data[0], data[1]]) as usize;
                        if data.len() > 2 + header_len {
                            audio_data.extend_from_slice(&data[2 + header_len..]);
                        }
                    }
                }
                Ok(Message::Text(text)) => {
                    if text.contains("Path:turn.end") {
                        break;
                    }
                }
                _ => break,
            }
        }

        Ok(audio_data)
    }

    async fn synthesize_stream(
        &self,
        text_stream: Pin<Box<dyn Stream<Item = String> + Send>>,
    ) -> Result<AudioStream> {
        let voice = self.voice.clone();
        let rate = self.rate.clone();

        let stream = text_stream.flat_map(move |text| {
            let voice = voice.clone();
            let rate = rate.clone();
            async_stream::stream! {
                let provider = EdgeTtsProvider::with_rate(voice, rate);
                match provider.synthesize(&text).await {
                    Ok(chunk) => yield Ok(chunk),
                    Err(e) => yield Err(e),
                }
            }
        }).boxed();

        Ok(Box::pin(stream) as AudioStream)
    }
}

fn chrono_like_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}{:03}", now.as_secs(), now.subsec_millis())
}
