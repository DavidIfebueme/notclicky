use anyhow::Result;
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

use crate::ai::providers::{LlmMessage, LlmProvider, LlmRequest, LlmResponse, LlmStream};

#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    messages: Vec<ApiMessage>,
    max_tokens: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
    model: String,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

#[derive(Deserialize)]
struct StreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<StreamDelta>,
}

#[derive(Deserialize)]
struct StreamDelta {
    text: Option<String>,
}

pub struct AnthropicProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Self {
        let client = reqwest::Client::new();
        Self { api_key, model, client }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse> {
        let model = req.model.clone().unwrap_or_else(|| self.model.clone());
        let messages: Vec<ApiMessage> = req.messages.iter().map(|m| ApiMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        }).collect();

        let api_req = MessagesRequest {
            model,
            messages,
            max_tokens: req.max_tokens.unwrap_or(4096),
            stream: false,
            temperature: req.temperature,
        };

        let resp = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&api_req)
            .send()
            .await?;

        let api_resp: MessagesResponse = resp.json().await?;

        let content = api_resp.content.iter()
            .filter_map(|b| b.text.clone())
            .collect::<Vec<_>>()
            .join("");

        Ok(LlmResponse {
            content,
            model: api_resp.model,
        })
    }

    async fn stream(&self, req: LlmRequest) -> Result<LlmStream> {
        let model = req.model.clone().unwrap_or_else(|| self.model.clone());
        let messages: Vec<ApiMessage> = req.messages.iter().map(|m| ApiMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        }).collect();

        let api_req = MessagesRequest {
            model,
            messages,
            max_tokens: req.max_tokens.unwrap_or(4096),
            stream: true,
            temperature: req.temperature,
        };

        let resp = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&api_req)
            .send()
            .await?;

        let stream = resp.bytes_stream()
            .eventsource()
            .filter_map(move |event| {
                let result = match event {
                    Ok(event) => {
                        let stream_event: StreamEvent = match serde_json::from_str(&event.data) {
                            Ok(e) => e,
                            Err(_) => return std::future::ready(None),
                        };
                        if stream_event.event_type == "content_block_delta" {
                            stream_event.delta.and_then(|d| d.text.map(Ok))
                        } else {
                            None
                        }
                    }
                    Err(e) => Some(Err(anyhow::anyhow!("SSE error: {}", e))),
                };
                std::future::ready(result)
            })
            .boxed();

        Ok(Box::pin(stream) as LlmStream)
    }
}
