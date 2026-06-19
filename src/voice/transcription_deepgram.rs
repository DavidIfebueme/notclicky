use anyhow::Result;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::transcription::{SttProvider, Transcript};

pub struct DeepgramSttProvider {
    api_key: String,
    client: Client,
}

#[derive(Deserialize, Debug)]
struct DeepgramResult {
    channel: DeepgramChannel,
}

#[derive(Deserialize, Debug)]
struct DeepgramChannel {
    alternatives: Vec<DeepgramAlternative>,
}

#[derive(Deserialize, Debug)]
struct DeepgramAlternative {
    transcript: String,
}

#[derive(Deserialize, Debug)]
struct DeepgramMessage {
    #[serde(rename = "type")]
    msg_type: String,
    channel: Option<DeepgramResult>,
}

impl DeepgramSttProvider {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { api_key, client }
    }

    #[allow(dead_code)]
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub async fn transcribe_bytes(&self, audio_data: &[u8]) -> Result<String> {
        let url = "https://api.deepgram.com/v1/listen?model=nova-2&language=en&punctuate=true&smart_format=true";
        let resp = self.client
            .post(url)
            .header("Authorization", format!("Token {}", self.api_key))
            .header("Content-Type", "audio/wav")
            .body(audio_data.to_vec())
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Deepgram STT error {}: {}", status, body);
        }

        let json: serde_json::Value = resp.json().await?;
        let text = json["results"]["channels"][0]["alternatives"][0]["transcript"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();
        Ok(text)
    }
}

#[async_trait]
impl SttProvider for DeepgramSttProvider {
    async fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<Transcript> {
        let wav_data = encode_wav(audio, sample_rate);
        let text = self.transcribe_bytes(&wav_data).await?;
        Ok(Transcript {
            text,
            _is_final: true,
        })
    }
}

pub struct DeepgramStreamingSession {
    ws_stream: futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    ws_sink: futures::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
}

impl DeepgramStreamingSession {
    pub async fn connect(api_key: &str) -> Result<Self> {
        let url = format!(
            "wss://api.deepgram.com/v1/listen?model=nova-2&language=en&punctuate=true&smart_format=true&interim_results=true&endpointing=300&utterance_end_ms=1000"
        );

        let mut request = tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(&url)?;
        let headers = request.headers_mut();
        headers.insert("Authorization", format!("Token {}", api_key).parse()?);

        let (ws, _) = connect_async(request).await?;
        let (ws_sink, ws_stream) = ws.split();

        Ok(Self { ws_stream, ws_sink })
    }

    pub async fn send_audio(&mut self, audio: &[f32]) -> Result<()> {
        let pcm = encode_pcm_i16(audio);
        self.ws_sink.send(Message::Binary(pcm.into())).await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn send_raw_audio(&mut self, data: &[u8]) -> Result<()> {
        self.ws_sink.send(Message::Binary(data.to_vec().into())).await?;
        Ok(())
    }

    pub async fn finalize(&mut self) -> Result<()> {
        self.ws_sink.send(Message::Text("{\"type\": \"Finalize\"}".into())).await?;
        Ok(())
    }

    pub async fn close(&mut self) -> Result<()> {
        self.ws_sink.send(Message::Close(None)).await?;
        Ok(())
    }

    pub async fn next_transcript(&mut self) -> Option<Result<StreamingTranscript>> {
        while let Some(msg) = self.ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(parsed) = serde_json::from_str::<DeepgramMessage>(&text) {
                        match parsed.msg_type.as_str() {
                            "Results" | "Transcript" => {
                                if let Some(channel) = parsed.channel {
                                    if let Some(alt) = channel.channel.alternatives.first() {
                                        let is_final = parsed.msg_type == "Results";
                                        return Some(Ok(StreamingTranscript {
                                            text: alt.transcript.trim().to_string(),
                                            is_final,
                                        }));
                                    }
                                }
                            }
                            "UtteranceEnd" => {
                                return None;
                            }
                            _ => continue,
                        }
                    }
                }
                Ok(Message::Close(_)) => return None,
                Err(e) => return Some(Err(anyhow::anyhow!("WebSocket error: {}", e))),
                _ => continue,
            }
        }
        None
    }
}

pub struct StreamingTranscript {
    pub text: String,
    pub is_final: bool,
}

fn encode_pcm_i16(samples: &[f32]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(samples.len() * 2);
    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let val = (clamped * 32767.0) as i16;
        buf.extend_from_slice(&val.to_le_bytes());
    }
    buf
}

pub struct DeepgramWakeWordSession {
    ws_stream: futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    ws_sink: futures::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    _sample_rate: u32,
}

impl DeepgramWakeWordSession {
    pub async fn connect(api_key: &str, sample_rate: u32) -> Result<Self> {
        let url = format!(
            "wss://api.deepgram.com/v1/listen?model=nova-2&language=en&punctuate=true&smart_format=true&interim_results=true&endpointing=300&sample_rate={}&channels=1&encoding=linear16",
            sample_rate
        );

        let mut request = tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(&url)?;
        let headers = request.headers_mut();
        headers.insert("Authorization", format!("Token {}", api_key).parse()?);

        let (ws, _) = connect_async(request).await?;
        let (ws_sink, ws_stream) = ws.split();

        Ok(Self {
            ws_stream,
            ws_sink,
            _sample_rate: sample_rate,
        })
    }

    pub async fn send_audio(&mut self, audio: &[f32]) -> Result<()> {
        if audio.is_empty() {
            return Ok(());
        }
        let pcm = encode_pcm_i16(audio);
        self.ws_sink.send(Message::Binary(pcm.into())).await?;
        Ok(())
    }

    pub async fn next_transcript(&mut self) -> Option<Result<WakeWordTranscript>> {
        while let Some(msg) = self.ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(parsed) = serde_json::from_str::<DeepgramMessage>(&text) {
                        match parsed.msg_type.as_str() {
                            "Results" | "Transcript" => {
                                if let Some(channel) = parsed.channel {
                                    if let Some(alt) = channel.channel.alternatives.first() {
                                        let transcript_text = alt.transcript.trim().to_string();
                                        if !transcript_text.is_empty() {
                                            eprintln!("notclicky: wake word heard \"{}\" (final={})", transcript_text, parsed.msg_type == "Results");
                                        }
                                        let is_final = parsed.msg_type == "Results";
                                        return Some(Ok(WakeWordTranscript {
                                            text: transcript_text,
                                            is_final,
                                        }));
                                    }
                                }
                            }
                            _ => continue,
                        }
                    }
                }
                Ok(Message::Close(_)) => return None,
                Err(e) => return Some(Err(anyhow::anyhow!("WebSocket error: {}", e))),
                _ => continue,
            }
        }
        None
    }

    pub async fn close(&mut self) -> Result<()> {
        self.ws_sink.send(Message::Close(None)).await?;
        Ok(())
    }
}

pub struct WakeWordTranscript {
    pub text: String,
    pub is_final: bool,
}

pub fn encode_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
    let block_align = num_channels * bits_per_sample / 8;
    let data_size = samples.len() as u32 * 2;
    let file_size = 36 + data_size;

    let mut buf = Vec::with_capacity(44 + data_size as usize);

    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&num_channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());

    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let val = (clamped * 32767.0) as i16;
        buf.extend_from_slice(&val.to_le_bytes());
    }

    buf
}
