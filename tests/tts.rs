use notclicky::voice::tts::TtsProvider;
use std::time::Duration;

#[tokio::test]
async fn edge_tts_synthesize_full_voice() {
    let provider = notclicky::voice::tts_providers::edge::EdgeTtsProvider::new(
        "Microsoft Server Speech Text to Speech Voice (en-US, AriaNeural)".to_string(),
    );

    for attempt in 1..=3 {
        let result = provider.synthesize("Hello world").await;
        match result {
            Ok(audio) => {
                assert!(!audio.is_empty(), "Edge TTS returned empty audio");
                eprintln!("Full voice name OK: {} bytes", audio.len());
                return;
            }
            Err(e) => {
                eprintln!("Attempt {} failed: {}", attempt, e);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
    panic!("Edge TTS failed after 3 attempts");
}

#[tokio::test]
async fn deepgram_tts_synthesize() {
    let key = std::env::var("DEEPGRAM_API_KEY").unwrap_or_default();
    if key.is_empty() {
        eprintln!("Skipping Deepgram TTS test — no API key");
        return;
    }
    let provider = notclicky::voice::tts_providers::deepgram::DeepgramTtsProvider::with_model(
        key,
        "aura-2-arcas-en".to_string(),
    );
    let audio = provider
        .synthesize("Hello world")
        .await
        .expect("Deepgram TTS failed");
    assert!(!audio.is_empty(), "Deepgram TTS returned empty audio");
    eprintln!("Deepgram Aura OK: {} bytes", audio.len());
}
