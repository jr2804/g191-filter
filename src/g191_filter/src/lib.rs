// SPDX-License-Identifier: MIT
// Copyright 2026, Jan.Reimes

mod filter_id;
mod fir;
mod iir;
mod resample;
mod pyo3_bindings;

pub mod coeffs_generated;

pub use filter_id::{FilterId, FilterType};
pub use fir::FirFilter;
pub use iir::{CascadeIirFilter, DirectIirFilter, IirFilter};
pub use resample::Resampler;

use coeffs_generated::{fir as fir_coeffs, iir as iir_coeffs};

/// Configuration of a filter
#[derive(Debug, Clone)]
pub struct FilterConfig {
    pub filter_type: FilterType,
    pub filter_id: FilterId,
    /// Sampling rate of the filter design (Hz)
    pub sample_rate: f64,
    /// Number of filter coefficients / stages
    pub length: usize,
    /// Coefficients for FIR (h0 array) or IIR (b, a, gain)
    pub coefficients: Coefficients,
    /// Down/upsampling ratio (1 = no rate change)
    pub ratio_num: i64,
    pub ratio_den: i64,
    /// Gain factor
    pub gain: f64,
}

#[derive(Debug, Clone)]
pub enum Coefficients {
    Fir {
        /// FIR coefficients (normalized)
        h0: Vec<f64>,
    },
    IirParallel {
        /// Gain factor
        gain: f64,
        /// Direct path coefficient
        direct: f64,
        /// Numerator coefficients (b[3] per block)
        b: Vec<[f64; 3]>,
        /// Denominator coefficients (c[2] per block)
        c: Vec<[f64; 2]>,
    },
    IirCascade {
        /// Gain factor
        gain: f64,
        /// Numerator coefficients (a[2] per stage, STL convention: feedforward on x)
        b: Vec<[f64; 2]>,
        /// Denominator coefficients (b[2] per stage, STL convention: feedback on y)
        a: Vec<[f64; 2]>,
    },
    IirDirect {
        /// Gain factor
        gain: f64,
        /// Numerator coefficients (b)
        b: Vec<f64>,
        /// Denominator coefficients (a, a[0] should be 1.0)
        a: Vec<f64>,
    },
}

/// Look up a filter configuration by FilterId
pub fn get_filter_config(filter_id: FilterId) -> Option<FilterConfig> {
    let (filter_type, sample_rate, length, coefficients, ratio_num, ratio_den, gain) = match filter_id {
        FilterId::HQDown2To1 => (
            FilterType::Fir, 16000.0, fir_coeffs::HQ_DOWN_2_TO_1.len(),
            Coefficients::Fir { h0: fir_coeffs::HQ_DOWN_2_TO_1.to_vec() },
            1, 2, 1.0,
        ),
        FilterId::HQDown3To1 => (
            FilterType::Fir, 16000.0, fir_coeffs::HQ_DOWN_3_TO_1.len(),
            Coefficients::Fir { h0: fir_coeffs::HQ_DOWN_3_TO_1.to_vec() },
            1, 3, 1.0,
        ),
        FilterId::FlatBandPass => (
            FilterType::Fir, 8000.0, fir_coeffs::FLAT_BAND_PASS.len(),
            Coefficients::Fir { h0: fir_coeffs::FLAT_BAND_PASS.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::IRS8 => (
            FilterType::Fir, 8000.0, fir_coeffs::IRS8.len(),
            Coefficients::Fir { h0: fir_coeffs::IRS8.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::IRS16 => (
            FilterType::Fir, 16000.0, fir_coeffs::IRS16.len(),
            Coefficients::Fir { h0: fir_coeffs::IRS16.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::ModIRS16 => (
            FilterType::Fir, 16000.0, fir_coeffs::MOD_IRS16.len(),
            Coefficients::Fir { h0: fir_coeffs::MOD_IRS16.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::ModIRS48 => (
            FilterType::Fir, 8000.0, fir_coeffs::MOD_IRS48.len(),
            Coefficients::Fir { h0: fir_coeffs::MOD_IRS48.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::LP1p5_48k => (
            FilterType::Fir, 48000.0, fir_coeffs::LP1P5_48K.len(),
            Coefficients::Fir { h0: fir_coeffs::LP1P5_48K.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::LP35_48k => (
            FilterType::Fir, 48000.0, fir_coeffs::LP35_48K.len(),
            Coefficients::Fir { h0: fir_coeffs::LP35_48K.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::LP7_48k => (
            FilterType::Fir, 48000.0, fir_coeffs::LP7_48K.len(),
            Coefficients::Fir { h0: fir_coeffs::LP7_48K.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::LP10_48k => (
            FilterType::Fir, 48000.0, fir_coeffs::LP10_48K.len(),
            Coefficients::Fir { h0: fir_coeffs::LP10_48K.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::LP12_48k => (
            FilterType::Fir, 48000.0, fir_coeffs::LP12_48K.len(),
            Coefficients::Fir { h0: fir_coeffs::LP12_48K.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::LP14_48k => (
            FilterType::Fir, 48000.0, fir_coeffs::LP14_48K.len(),
            Coefficients::Fir { h0: fir_coeffs::LP14_48K.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::LP20_48k => (
            FilterType::Fir, 48000.0, fir_coeffs::LP20_48K.len(),
            Coefficients::Fir { h0: fir_coeffs::LP20_48K.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::G712_8k => (
            FilterType::Iir, 8000.0, 4,
            Coefficients::IirParallel {
                gain: 1.0,
                direct: iir_coeffs::G712_16K_K,
                b: iir_coeffs::G712_16K_B.to_vec(),
                c: iir_coeffs::G712_16K_C.to_vec(),
            },
            1, 1, 1.0,
        ),
        FilterId::DirDCRemoval => (
            FilterType::Iir, 48000.0, 2,
            Coefficients::IirDirect {
                gain: iir_coeffs::DIR_DC_GAIN,
                b: iir_coeffs::DIR_DC_A.to_vec(),
                a: iir_coeffs::DIR_DC_B.to_vec(),
            },
            1, 1, 1.0,
        ),
        FilterId::DirLP3To1 => (
            FilterType::Iir, 16000.0, 24,
            Coefficients::IirDirect {
                gain: iir_coeffs::DIR_LP_3TO1_GAIN,
                b: iir_coeffs::DIR_LP_3TO1_A.to_vec(),
                a: iir_coeffs::DIR_LP_3TO1_B.to_vec(),
            },
            1, 3, 1.0,
        ),
        FilterId::DirLP1To3 => (
            FilterType::Iir, 16000.0, 24,
            Coefficients::IirDirect {
                gain: iir_coeffs::DIR_LP_3TO1_GAIN,
                b: iir_coeffs::DIR_LP_3TO1_A.to_vec(),
                a: iir_coeffs::DIR_LP_3TO1_B.to_vec(),
            },
            3, 1, 1.0,
        ),
        FilterId::CascLP3To1 => (
            FilterType::Iir, 16000.0, 7,
            Coefficients::IirCascade {
                gain: iir_coeffs::CASC_LP_GAIN_3TO1,
                b: iir_coeffs::CASC_LP_A.to_vec(),
                a: iir_coeffs::CASC_LP_B.to_vec(),
            },
            1, 3, 1.0,
        ),
        FilterId::CascLP1To3 => (
            FilterType::Iir, 16000.0, 7,
            Coefficients::IirCascade {
                gain: iir_coeffs::CASC_LP_GAIN_1TO3,
                b: iir_coeffs::CASC_LP_A.to_vec(),
                a: iir_coeffs::CASC_LP_B.to_vec(),
            },
            3, 1, 1.0,
        ),
        FilterId::Unknown => return None,
    };

    Some(FilterConfig {
        filter_type,
        filter_id,
        sample_rate,
        length,
        coefficients,
        ratio_num,
        ratio_den,
        gain,
    })
}

/// Basic filter information
#[derive(Debug, Clone)]
pub struct FilterInfo {
    pub id: FilterId,
    pub filter_type: FilterType,
    pub sample_rate: f64,
    pub ratio_num: i64,
    pub ratio_den: i64,
    pub length: usize,
}

/// Get filter information
pub fn filter_info(filter_id: FilterId) -> Option<FilterInfo> {
    let config = get_filter_config(filter_id)?;
    Some(FilterInfo {
        id: config.filter_id,
        filter_type: config.filter_type,
        sample_rate: config.sample_rate,
        ratio_num: config.ratio_num,
        ratio_den: config.ratio_den,
        length: config.length,
    })
}

/// List all available filter IDs
pub fn list_filter_ids() -> Vec<FilterId> {
    use FilterId::*;
    vec![
        HQDown2To1, HQDown3To1, FlatBandPass,
        IRS8, IRS16, ModIRS16, ModIRS48,
        LP1p5_48k, LP35_48k, LP7_48k, LP10_48k, LP12_48k, LP14_48k, LP20_48k,
        G712_8k, DirDCRemoval, DirLP3To1, DirLP1To3, CascLP3To1, CascLP1To3,
    ]
}

/// Export coefficients in scipy-compatible (b, a) format
pub fn get_coefficients_ba(filter_id: FilterId) -> Option<(Vec<f64>, Vec<f64>)> {
    let config = get_filter_config(filter_id)?;
    match &config.coefficients {
        Coefficients::Fir { h0 } => Some((h0.clone(), vec![1.0])),
        Coefficients::IirDirect { b, a, .. } => Some((b.clone(), a.clone())),
        Coefficients::IirCascade { b, a, gain } => Some(cascade_to_ba(b, a, *gain)),
        Coefficients::IirParallel { b, c, gain, direct } => Some(parallel_to_ba(b, c, *gain, *direct)),
    }
}

/// Export coefficients in scipy SOS format
pub fn get_coefficients_sos(filter_id: FilterId) -> Option<Vec<[f64; 6]>> {
    let config = get_filter_config(filter_id)?;
    match &config.coefficients {
        Coefficients::Fir { h0 } => {
            // FIR: single section with a = [1]
            let mut b = [0.0; 3];
            for (i, &v) in h0.iter().take(3).enumerate() {
                b[i] = v;
            }
            Some(vec![[b[0], b[1], b[2], 1.0, 0.0, 0.0]])
        }
        Coefficients::IirDirect { b, a, .. } => {
            let mut b_pad = [0.0; 3];
            let mut a_pad = [0.0; 3];
            for (i, &v) in b.iter().take(3).enumerate() { b_pad[i] = v; }
            for (i, &v) in a.iter().take(3).enumerate() { a_pad[i] = v; }
            let a0 = a_pad[0];
            if a0 == 0.0 { return None; }
            Some(vec![[b_pad[0]/a0, b_pad[1]/a0, b_pad[2]/a0, 1.0, a_pad[1]/a0, a_pad[2]/a0]])
        }
        Coefficients::IirCascade { b, a, gain } => {
            let mut sos = Vec::new();
            let mut g = *gain;
            for (i, stage_num) in b.iter().enumerate() {
                let stage_den = a.get(i).copied().unwrap_or([0.0, 0.0]);
                // SOS: [b0, b1, b2, 1, a1, a2] — numerator from STL b, denominator from STL a
                let scale = if i == 0 { g } else { 1.0 };
                sos.push([scale, stage_num[0]*scale, stage_num[1]*scale,
                          1.0, stage_den[0], stage_den[1]]);
                g = 1.0;
            }
            Some(sos)
        }
        Coefficients::IirParallel { .. } => None,
    }
}

/// Convert cascade form IIR to single (b, a) numerator/denominator
/// STL convention: b = numerator (feedforward), a = denominator (feedback)
fn cascade_to_ba(b_num: &[[f64; 2]], a_den: &[[f64; 2]], gain: f64) -> (Vec<f64>, Vec<f64>) {
    // Each stage: H(z) = (1 + num0 z^-1 + num1 z^-2) / (1 + den0 z^-1 + den1 z^-2)
    // where num = b (feedforward), den = a (feedback), from the STL kernel:
    //   yj = xj + a[n][0]*T[n][0] + a[n][1]*T[n][1] - (b[n][0]*T[n][2] + b[n][1]*T[n][3])
    // (STL 'a' = feedforward/numerator, STL 'b' = feedback/denominator)
    let num_stages = b_num.len();
    let mut b_out: Vec<f64> = vec![gain];
    let mut a_out: Vec<f64> = vec![1.0];
    for i in 0..num_stages {
        // scipy convention: numerator = 1 + stl_a0 z^-1 + stl_a1 z^-2
        let stage_b = [1.0, b_num[i][0], b_num[i][1]];
        // scipy convention: denominator = 1 + stl_b0 z^-1 + stl_b1 z^-2
        let stage_a = [1.0, a_den[i][0], a_den[i][1]];
        let mut new_b = vec![0.0; b_out.len() + stage_b.len() - 1];
        for (j, &b_val) in b_out.iter().enumerate() {
            for (k, &sb) in stage_b.iter().enumerate() {
                new_b[j + k] += b_val * sb;
            }
        }
        b_out = new_b;
        let mut new_a = vec![0.0; a_out.len() + stage_a.len() - 1];
        for (j, &a_val) in a_out.iter().enumerate() {
            for (k, &sa) in stage_a.iter().enumerate() {
                new_a[j + k] += a_val * sa;
            }
        }
        a_out = new_a;
    }
    (b_out, a_out)
}

/// Convert parallel form IIR to single (b, a) numerator/denominator
fn parallel_to_ba(b: &[[f64; 3]], c: &[[f64; 2]], gain: f64, direct: f64) -> (Vec<f64>, Vec<f64>) {
    // H(z) = gain * (direct + sum_i (b_i0 + b_i1 z^-1 + b_i2 z^-2) / (1 + c_i0 z^-1 + c_i1 z^-2))
    let num_blocks = b.len();
    let mut a_out: Vec<f64> = vec![1.0];
    for i in 0..num_blocks {
        let bc = [1.0, c[i][0], c[i][1]];
        let mut new_a = vec![0.0; a_out.len() + bc.len() - 1];
        for (j, &a_val) in a_out.iter().enumerate() {
            for (k, &sa) in bc.iter().enumerate() {
                new_a[j + k] += a_val * sa;
            }
        }
        a_out = new_a;
    }

    let mut b_out: Vec<f64> = vec![0.0; a_out.len()];
    // Direct path: gain * direct * a_out
    for (k, &ak) in a_out.iter().enumerate() {
        b_out[k] += gain * direct * ak;
    }
    // Each block: gain * (b_i0 + b_i1 z^-1 + b_i2 z^-2) * (a_out / block_c_i)
    for i in 0..num_blocks {
        // a_out / block_c_i = product of all other blocks' denominators
        let mut other_a: Vec<f64> = vec![1.0];
        for j in 0..num_blocks {
            if j == i { continue; }
            let jc = [1.0, c[j][0], c[j][1]];
            let mut new_a = vec![0.0; other_a.len() + jc.len() - 1];
            for (m, &av) in other_a.iter().enumerate() {
                for (n, &sv) in jc.iter().enumerate() {
                    new_a[m + n] += av * sv;
                }
            }
            other_a = new_a;
        }
        // Numerator contribution: gain * (b_i0 + b_i1 z^-1 + b_i2 z^-2) * other_a
        for (m, &bm) in b[i].iter().enumerate() {
            for (n, &av) in other_a.iter().enumerate() {
                b_out[m + n] += gain * bm * av;
            }
        }
    }
    (b_out, a_out)
}
