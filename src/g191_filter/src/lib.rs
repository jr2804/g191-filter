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

/// Internal state of a filter, used for blockwise streaming processing.
///
/// Serializes to a flat `Vec<f64>` so it can cross the Rust↔Python boundary
/// cheaply via the `pyo3` array protocol.
#[derive(Debug, Clone)]
pub enum FilterState {
    Fir { t: Vec<f64>, k0: i64 },
    IirParallel { t: Vec<[f64; 2]>, k0: i64 },
    IirCascade { t: Vec<[f64; 4]>, k0: i64 },
    IirDirect { t: Vec<[f64; 2]>, k0: i64 },
}

impl FilterState {
    /// Flat representation for Python interop
    pub fn to_vec(&self) -> Vec<f64> {
        match self {
            Self::Fir { t, k0 } => {
                let mut v = t.clone();
                v.push(*k0 as f64);
                v
            }
            Self::IirParallel { t, k0 } => {
                let mut v = Vec::with_capacity(t.len() * 2 + 1);
                for row in t {
                    v.push(row[0]);
                    v.push(row[1]);
                }
                v.push(*k0 as f64);
                v
            }
            Self::IirCascade { t, k0 } => {
                let mut v = Vec::with_capacity(t.len() * 4 + 1);
                for row in t {
                    v.extend_from_slice(row);
                }
                v.push(*k0 as f64);
                v
            }
            Self::IirDirect { t, k0 } => {
                let mut v = Vec::with_capacity(t.len() * 2 + 1);
                for row in t {
                    v.push(row[0]);
                    v.push(row[1]);
                }
                v.push(*k0 as f64);
                v
            }
        }
    }

    /// Reconstruct from a flat representation
    pub fn from_vec(state: &[f64], filter_kind: &str, block_count: usize) -> Option<Self> {
        match filter_kind {
            "fir" => {
                if state.len() != block_count + 1 {
                    return None;
                }
                let t = state[..block_count].to_vec();
                let k0 = state[block_count] as i64;
                Some(Self::Fir { t, k0 })
            }
            "iir_parallel" => {
                let expected = block_count * 2 + 1;
                if state.len() != expected {
                    return None;
                }
                let mut t = vec![[0.0; 2]; block_count];
                for n in 0..block_count {
                    t[n][0] = state[n * 2];
                    t[n][1] = state[n * 2 + 1];
                }
                let k0 = state[block_count * 2] as i64;
                Some(Self::IirParallel { t, k0 })
            }
            "iir_cascade" => {
                let expected = block_count * 4 + 1;
                if state.len() != expected {
                    return None;
                }
                let mut t = vec![[0.0; 4]; block_count];
                for n in 0..block_count {
                    for k in 0..4 {
                        t[n][k] = state[n * 4 + k];
                    }
                }
                let k0 = state[block_count * 4] as i64;
                Some(Self::IirCascade { t, k0 })
            }
            "iir_direct" => {
                let expected = block_count * 2 + 1;
                if state.len() != expected {
                    return None;
                }
                let mut t = vec![[0.0; 2]; block_count];
                for n in 0..block_count {
                    t[n][0] = state[n * 2];
                    t[n][1] = state[n * 2 + 1];
                }
                let k0 = state[block_count * 2] as i64;
                Some(Self::IirDirect { t, k0 })
            }
            _ => None,
        }
    }
}

/// Blockwise filter wrapper that retains its state across calls.
///
/// One-shot filtering is implemented on top of this; the one-shot path simply
/// creates a `BlockwiseFilter`, processes the entire input in chunks of
/// `block_size`, and concatenates the output.
pub struct BlockwiseFilter {
    /// Concrete filter instance (owned)
    inner: FilterInner,
    /// Cached filter id for re-creation
    filter_id: FilterId,
    /// Block size used for chunked processing
    block_size: usize,
}

enum FilterInner {
    Fir(FirFilter),
    IirParallel(IirFilter),
    IirCascade(CascadeIirFilter),
    IirDirect(DirectIirFilter),
}

impl BlockwiseFilter {
    /// Create a fresh blockwise filter for the given filter id and block size
    pub fn new(filter_id: FilterId, block_size: usize) -> Option<Self> {
        let config = get_filter_config(filter_id)?;
        let inner = match &config.coefficients {
            Coefficients::Fir { h0 } => {
                let hswitch = if config.ratio_num > config.ratio_den { 'U' } else { 'D' };
                let dwn_up = if hswitch == 'U' { config.ratio_num } else { config.ratio_den };
                let f = FirFilter::new(h0, config.gain, dwn_up, hswitch);
                FilterInner::Fir(f)
            }
            Coefficients::IirParallel { gain, direct, b, c } => {
                let f = IirFilter::new(*gain, *direct, b, c, config.ratio_num.max(config.ratio_den), config.ratio_num > config.ratio_den);
                FilterInner::IirParallel(f)
            }
            Coefficients::IirCascade { gain, b, a } => {
                let f = CascadeIirFilter::new(*gain, b, a, config.ratio_num.max(config.ratio_den), config.ratio_num > config.ratio_den);
                FilterInner::IirCascade(f)
            }
            Coefficients::IirDirect { gain, b, a } => {
                let f = DirectIirFilter::new(*gain, b, a, config.ratio_num.max(config.ratio_den), config.ratio_num > config.ratio_den);
                FilterInner::IirDirect(f)
            }
        };
        Some(Self {
            inner,
            filter_id,
            block_size: block_size.max(1),
        })
    }

    /// Process a single chunk of input, appending to `output`.
    /// This is the streaming entry point: callers accumulate `output` across
    /// chunks and pass the state in via `set_state` between calls.
    pub fn process_chunk(&mut self, x: &[f64], output: &mut Vec<f64>) {
        let bs = self.block_size;
        if x.len() <= bs {
            let y = self.process_block(x);
            output.extend_from_slice(&y);
        } else {
            for chunk in x.chunks(bs) {
                let y = self.process_block(chunk);
                output.extend_from_slice(&y);
            }
        }
    }

    /// Process the entire input at once (uses chunked processing internally)
    pub fn process_all(&mut self, x: &[f64]) -> Vec<f64> {
        let mut out = Vec::new();
        self.process_chunk(x, &mut out);
        out
    }

    fn process_block(&mut self, x: &[f64]) -> Vec<f64> {
        match &mut self.inner {
            FilterInner::Fir(f) => f.process_block(x),
            FilterInner::IirParallel(f) => f.process_block(x),
            FilterInner::IirCascade(f) => f.process_block(x),
            FilterInner::IirDirect(f) => f.process_block(x),
        }
    }

    /// Snapshot the current state
    pub fn get_state(&self) -> FilterState {
        match &self.inner {
            FilterInner::Fir(f) => {
                let v = f.get_state();
                let k0 = *v.last().unwrap_or(&0.0) as i64;
                let mut t = v;
                t.pop();
                FilterState::Fir { t, k0 }
            }
            FilterInner::IirParallel(f) => {
                let v = f.get_state();
                let nblocks = (v.len() - 1) / 2;
                let mut t = vec![[0.0; 2]; nblocks];
                for n in 0..nblocks {
                    t[n][0] = v[n * 2];
                    t[n][1] = v[n * 2 + 1];
                }
                let k0 = v[v.len() - 1] as i64;
                FilterState::IirParallel { t, k0 }
            }
            FilterInner::IirCascade(f) => {
                let v = f.get_state();
                let nblocks = (v.len() - 1) / 4;
                let mut t = vec![[0.0; 4]; nblocks];
                for n in 0..nblocks {
                    for k in 0..4 {
                        t[n][k] = v[n * 4 + k];
                    }
                }
                let k0 = v[v.len() - 1] as i64;
                FilterState::IirCascade { t, k0 }
            }
            FilterInner::IirDirect(f) => {
                let v = f.get_state();
                let nblocks = (v.len() - 1) / 2;
                let mut t = vec![[0.0; 2]; nblocks];
                for n in 0..nblocks {
                    t[n][0] = v[n * 2];
                    t[n][1] = v[n * 2 + 1];
                }
                let k0 = v[v.len() - 1] as i64;
                FilterState::IirDirect { t, k0 }
            }
        }
    }

    /// Restore state from a snapshot
    pub fn set_state(&mut self, state: FilterState) {
        let flat = state.to_vec();
        match &mut self.inner {
            FilterInner::Fir(f) => f.set_state(&flat),
            FilterInner::IirParallel(f) => f.set_state(&flat),
            FilterInner::IirCascade(f) => f.set_state(&flat),
            FilterInner::IirDirect(f) => f.set_state(&flat),
        }
    }

    /// Reset the filter to zero state
    pub fn reset(&mut self) {
        match &mut self.inner {
            FilterInner::Fir(f) => f.reset(),
            FilterInner::IirParallel(f) => f.reset(),
            FilterInner::IirCascade(f) => f.reset(),
            FilterInner::IirDirect(f) => f.reset(),
        }
    }

    /// Identifier of the underlying filter
    pub fn filter_id(&self) -> FilterId {
        self.filter_id
    }

    /// Block size used for chunked processing
    pub fn block_size(&self) -> usize {
        self.block_size
    }
}

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
            1, 1, -1.0,  /* STL: mod_irs16/48 apply polarity inversion */
        ),
        FilterId::ModIRS48 => (
            FilterType::Fir, 48000.0, fir_coeffs::MOD_IRS48.len(),
            Coefficients::Fir { h0: fir_coeffs::MOD_IRS48.to_vec() },
            1, 1, -1.0,  /* STL: mod_irs16/48 apply polarity inversion */
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
        // --- HQ upsamplers: reuse down-sampler coeff set, gain = up-factor, 'U' kernel ---
        FilterId::HQUp1To2 => (
            FilterType::Fir, 8000.0, fir_coeffs::HQ_DOWN_2_TO_1.len(),
            Coefficients::Fir { h0: fir_coeffs::HQ_DOWN_2_TO_1.to_vec() },
            2, 1, 2.0,
        ),
        FilterId::HQUp1To3 => (
            FilterType::Fir, 8000.0, fir_coeffs::HQ_DOWN_3_TO_1.len(),
            Coefficients::Fir { h0: fir_coeffs::HQ_DOWN_3_TO_1.to_vec() },
            3, 1, 3.0,
        ),
        // --- Flat band-pass family (reuse flat_band_pass coefficients) ---
        FilterId::FlatBandPass1 => (
            FilterType::Fir, 8000.0, fir_coeffs::FLAT_BAND_PASS.len(),
            Coefficients::Fir { h0: fir_coeffs::FLAT_BAND_PASS.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::FlatBandPass1To2 => (
            FilterType::Fir, 8000.0, fir_coeffs::FLAT_BAND_PASS.len(),
            Coefficients::Fir { h0: fir_coeffs::FLAT_BAND_PASS.to_vec() },
            2, 1, 2.0,
        ),
        // --- Psophometric / measurement filters ---
        FilterId::Msin16k => (
            FilterType::Fir, 16000.0, fir_coeffs::MSIN_16K.len(),
            Coefficients::Fir { h0: fir_coeffs::MSIN_16K.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::Pso8k => (
            FilterType::Fir, 8000.0, fir_coeffs::PSO_8K.len(),
            Coefficients::Fir { h0: fir_coeffs::PSO_8K.to_vec() },
            1, 1, 1.0,
        ),
        // --- IRS variants ---
        FilterId::Hirs16 => (
            FilterType::Fir, 16000.0, fir_coeffs::HIRS16.len(),
            Coefficients::Fir { h0: fir_coeffs::HIRS16.to_vec() },
            1, 1, 1.08,
        ),
        FilterId::TiaIrs8 => (
            FilterType::Fir, 8000.0, fir_coeffs::TIA_IRS8.len(),
            Coefficients::Fir { h0: fir_coeffs::TIA_IRS8.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::RxIrs8 => (
            FilterType::Fir, 8000.0, fir_coeffs::RX_MOD_IRS8.len(),
            Coefficients::Fir { h0: fir_coeffs::RX_MOD_IRS8.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::RxIrs16 => (
            FilterType::Fir, 16000.0, fir_coeffs::RX_MOD_IRS16.len(),
            Coefficients::Fir { h0: fir_coeffs::RX_MOD_IRS16.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::Dsm16k => (
            FilterType::Fir, 16000.0, fir_coeffs::DSM_16K.len(),
            Coefficients::Fir { h0: fir_coeffs::DSM_16K.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::P341_16k => (
            FilterType::Fir, 16000.0, fir_coeffs::P341_16K.len(),
            Coefficients::Fir { h0: fir_coeffs::P341_16K.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::Bp5k16k => (
            FilterType::Fir, 16000.0, fir_coeffs::BP5K_16K.len(),
            Coefficients::Fir { h0: fir_coeffs::BP5K_16K.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::Bp100_5k16k => (
            FilterType::Fir, 16000.0, fir_coeffs::BP100_5K_16K.len(),
            Coefficients::Fir { h0: fir_coeffs::BP100_5K_16K.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::Bp14k32k => (
            FilterType::Fir, 32000.0, fir_coeffs::BP14K_32K.len(),
            Coefficients::Fir { h0: fir_coeffs::BP14K_32K.to_vec() },
            1, 1, 1.0,
        ),
        FilterId::Bp20k48k => (
            FilterType::Fir, 48000.0, fir_coeffs::BP20K_48K.len(),
            Coefficients::Fir { h0: fir_coeffs::BP20K_48K.to_vec() },
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
        // --- Standard PCM filters (same G.712 parallel IIR, different rate factors) ---
        FilterId::Pcm16k => (
            FilterType::Iir, 16000.0, 4,
            Coefficients::IirParallel {
                gain: 1.0,
                direct: iir_coeffs::G712_16K_K,
                b: iir_coeffs::G712_16K_B.to_vec(),
                c: iir_coeffs::G712_16K_C.to_vec(),
            },
            1, 1, 1.0,
        ),
        FilterId::Pcm2To1 => (
            FilterType::Iir, 16000.0, 4,
            Coefficients::IirParallel {
                gain: 1.0,
                direct: iir_coeffs::G712_16K_K,
                b: iir_coeffs::G712_16K_B.to_vec(),
                c: iir_coeffs::G712_16K_C.to_vec(),
            },
            1, 2, 1.0,
        ),
        FilterId::Pcm1To2 => (
            FilterType::Iir, 8000.0, 4,
            Coefficients::IirParallel {
                gain: 1.0,
                direct: iir_coeffs::G712_16K_K,
                b: iir_coeffs::G712_16K_B.to_vec(),
                c: iir_coeffs::G712_16K_C.to_vec(),
            },
            2, 1, 1.0,
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
        HQDown2To1, HQDown3To1, HQUp1To2, HQUp1To3,
        FlatBandPass, FlatBandPass1, FlatBandPass1To2,
        IRS8, IRS16, ModIRS16, ModIRS48,
        Msin16k, Pso8k, Dsm16k, Hirs16, TiaIrs8, RxIrs8, RxIrs16,
        P341_16k, Bp5k16k, Bp100_5k16k, Bp14k32k, Bp20k48k,
        LP1p5_48k, LP35_48k, LP7_48k, LP10_48k, LP12_48k, LP14_48k, LP20_48k,
        G712_8k, Pcm16k, Pcm2To1, Pcm1To2,
        DirDCRemoval, DirLP3To1, DirLP1To3, CascLP3To1, CascLP1To3,
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
    for ci in c.iter().take(num_blocks) {
        let bc = [1.0, ci[0], ci[1]];
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
    for (bi, ci) in b.iter().zip(c.iter()).take(num_blocks) {
        // a_out / block_c_i = product of all other blocks' denominators
        let mut other_a: Vec<f64> = vec![1.0];
        for cj in c.iter().take(num_blocks) {
            if std::ptr::eq(cj, ci) { continue; }
            let jc = [1.0, cj[0], cj[1]];
            let mut new_a = vec![0.0; other_a.len() + jc.len() - 1];
            for (m, &av) in other_a.iter().enumerate() {
                for (n, &sv) in jc.iter().enumerate() {
                    new_a[m + n] += av * sv;
                }
            }
            other_a = new_a;
        }
        // Numerator contribution: gain * (b_i0 + b_i1 z^-1 + b_i2 z^-2) * other_a
        for (m, &bm) in bi.iter().enumerate() {
            for (n, &av) in other_a.iter().enumerate() {
                b_out[m + n] += gain * bm * av;
            }
        }
    }
    (b_out, a_out)
}
