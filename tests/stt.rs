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
