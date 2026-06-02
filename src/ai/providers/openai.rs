use anyhow::Result;
use async_trait::async_trait;

use crate::ai::providers::LlmProvider;

pub struct OpenAiProvider {
    api_key: String,
    model: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(&self, _req: crate::ai::providers::LlmRequest) -> Result<crate::ai::providers::LlmResponse> {
        todo!()
    }

    async fn stream(&self, _req: crate::ai::providers::LlmRequest) -> Result<crate::ai::providers::LlmStream> {
        todo!()
    }
}
