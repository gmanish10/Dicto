use rubato::{FftFixedIn, Resampler};

const TARGET_RATE: u32 = 16_000;

/// Convert native-rate interleaved samples to 16kHz mono f32 — the format whisper wants.
pub fn to_16k_mono(
    interleaved: &[f32],
    sample_rate: u32,
    channels: u16,
) -> anyhow::Result<Vec<f32>> {
    let mono = if channels > 1 {
        super::Recorder::mono_mix(interleaved, channels)
    } else {
        interleaved.to_vec()
    };

    if sample_rate == TARGET_RATE {
        return Ok(mono);
    }
    if mono.is_empty() {
        return Ok(Vec::new());
    }

    // Process in chunks of 1024 input samples for efficiency.
    const CHUNK: usize = 1024;
    let mut resampler = FftFixedIn::<f32>::new(
        sample_rate as usize,
        TARGET_RATE as usize,
        CHUNK,
        2, // sub-chunks
        1, // channels
    )?;

    let mut out = Vec::with_capacity(
        (mono.len() as u64 * TARGET_RATE as u64 / sample_rate as u64) as usize + CHUNK,
    );
    let mut cursor = 0;
    while cursor + CHUNK <= mono.len() {
        let input = vec![mono[cursor..cursor + CHUNK].to_vec()];
        let resampled = resampler.process(&input, None)?;
        out.extend_from_slice(&resampled[0]);
        cursor += CHUNK;
    }
    // Tail: pad with zeros to the next chunk boundary, then trim the proportional tail.
    let remaining = mono.len() - cursor;
    if remaining > 0 {
        let mut tail = mono[cursor..].to_vec();
        tail.resize(CHUNK, 0.0);
        let resampled = resampler.process(&[tail], None)?;
        let valid_out = remaining as u64 * TARGET_RATE as u64 / sample_rate as u64;
        out.extend(resampled[0].iter().take(valid_out as usize));
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_when_already_16k() {
        let samples = vec![0.1, -0.2, 0.3, 0.4];
        let out = to_16k_mono(&samples, 16_000, 1).unwrap();
        assert_eq!(out, samples);
    }

    #[test]
    fn handles_stereo_mono_mix() {
        let stereo = vec![0.5, -0.5, 0.5, -0.5]; // L=0.5, R=-0.5 → mono=0.0
        let out = to_16k_mono(&stereo, 16_000, 2).unwrap();
        assert!(out.iter().all(|&s| s.abs() < 1e-6));
    }
}
