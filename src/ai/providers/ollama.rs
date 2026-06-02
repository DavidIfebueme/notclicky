use anyhow::Result;
use async_trait::async_trait;

use crate::ai::providers::LlmProvider;

pub struct OllamaProvider {
    model: String,
}

impl OllamaProvider {
    pub fn new(model: String) -> Self {
        Self { model }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, _req: crate::ai::providers::LlmRequest) -> Result<crate::ai::providers::LlmResponse> {
        todo!()
    }

    async fn stream(&self, _req: crate::ai::providers::LlmRequest) -> Result<crate::ai::providers::LlmStream> {
        todo!()
    }
}
