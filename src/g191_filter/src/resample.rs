// SPDX-License-Identifier: MIT
// Copyright 2026, Jan.Reimes

//! Sample-rate converter using windowed-sinc interpolation.
//!
//! Single `resample()` entry point handles integer/non-integer up- and
//! downsampling. The interpolation kernel is a Hamming-windowed sinc with
//! cutoff at `min(src_Nyquist, dst_Nyquist)`, so downsampling is
//! anti-aliased and upsampling suppresses spectral images. Every output
//! sample is normalized by the effective kernel sum, keeping unit DC gain
//! at interior and edge positions alike.

const TAPS: i64 = 32; // kernel half-width in source samples

/// Windowed-sinc resampler
pub struct Resampler {
    ratio: f64,
}

impl Resampler {
    /// Create a new resampler (src_rate -> dst_rate)
    pub fn new(src_rate: f64, dst_rate: f64) -> Self {
        assert!(src_rate > 0.0 && dst_rate > 0.0, "Sample rates must be positive");
        Self { ratio: dst_rate / src_rate }
    }

    /// Get ratio (dst_rate / src_rate)
    pub fn ratio(&self) -> f64 {
        self.ratio
    }

    /// Resample `input` from source to destination rate.
    ///
    /// Output length is `round(len * ratio)`.
    pub fn resample(&self, input: &[f64]) -> Vec<f64> {
        if (self.ratio - 1.0).abs() < 1e-9 || input.is_empty() {
            return input.to_vec();
        }
        let len_in = input.len() as i64;
        let len_out = (input.len() as f64 * self.ratio).round() as usize;
        // Normalized cutoff: min of both Nyquist frequencies (in src units)
        let cutoff = 0.5 * self.ratio.min(1.0);
        let two_c = 2.0 * cutoff;

        let mut out = Vec::with_capacity(len_out);
        for o in 0..len_out {
            // Source position for output sample o
            let t = o as f64 / self.ratio;
            let i0 = t.floor() as i64;
            let frac = t - i0 as f64;

            let mut acc = 0.0;
            let mut wsum = 0.0;
            for k in -TAPS..=TAPS {
                let idx = i0 + k;
                if idx < 0 || idx >= len_in {
                    continue;
                }
                // Kernel: two_c * sinc(two_c * (k - frac)), Hamming window
                let d = k as f64 - frac;
                let x = std::f64::consts::PI * two_c * d;
                let sinc = if x.abs() < 1e-12 { two_c } else { x.sin() / x };
                let warg = std::f64::consts::PI * d / (TAPS as f64 + 1.0);
                let w = 0.54 + 0.46 * warg.cos();
                let wgt = sinc * w;
                acc += input[idx as usize] * wgt;
                wsum += wgt;
            }
            // Normalize by the effective kernel sum for unit DC gain
            if wsum != 0.0 {
                acc /= wsum;
            }
            out.push(acc);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-3;

    fn sine(freq: f64, rate: f64, n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * freq * i as f64 / rate).sin())
            .collect()
    }

    #[test]
    fn identity_returns_input() {
        let r = Resampler::new(48_000.0, 48_000.0);
        let x = sine(1000.0, 48_000.0, 100);
        assert_eq!(r.resample(&x), x);
    }

    #[test]
    fn length_scales_with_ratio() {
        let x = vec![0.5; 1000];
        for (src, dst, expected) in [
            (48_000.0, 96_000.0, 2000),
            (96_000.0, 48_000.0, 500),
            (44_100.0, 48_000.0, 1088), // round(1000 * 48000/44100)
        ] {
            let y = Resampler::new(src, dst).resample(&x);
            assert_eq!(y.len(), expected, "src={src} dst={dst}");
        }
    }

    #[test]
    fn upsample_dc_signal_keeps_amplitude() {
        let x = vec![0.7; 1000];
        let y = Resampler::new(48_000.0, 96_000.0).resample(&x);
        // Skip edges (kernel ramp-in); interior must hold the DC level
        let interior = &y[100..y.len() - 100];
        for &v in interior {
            assert!((v - 0.7).abs() < EPS, "DC drifted: {v}");
        }
    }

    #[test]
    fn downsample_dc_signal_keeps_amplitude() {
        let x = vec![0.7; 1000];
        let y = Resampler::new(96_000.0, 48_000.0).resample(&x);
        assert_eq!(y.len(), 500, "output length");
        let interior = &y[100..y.len() - 100];
        for &v in interior {
            assert!((v - 0.7).abs() < EPS, "DC drifted: {v}");
        }
    }

    #[test]
    fn downsample_suppresses_above_dst_nyquist() {
        // 96 kHz -> 48 kHz cuts at the dst Nyquist (24 kHz).
        // A 24.5 kHz tone (just above cutoff in src units) must be
        // attenuated; a 20 kHz tone (passband) must pass.
        let r = Resampler::new(96_000.0, 48_000.0);
        let stopband = sine(24_500.0, 96_000.0, 8000);
        let y_stop = r.resample(&stopband);
        let rms_in = (stopband.iter().map(|v| v * v).sum::<f64>() / stopband.len() as f64).sqrt();
        let rms_out = (y_stop.iter().map(|v| v * v).sum::<f64>() / y_stop.len() as f64).sqrt();
        assert!(rms_out < 0.15 * rms_in, "anti-alias: rms_in={rms_in} rms_out={rms_out}");

        let passband = sine(20_000.0, 96_000.0, 8000);
        let y_pass = r.resample(&passband);
        let rms_in_p = (passband.iter().map(|v| v * v).sum::<f64>() / passband.len() as f64).sqrt();
        let rms_out_p = (y_pass.iter().map(|v| v * v).sum::<f64>() / y_pass.len() as f64).sqrt();
        assert!(rms_out_p > 0.95 * rms_in_p, "passband loss: in={rms_in_p} out={rms_out_p}");
    }

    #[test]
    fn upsample_preserves_passband_sine() {
        // 1 kHz sine at 48 kHz -> 96 kHz: the original samples must reappear
        // at even output indices (t = k maps exactly).
        let x = sine(1000.0, 48_000.0, 500);
        let y = Resampler::new(48_000.0, 96_000.0).resample(&x);
        for (k, &xv) in x.iter().enumerate().take(400).skip(100) {
            let yv = y[2 * k];
            assert!((xv - yv).abs() < 1e-2, "k={k}: {xv} vs {yv}");
        }
    }

    #[test]
    fn noninteger_preserves_dc() {
        let x = vec![0.5; 2000];
        let y = Resampler::new(44_100.0, 48_000.0).resample(&x);
        let interior = &y[100..y.len() - 100];
        for &v in interior {
            assert!((v - 0.5).abs() < EPS, "DC drifted: {v}");
        }
    }

    #[test]
    fn empty_input_is_noop() {
        let r = Resampler::new(48_000.0, 96_000.0);
        assert!(r.resample(&[]).is_empty());
    }

    #[test]
    #[should_panic]
    fn nonpositive_rate_panics() {
        let _ = Resampler::new(0.0, 48_000.0);
    }
}
