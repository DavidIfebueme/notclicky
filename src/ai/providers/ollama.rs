use crate::ai::providers::openai_compat::OpenAiCompatProvider;
use crate::ai::providers::LlmProvider;

pub struct OllamaProvider {
    inner: OpenAiCompatProvider,
}

impl OllamaProvider {
    pub fn new(model: String) -> Self {
        let inner = OpenAiCompatProvider::new(
            "http://localhost:11434".to_string(),
            String::new(),
            model,
        );
        Self { inner }
    }
}

#[async_trait::async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, req: crate::ai::providers::LlmRequest) -> anyhow::Result<crate::ai::providers::LlmResponse> {
        self.inner.complete(req).await
    }

    async fn stream(&self, req: crate::ai::providers::LlmRequest) -> anyhow::Result<crate::ai::providers::LlmStream> {
        self.inner.stream(req).await
    }
}
