use anyhow::Result;
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

use crate::ai::providers::{LlmMessage, LlmProvider, LlmRequest, LlmResponse, LlmStream};

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    model: Option<String>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageContent,
}

#[derive(Deserialize)]
struct ChatMessageContent {
    content: String,
}

#[derive(Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
}

#[derive(Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

pub struct OpenAiCompatProvider {
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiCompatProvider {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        let client = reqwest::Client::new();
        Self { base_url, api_key, model, client }
    }

    fn chat_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{}/v1/chat/completions", base)
    }

    fn messages_from_request(&self, req: &LlmRequest) -> Vec<ChatMessage> {
        req.messages.iter().map(|m| ChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        }).collect()
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatProvider {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse> {
        let model = req.model.clone().unwrap_or_else(|| self.model.clone());
        let chat_req = ChatRequest {
            model,
            messages: self.messages_from_request(&req),
            stream: false,
            max_tokens: req.max_tokens,
            temperature: req.temperature,
        };

        let resp = self.client
            .post(self.chat_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&chat_req)
            .send()
            .await?;

        let chat_resp: ChatResponse = resp.json().await?;

        let content = chat_resp.choices.first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(LlmResponse {
            content,
            model: chat_resp.model.unwrap_or_else(|| self.model.clone()),
        })
    }

    async fn stream(&self, req: LlmRequest) -> Result<LlmStream> {
        let model = req.model.clone().unwrap_or_else(|| self.model.clone());
        let chat_req = ChatRequest {
            model,
            messages: self.messages_from_request(&req),
            stream: true,
            max_tokens: req.max_tokens,
            temperature: req.temperature,
        };

        let resp = self.client
            .post(self.chat_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&chat_req)
            .send()
            .await?;

        let stream = resp.bytes_stream()
            .eventsource()
            .map(move |event| {
                match event {
                    Ok(event) => {
                        if event.data == "[DONE]" {
                            return None;
                        }
                        let chunk: StreamChunk = serde_json::from_str(&event.data).ok()?;
                        let content = chunk.choices.first()?.delta.content.clone()?;
                        Some(Ok(content))
                    }
                    Err(e) => Some(Err(anyhow::anyhow!("SSE error: {}", e))),
                }
            })
            .filter_map(|opt| async move { opt })
            .boxed();

        Ok(Box::pin(stream) as LlmStream)
    }
}
