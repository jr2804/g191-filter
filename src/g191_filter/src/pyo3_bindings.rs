// SPDX-License-Identifier: MIT
// Copyright 2026, Jan.Reimes
#![allow(clippy::useless_conversion)] // `?` on PyErr in PyResult fns triggers identity From

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pyo3::types::PyDict;
use numpy::{PyArray1, PyReadonlyArray1};
use std::str::FromStr;
use std::path::PathBuf;

use crate::{
    get_coefficients_ba, get_coefficients_sos, get_filter_config, list_filter_ids, filter_info,
    BlockwiseFilter, CascadeIirFilter, Coefficients, DirectIirFilter, FilterId, FilterState,
    FilterType, FirFilter, IirFilter,
};

/// Filter a wave file
#[pyfunction]
#[pyo3(signature = (filter_id, input_file, output_file=None, sample_rate=None, inplace=false, block_size=None))]
fn filter_wave(
    filter_id: &str,
    input_file: &str,
    output_file: Option<&str>,
    sample_rate: Option<f64>,
    inplace: bool,
    block_size: Option<usize>,
) -> PyResult<String> {
    let fid = FilterId::from_str(filter_id)
        .map_err(PyValueError::new_err)?;

    let config = get_filter_config(fid)
        .ok_or_else(|| PyValueError::new_err(format!("Filter {filter_id} not found")))?;

    let out_file = if inplace {
        input_file.to_string()
    } else {
        output_file.map(|s| s.to_string()).unwrap_or_else(|| {
            let path = PathBuf::from(input_file);
            let base = path.file_stem().unwrap().to_str().unwrap();
            let dir = path.parent().unwrap_or(std::path::Path::new(""));
            dir.join(format!("{base}_filtered.wav")).to_string_lossy().to_string()
        })
    };

    let (samples, original_sr, orig_fmt, orig_bits) = read_wav(input_file)?;

    let (resampled, effective_sr) = if let Some(target_sr) = sample_rate {
        if target_sr != original_sr {
            let resampler = crate::Resampler::new(original_sr, target_sr);
            let resampled = resampler.resample(&samples);
            (resampled, target_sr)
        } else {
            (samples, original_sr)
        }
    } else {
        (samples, original_sr)
    };

    let filtered = if let Some(bs) = block_size {
        let bs = bs.max(1);
        let mut bw = BlockwiseFilter::new(fid, bs)
            .ok_or_else(|| PyValueError::new_err(format!("Filter {filter_id} not found")))?;
        bw.process_all(&resampled)
    } else {
        match &config.coefficients {
            Coefficients::Fir { h0 } => {
                let hswitch = if config.ratio_num > config.ratio_den { 'U' } else { 'D' };
                let dwn_up = if hswitch == 'U' { config.ratio_num } else { config.ratio_den };
                let mut filter = FirFilter::new(h0, config.gain, dwn_up, hswitch);
                filter.process_block(&resampled)
            }
            Coefficients::IirParallel { gain, direct, b, c } => {
                let mut filter = IirFilter::new(*gain, *direct, b, c, config.ratio_num.max(config.ratio_den), config.ratio_num > config.ratio_den);
                filter.process_block(&resampled)
            }
            Coefficients::IirCascade { gain, b, a } => {
                let mut filter = CascadeIirFilter::new(*gain, b, a, config.ratio_num.max(config.ratio_den), config.ratio_num > config.ratio_den);
                filter.process_block(&resampled)
            }
            Coefficients::IirDirect { gain, b, a } => {
                let mut filter = DirectIirFilter::new(*gain, b, a, config.ratio_num.max(config.ratio_den), config.ratio_num > config.ratio_den);
                filter.process_block(&resampled)
            }
        }
    };

    write_wav(&out_file, &filtered, effective_sr, orig_fmt, orig_bits)?;
    Ok(out_file)
}

/// Filter a numpy array (internally blockwise with configurable block size)
#[pyfunction]
#[pyo3(signature = (filter_id, input_array, block_size=None))]
fn filter_array<'py>(
    py: Python<'py>,
    filter_id: &str,
    input_array: PyReadonlyArray1<'py, f64>,
    block_size: Option<usize>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let fid = FilterId::from_str(filter_id)
        .map_err(PyValueError::new_err)?;

    // block_size=None means no chunking (one-shot processing).
    // Explicit chunking is available via BlockwiseFilter for streaming use cases.
    let input = input_array.as_array().to_vec();
    let filtered = if let Some(bs) = block_size {
        let bs = bs.max(1);
        let mut bw = BlockwiseFilter::new(fid, bs)
            .ok_or_else(|| PyValueError::new_err(format!("Filter {filter_id} not found")))?;
        bw.process_all(&input)
    } else {
        let config = get_filter_config(fid)
            .ok_or_else(|| PyValueError::new_err(format!("Filter {filter_id} not found")))?;
        match &config.coefficients {
            Coefficients::Fir { h0 } => {
                let hswitch = if config.ratio_num > config.ratio_den { 'U' } else { 'D' };
                let dwn_up = if hswitch == 'U' { config.ratio_num } else { config.ratio_den };
                let mut filter = FirFilter::new(h0, config.gain, dwn_up, hswitch);
                filter.process_block(&input)
            }
            Coefficients::IirParallel { gain, direct, b, c } => {
                let mut filter = IirFilter::new(*gain, *direct, b, c, config.ratio_num.max(config.ratio_den), config.ratio_num > config.ratio_den);
                filter.process_block(&input)
            }
            Coefficients::IirCascade { gain, b, a } => {
                let mut filter = CascadeIirFilter::new(*gain, b, a, config.ratio_num.max(config.ratio_den), config.ratio_num > config.ratio_den);
                filter.process_block(&input)
            }
            Coefficients::IirDirect { gain, b, a } => {
                let mut filter = DirectIirFilter::new(*gain, b, a, config.ratio_num.max(config.ratio_den), config.ratio_num > config.ratio_den);
                filter.process_block(&input)
            }
        }
    };

    Ok(PyArray1::from_vec(py, filtered))
}

/// Python class for blockwise (streaming) filtering.
///
/// Maintains the filter state in Rust between calls so that callers can
/// process large inputs in chunks while keeping the internal delay lines
/// and phase counters consistent. The `state` attribute carries the
/// snapshot between chunks; both Rust and Python sides speak the same
/// flat `numpy.ndarray` representation.
#[pyclass]
struct BlockwiseFilterPy {
    inner: BlockwiseFilter,
    /// Underlying coefficient kind, used when rebuilding state
    kind: String,
    /// Number of delay-line blocks
    block_count: usize,
}

#[pymethods]
impl BlockwiseFilterPy {
    #[new]
    #[pyo3(signature = (filter_id, block_size=None))]
    fn new(filter_id: &str, block_size: Option<usize>) -> PyResult<Self> {
        let fid = FilterId::from_str(filter_id)
            .map_err(PyValueError::new_err)?;
        let bs = block_size.unwrap_or(8192).max(1);
        let bw = BlockwiseFilter::new(fid, bs)
            .ok_or_else(|| PyValueError::new_err(format!("Filter {filter_id} not found")))?;
        let (kind, block_count) = classify_filter(&bw);
        Ok(Self { inner: bw, kind, block_count })
    }

    /// Process a chunk of input. The state is updated in place.
    fn process<'py>(
        &mut self,
        py: Python<'py>,
        input_array: PyReadonlyArray1<'py, f64>,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let input = input_array.as_array().to_vec();
        let y = self.inner.process_block(&input);
        Ok(PyArray1::from_vec(py, y))
    }

    /// Process the entire input using chunked processing (streaming).
    fn process_all(&mut self, input_array: Vec<f64>) -> Vec<f64> {
        self.inner.process_all(&input_array)
    }

    /// Reset the filter state to zero.
    fn reset(&mut self) {
        self.inner.reset();
    }

    /// Snapshot the current state.
    #[getter]
    fn state<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let s = self.inner.get_state();
        Ok(PyArray1::from_vec(py, s.to_vec()))
    }

    /// Restore a previously snapshotted state.
    #[setter]
    fn set_state<'py>(
        &mut self,
        _py: Python<'py>,
        value: PyReadonlyArray1<'py, f64>,
    ) -> PyResult<()> {
        let v = value.as_array().to_vec();
        let state = FilterState::from_vec(&v, &self.kind, self.block_count).ok_or_else(|| {
            PyValueError::new_err(format!(
                "State has length {} but {} requires {}",
                v.len(),
                self.kind,
                expected_state_len(&self.kind, self.block_count)
            ))
        })?;
        self.inner.set_state(state);
        Ok(())
    }

    /// Identifier of the underlying filter.
    #[getter]
    fn filter_id(&self) -> String {
        self.inner.filter_id().to_string()
    }

    /// Block size used for chunked processing.
    #[getter]
    fn block_size(&self) -> usize {
        self.inner.block_size()
    }
}

/// Derive the state layout kind and block count from a filter's initial state.
fn classify_filter(bw: &BlockwiseFilter) -> (String, usize) {
    match bw.get_state() {
        FilterState::Fir { t, .. } => ("fir".to_string(), t.len()),
        FilterState::IirParallel { t, .. } => ("iir_parallel".to_string(), t.len()),
        FilterState::IirCascade { t, .. } => ("iir_cascade".to_string(), t.len()),
        FilterState::IirDirect { t, .. } => ("iir_direct".to_string(), t.len()),
    }
}

fn expected_state_len(kind: &str, block_count: usize) -> usize {
    match kind {
        "fir" => block_count + 1,
        "iir_parallel" | "iir_direct" => block_count * 2 + 1,
        "iir_cascade" => block_count * 4 + 1,
        _ => 0,
    }
}

/// Export filter impulse response to a wave file
#[pyfunction]
#[pyo3(signature = (filter_id, output_file, sample_rate, length, fade_out=None))]
fn export_impulse_response(
    filter_id: &str,
    output_file: &str,
    sample_rate: f64,
    length: usize,
    fade_out: Option<usize>,
) -> PyResult<String> {
    let fid = FilterId::from_str(filter_id)
        .map_err(PyValueError::new_err)?;

    let config = get_filter_config(fid)
        .ok_or_else(|| PyValueError::new_err(format!("Filter {filter_id} not found")))?;

    let mut ir = match &config.coefficients {
        Coefficients::Fir { h0 } => {
            // For FIR, resample coefficients if needed
            if (sample_rate - config.sample_rate).abs() < 1e-9 {
                h0.clone()
            } else {
                let resampler = crate::Resampler::new(config.sample_rate, sample_rate);
                resampler.resample(h0)
            }
        }
        _ => {
            let dirac = vec![1.0; length];
            let filtered = match &config.coefficients {
                Coefficients::IirParallel { gain, direct, b, c } => {
                    let mut filter = IirFilter::new(*gain, *direct, b, c, config.ratio_num.max(config.ratio_den), config.ratio_num > config.ratio_den);
                    filter.process_block(&dirac)
                }
                Coefficients::IirCascade { gain, b, a } => {
                    let mut filter = CascadeIirFilter::new(*gain, b, a, config.ratio_num.max(config.ratio_den), config.ratio_num > config.ratio_den);
                    filter.process_block(&dirac)
                }
                Coefficients::IirDirect { gain, b, a } => {
                    let mut filter = DirectIirFilter::new(*gain, b, a, config.ratio_num.max(config.ratio_den), config.ratio_num > config.ratio_den);
                    filter.process_block(&dirac)
                }
                Coefficients::Fir { .. } => unreachable!(),
            };
            let mut ir = filtered;
            if let Some(fade_len) = fade_out {
                for i in 0..fade_len.min(ir.len()) {
                    let fade_pos = (fade_len - i) as f64 / fade_len as f64;
                    let idx = ir.len() - fade_len + i;
                    ir[idx] *= fade_pos;
                }
            }
            ir
        }
    };

    if (sample_rate - config.sample_rate).abs() > 1e-9 {
        let resampler = crate::Resampler::new(config.sample_rate, sample_rate);
        ir = resampler.resample(&ir);
    }

    write_wav(output_file, &ir, sample_rate, hound::SampleFormat::Float, 32)?;
    Ok(output_file.to_string())
}

/// Get filter coefficients in (b, a) format
#[pyfunction]
fn get_coefficients_ba_py(filter_id: &str) -> PyResult<(Vec<f64>, Vec<f64>)> {
    let fid = FilterId::from_str(filter_id)
        .map_err(PyValueError::new_err)?;
    get_coefficients_ba(fid)
        .ok_or_else(|| PyValueError::new_err(format!("Cannot convert filter {filter_id} to (b,a) format")))
}

/// Get filter coefficients in SOS format
#[pyfunction]
fn get_coefficients_sos_py(filter_id: &str) -> PyResult<Vec<[f64; 6]>> {
    let fid = FilterId::from_str(filter_id)
        .map_err(PyValueError::new_err)?;
    get_coefficients_sos(fid)
        .ok_or_else(|| PyValueError::new_err(format!("Cannot convert filter {filter_id} to SOS format")))
}

/// List all available filter IDs
#[pyfunction]
fn list_filters() -> Vec<String> {
    list_filter_ids()
        .iter()
        .map(|fid| fid.stl_name().to_string())
        .collect()
}

/// Get information about a filter
#[pyfunction]
fn get_filter_info_py<'a>(py: Python<'a>, filter_id: &str) -> PyResult<Py<PyDict>> {
    let fid = FilterId::from_str(filter_id)
        .map_err(PyValueError::new_err)?;
    let info = filter_info(fid)
        .ok_or_else(|| PyValueError::new_err(format!("Filter {filter_id} not found")))?;

    let dict = PyDict::new(py);
    dict.set_item("id", info.id.to_string())?;
    dict.set_item("type", match info.filter_type {
        FilterType::Fir => "fir",
        FilterType::Iir => "iir",
    })?;
    dict.set_item("sample_rate", info.sample_rate)?;
    dict.set_item("ratio_num", info.ratio_num)?;
    dict.set_item("ratio_den", info.ratio_den)?;
    dict.set_item("length", info.length)?;
    Ok(dict.into())
}

/// Frequency response arrays (frequencies, magnitudes) bound to Python.
type FreqResponse<'py> = (Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>);

/// Compute frequency response of a filter
#[pyfunction]
fn get_frequency_response<'py>(
    py: Python<'py>,
    filter_id: &str,
    n_points: usize,
    sample_rate: f64,
) -> PyResult<FreqResponse<'py>> {
    let fid = FilterId::from_str(filter_id)
        .map_err(PyValueError::new_err)?;

    let (b, a) = get_coefficients_ba(fid)
        .ok_or_else(|| PyValueError::new_err(format!("Cannot compute frequency response for {filter_id}")))?;

    let mut freqs = Vec::with_capacity(n_points);
    let mut mags = Vec::with_capacity(n_points);

    for i in 0..n_points {
        let freq = (i as f64 / n_points as f64) * sample_rate / 2.0;
        let omega = 2.0 * std::f64::consts::PI * freq / sample_rate;

        // H(e^jw) = sum(b[n] * e^(-jwn)) / sum(a[n] * e^(-jwn))
        let mut num_re = 0.0;
        let mut num_im = 0.0;
        for (k, &bk) in b.iter().enumerate() {
            let phase = -omega * k as f64;
            num_re += bk * phase.cos();
            num_im += bk * phase.sin();
        }
        let mut den_re = 0.0;
        let mut den_im = 0.0;
        for (k, &ak) in a.iter().enumerate() {
            let phase = -omega * k as f64;
            den_re += ak * phase.cos();
            den_im += ak * phase.sin();
        }

        let num_mag = (num_re * num_re + num_im * num_im).sqrt();
        let den_mag = (den_re * den_re + den_im * den_im).sqrt();
        let mag = if den_mag > 0.0 { num_mag / den_mag } else { 0.0 };

        freqs.push(freq);
        mags.push(20.0 * mag.log10().max(-200.0));
    }

    Ok((PyArray1::from_vec(py, freqs), PyArray1::from_vec(py, mags)))
}

fn read_wav(path: &str) -> PyResult<(Vec<f64>, f64, hound::SampleFormat, u16)> {
    use hound::WavReader;
    let reader = WavReader::open(path)
        .map_err(|e| PyValueError::new_err(format!("Failed to read WAV: {e}")))?;
    let spec = reader.spec();
    let sr = spec.sample_rate as f64;
    let fmt = spec.sample_format;
    let bits = spec.bits_per_sample;
    let samples: Vec<f64> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max = 1 << (spec.bits_per_sample - 1);
            reader
                .into_samples::<i32>()
                .map(|s| s.map(|v| v as f64 / max as f64))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| PyValueError::new_err(format!("Failed to decode WAV samples: {e}")))?
        }
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .map(|s| s.map(|v| v as f64))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| PyValueError::new_err(format!("Failed to decode WAV samples: {e}")))?,
    };
    Ok((samples, sr, fmt, bits))
}

fn write_wav(
    path: &str,
    data: &[f64],
    sample_rate: f64,
    fmt: hound::SampleFormat,
    bits: u16,
) -> PyResult<()> {
    use hound::{SampleFormat, WavSpec, WavWriter};
    let spec = WavSpec {
        channels: 1,
        sample_rate: sample_rate as u32,
        bits_per_sample: bits,
        sample_format: fmt,
    };
    let mut writer = WavWriter::create(path, spec)
        .map_err(|e| PyValueError::new_err(format!("Failed to write WAV: {e}")))?;

    for &sample in data {
        match fmt {
            SampleFormat::Float => {
                writer.write_sample(sample as f32)
                    .map_err(|e| PyValueError::new_err(format!("Failed to write sample: {e}")))?;
            }
            SampleFormat::Int => {
                let max = (1i64 << (bits - 1)) as f64 - 1.0;
                let int_sample = (sample.clamp(-1.0, 1.0) * max) as i32;
                writer.write_sample(int_sample)
                    .map_err(|e| PyValueError::new_err(format!("Failed to write sample: {e}")))?;
            }
        }
    }
    writer.finalize()
        .map_err(|e| PyValueError::new_err(format!("Failed to finalize WAV: {e}")))?;
    Ok(())
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(filter_wave, m)?)?;
    m.add_function(wrap_pyfunction!(filter_array, m)?)?;
    m.add_function(wrap_pyfunction!(export_impulse_response, m)?)?;
    m.add_function(wrap_pyfunction!(get_coefficients_ba_py, m)?)?;
    m.add_function(wrap_pyfunction!(get_coefficients_sos_py, m)?)?;
    m.add_function(wrap_pyfunction!(list_filters, m)?)?;
    m.add_function(wrap_pyfunction!(get_filter_info_py, m)?)?;
    m.add_function(wrap_pyfunction!(get_frequency_response, m)?)?;
    m.add_class::<BlockwiseFilterPy>()?;
    // Re-export the class under a more idiomatic name
    m.add("BlockwiseFilter", m.getattr("BlockwiseFilterPy")?)?;
    m.add("__version__", "0.1.0")?;
    Ok(())
}
