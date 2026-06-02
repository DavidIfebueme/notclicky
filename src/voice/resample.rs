pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    let ratio = to_rate as f64 / from_rate as f64;
    let new_len = (samples.len() as f64 * ratio) as usize;
    let mut out = Vec::with_capacity(new_len);
    for i in 0..new_len {
        let src_pos = i as f64 / ratio;
        let idx = src_pos as usize;
        let frac = src_pos - idx as f64;
        if idx + 1 < samples.len() {
            out.push(samples[idx] * (1.0 - frac as f32) + samples[idx + 1] * frac as f32);
        } else {
            out.push(samples[samples.len().saturating_sub(1)]);
        }
    }
    out
}
