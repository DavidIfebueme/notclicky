use anyhow::Result;
use async_trait::async_trait;
use futures::{SinkExt, Stream, StreamExt};
use rand::Rng;
use sha2::{Sha256, Digest};
use std::pin::Pin;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use crate::voice::tts::{AudioChunk, AudioStream, TtsProvider};

const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const EDGE_TTS_URL: &str = "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1";
const SEC_MS_GEC_VERSION: &str = "1-143.0.3650.75";

pub struct EdgeTtsProvider {
    voice: String,
    rate: String,
}

impl EdgeTtsProvider {
    pub fn new(voice: String) -> Self {
        Self { voice, rate: "+0%".to_string() }
    }

    #[allow(dead_code)]
    pub fn with_rate(voice: String, rate: String) -> Self {
        Self { voice, rate }
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

    fn generate_sec_ms_gec() -> String {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let win_epoch_secs: u64 = 11644473600;
        let mut ticks_secs = now_secs + win_epoch_secs;
        ticks_secs -= ticks_secs % 300;

        let ticks_100ns = ticks_secs * 10_000_000;

        let str_to_hash = format!("{}{}", ticks_100ns, TRUSTED_CLIENT_TOKEN);
        let mut hasher = Sha256::new();
        hasher.update(str_to_hash.as_bytes());
        format!("{:X}", hasher.finalize())
    }

    fn generate_muid() -> String {
        let mut rng = rand::rng();
        format!("{:032X}", rng.random::<u128>())
    }

    fn build_ssml(&self, text: &str) -> String {
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;");

        format!(
            "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'>\
             <voice name='{}'>\
             <prosody pitch='+0Hz' rate='{}' volume='+0%'>\
             {}\
             </prosody>\
             </voice>\
             </speak>",
            self.voice, self.rate, escaped
        )
    }

    fn date_string() -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}{:03}", now.as_secs(), now.subsec_millis())
    }
}

#[async_trait]
impl TtsProvider for EdgeTtsProvider {
    async fn synthesize(&self, text: &str) -> Result<AudioChunk> {
        let conn_id = Self::connection_id();
        let sec_ms_gec = Self::generate_sec_ms_gec();
        let muid = Self::generate_muid();

        let url = format!(
            "{}?TrustedClientToken={}&ConnectionId={}&Sec-MS-GEC={}&Sec-MS-GEC-Version={}",
            EDGE_TTS_URL, TRUSTED_CLIENT_TOKEN, conn_id, sec_ms_gec, SEC_MS_GEC_VERSION
        );

        let mut req = url.into_client_request()?;
        let headers = req.headers_mut();
        headers.insert("Origin", "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold".parse()?);
        headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36 Edg/143.0.0.0".parse()?);
        headers.insert("Pragma", "no-cache".parse()?);
        headers.insert("Cache-Control", "no-cache".parse()?);
        headers.insert("Accept-Encoding", "gzip, deflate, br, zstd".parse()?);
        headers.insert("Accept-Language", "en-US,en;q=0.9".parse()?);
        headers.insert("Sec-WebSocket-Version", "13".parse()?);
        headers.insert("Cookie", format!("muid={}", muid).parse()?);

        let (mut ws_stream, _) = connect_async(req).await?;

        let config_msg = format!(
            "X-Timestamp:{}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n{{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"true\"}},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"}}}}}}}}",
            Self::date_string()
        );
        ws_stream.send(Message::Text(config_msg.into())).await?;

        let request_id = Self::connection_id();
        let ssml = self.build_ssml(text);
        let ssml_msg = format!(
            "X-RequestId:{}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{}Z\r\nPath:ssml\r\n\r\n{}",
            request_id,
            Self::date_string(),
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
