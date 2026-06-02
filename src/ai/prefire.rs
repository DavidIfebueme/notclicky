use crate::ai::providers::{LlmMessage, LlmProvider, LlmRequest};

pub struct PrefireResult {
    pub stream: crate::ai::providers::LlmStream,
    pub is_speculative: bool,
}

pub async fn try_prefire(
    provider: &dyn LlmProvider,
    interim_text: &str,
    system_prompt: &str,
    final_text: &str,
) -> Option<PrefireResult> {
    if interim_text.is_empty() {
        return None;
    }

    let divergence = compute_divergence(interim_text, final_text);
    if divergence > 0.15 {
        return None;
    }

    let req = build_request(interim_text, system_prompt);
    let stream = provider.stream(req).await.ok()?;

    Some(PrefireResult {
        stream,
        is_speculative: true,
    })
}

pub fn compute_divergence(interim: &str, final_text: &str) -> f32 {
    if interim.is_empty() {
        return 1.0;
    }

    let interim_words: Vec<&str> = interim.split_whitespace().collect();
    let final_words: Vec<&str> = final_text.split_whitespace().collect();

    let min_len = interim_words.len().min(final_words.len());
    if min_len == 0 {
        return 1.0;
    }

    let matches = interim_words.iter().zip(final_words.iter())
        .filter(|(a, b)| a.eq_ignore_ascii_case(b))
        .count();

    1.0 - (matches as f32 / min_len as f32)
}

fn build_request(text: &str, system_prompt: &str) -> LlmRequest {
    LlmRequest {
        messages: vec![
            LlmMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            LlmMessage {
                role: "user".to_string(),
                content: text.to_string(),
            },
        ],
        model: None,
        max_tokens: None,
        temperature: None,
    }
}
