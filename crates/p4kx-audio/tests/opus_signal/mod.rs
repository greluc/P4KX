//! Comparing decoded Opus sounds with the known signal they were encoded from.

/// Returns the signal-to-noise ratio in dB of channel `channel` of `decoded` against `signal`.
pub(crate) fn snr(signal: &[i16], decoded: &[i16], channels: usize, channel: usize) -> f64 {
    let (mut power, mut noise) = (0.0_f64, 0.0_f64);
    for (expected, got) in signal
        .iter()
        .skip(channel)
        .step_by(channels)
        .zip(decoded.iter().skip(channel).step_by(channels))
    {
        power += f64::from(*expected).powi(2);
        noise += (f64::from(*expected) - f64::from(*got)).powi(2);
    }
    10.0 * (power / noise.max(1.0)).log10()
}

/// Returns the interleaved samples of a WAV the crate wrote for `channels` channels.
pub(crate) fn wav_samples(wav: &[u8], channels: u16) -> Vec<i16> {
    let header_len = if channels > 2 { 68 } else { 44 };
    wav.get(header_len..)
        .unwrap_or_default()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i16::from_le_bytes(*pair))
        .collect()
}
