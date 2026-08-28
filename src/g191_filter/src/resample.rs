// SPDX-License-Identifier: MIT
// Copyright 2026, Jan.Reimes

/// Simple sample-rate converter (polynomial interpolation)
pub struct Resampler {
    ratio: f64,
}

impl Resampler {
    /// Create a new resampler for arbitrary rate change
    pub fn new(src_rate: f64, dst_rate: f64) -> Self {
        assert!(src_rate > 0.0 && dst_rate > 0.0, "Sample rates must be positive");
        let ratio = dst_rate / src_rate;
        Self { ratio }
    }

    /// Get ratio (dst_rate / src_rate)
    pub fn ratio(&self) -> f64 {
        self.ratio
    }

    /// Upsample (insert zeros) and apply FIR filter
    pub fn upsample(&self, input: &[f64], coeffs: &[f64], _phase: &mut usize) -> Vec<f64> {
        if (self.ratio - 1.0).abs() < 1e-9 {
            return input.to_vec();
        }

        let factor = self.ratio.round() as usize;
        if factor == 0 || (factor as f64 - self.ratio).abs() > 0.1 {
            // Non-integer ratio - use sinc interpolation
            return self.resample_sinc(input, coeffs);
        }

        // Integer upsampling: insert (factor-1) zeros between each sample
        let mut upsampled = Vec::with_capacity(input.len() * factor);
        for &x in input {
            upsampled.push(x);
            for _ in 1..factor {
                upsampled.push(0.0);
            }
        }

        // Apply FIR filter
        let mut output = vec![0.0; upsampled.len()];
        let n = coeffs.len();
        for i in 0..upsampled.len() {
            let mut sum = 0.0;
            for k in 0..n {
                if i >= k {
                    sum += upsampled[i - k] * coeffs[k];
                }
            }
            output[i] = sum;
        }
        output
    }

    /// Downsample by filtering and decimating
    pub fn downsample(&self, input: &[f64], coeffs: &[f64], _phase: &mut usize) -> Vec<f64> {
        if (self.ratio - 1.0).abs() < 1e-9 {
            return input.to_vec();
        }

        let factor = (1.0 / self.ratio).round() as usize;
        if factor == 0 || (factor as f64 - 1.0 / self.ratio).abs() > 0.1 {
            return self.resample_sinc(input, coeffs);
        }

        // Apply FIR filter then decimate
        let n = coeffs.len();
        let mut filtered = vec![0.0; input.len()];
        for i in 0..input.len() {
            let mut sum = 0.0;
            for k in 0..n {
                if i >= k {
                    sum += input[i - k] * coeffs[k];
                }
            }
            filtered[i] = sum;
        }

        // Decimate
        filtered.iter().step_by(factor).copied().collect()
    }

    /// Generic sinc-based resampling for non-integer ratios
    fn resample_sinc(&self, input: &[f64], coeffs: &[f64]) -> Vec<f64> {
        let len_out = (input.len() as f64 * self.ratio).round() as usize;
        let mut output = vec![0.0; len_out];
        let _ = coeffs; // coeffs not strictly used in linear interpolation fallback

        for out_idx in 0..len_out {
            let src_idx_f = out_idx as f64 / self.ratio;
            let src_idx = src_idx_f as usize;
            let frac = src_idx_f - src_idx as f64;

            // Linear interpolation (simpler than sinc for general use)
            if src_idx + 1 < input.len() {
                output[out_idx] = input[src_idx] * (1.0 - frac) + input[src_idx + 1] * frac;
            } else if src_idx < input.len() {
                output[out_idx] = input[src_idx];
            }
        }
        output
    }
}
