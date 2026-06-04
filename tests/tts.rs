use notclicky::voice::tts::TtsProvider;
use std::time::Duration;

#[tokio::test]
async fn edge_tts_synthesize_full_voice() {
    let provider = notclicky::voice::tts_providers::edge::EdgeTtsProvider::new(
        "Microsoft Server Speech Text to Speech Voice (en-US, AriaNeural)".to_string()
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
