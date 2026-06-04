use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

use super::transcription::{SttProvider, Transcript};

pub struct DeepgramSttProvider {
    api_key: String,
    client: Client,
}

impl DeepgramSttProvider {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { api_key, client }
    }

    pub fn transcribe_sync(&self, audio: &[f32], sample_rate: u32) -> Result<String> {
        let wav_data = encode_wav(audio, sample_rate);

        let rt = tokio::runtime::Runtime::new()?;
        let text = rt.block_on(async {
            self.transcribe_bytes(&wav_data).await
        })?;

        Ok(text)
    }

    async fn transcribe_bytes(&self, audio_data: &[u8]) -> Result<String> {
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

fn encode_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
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
