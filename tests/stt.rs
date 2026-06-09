#[test]
fn compute_rms_silence() {
    let samples = vec![0.0f32; 1024];
    let rms = notclicky::voice::capture::compute_rms(&samples);
    assert_eq!(rms, 0.0);
}

#[test]
fn compute_rms_full_scale() {
    let samples = vec![1.0f32; 4];
    let rms = notclicky::voice::capture::compute_rms(&samples);
    assert!((rms - 1.0).abs() < 0.001);
}

#[test]
fn compute_rms_mixed() {
    let samples = vec![0.5, -0.5, 0.5, -0.5];
    let rms = notclicky::voice::capture::compute_rms(&samples);
    assert!((rms - 0.5).abs() < 0.001);
}

#[test]
fn resample_identity() {
    let samples = vec![1.0f32, 2.0, 3.0, 4.0];
    let result = notclicky::voice::resample::resample(&samples, 16000, 16000);
    assert_eq!(result, samples);
}

#[test]
fn resample_downsample() {
    let samples = vec![1.0f32, 2.0, 3.0, 4.0];
    let result = notclicky::voice::resample::resample(&samples, 48000, 16000);
    assert_eq!(result.len(), 1);
}

#[test]
fn resample_upsample() {
    let samples = vec![1.0f32, 2.0];
    let result = notclicky::voice::resample::resample(&samples, 16000, 48000);
    assert_eq!(result.len(), 6);
    assert!((result[0] - 1.0).abs() < 0.01);
}

#[test]
fn screenshot_heuristic_visual_phrases() {
    assert!(notclicky::voice::assistant::should_attach_screenshot(
        "what's on my screen"
    ));
    assert!(notclicky::voice::assistant::should_attach_screenshot(
        "look at this window"
    ));
    assert!(notclicky::voice::assistant::should_attach_screenshot(
        "click that button"
    ));
    assert!(notclicky::voice::assistant::should_attach_screenshot(
        "where is the icon"
    ));
    assert!(notclicky::voice::assistant::should_attach_screenshot(
        "highlight this menu"
    ));
}

#[test]
fn screenshot_heuristic_visual_tokens() {
    assert!(notclicky::voice::assistant::should_attach_screenshot(
        "describe the dialog"
    ));
    assert!(notclicky::voice::assistant::should_attach_screenshot(
        "what does this tooltip say"
    ));
    assert!(notclicky::voice::assistant::should_attach_screenshot(
        "check the sidebar"
    ));
}

#[test]
fn screenshot_heuristic_non_visual() {
    assert!(!notclicky::voice::assistant::should_attach_screenshot(
        "what's the weather today"
    ));
    assert!(!notclicky::voice::assistant::should_attach_screenshot(
        "tell me a joke"
    ));
    assert!(!notclicky::voice::assistant::should_attach_screenshot(
        "how do I install rust"
    ));
    assert!(!notclicky::voice::assistant::should_attach_screenshot(
        "what time is it"
    ));
}
