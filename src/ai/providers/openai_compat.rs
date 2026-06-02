use anyhow::Result;
use async_trait::async_trait;

use crate::ai::providers::LlmProvider;

pub struct OpenAiCompatProvider {
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAiCompatProvider {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        Self { base_url, api_key, model }
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatProvider {
    async fn complete(&self, _req: crate::ai::providers::LlmRequest) -> Result<crate::ai::providers::LlmResponse> {
        todo!()
    }

    async fn stream(&self, _req: crate::ai::providers::LlmRequest) -> Result<crate::ai::providers::LlmStream> {
        todo!()
    }
}
