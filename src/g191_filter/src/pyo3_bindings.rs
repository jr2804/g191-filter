// SPDX-License-Identifier: MIT
// Copyright 2026, Jan.Reimes

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pyo3::types::PyDict;
use numpy::{PyArray1, PyReadonlyArray1};
use std::str::FromStr;
use std::path::PathBuf;

use crate::{
    get_coefficients_ba, get_coefficients_sos, get_filter_config, list_filter_ids, filter_info,
    CascadeIirFilter, Coefficients, DirectIirFilter, FilterId, FilterType, FirFilter, IirFilter,
};

/// Filter a wave file
#[pyfunction]
#[pyo3(signature = (filter_id, input_file, output_file=None, sample_rate=None, inplace=false))]
fn filter_wave(
    filter_id: &str,
    input_file: &str,
    output_file: Option<&str>,
    sample_rate: Option<f64>,
    inplace: bool,
) -> PyResult<String> {
    let fid = FilterId::from_str(filter_id)
        .map_err(|e| PyValueError::new_err(e))?;

    let config = get_filter_config(fid)
        .ok_or_else(|| PyValueError::new_err(format!("Filter {filter_id} not found")))?;

    let out_file = if inplace {
        input_file.to_string()
    } else {
        output_file.map(|s| s.to_string()).unwrap_or_else(|| {
            let path = PathBuf::from(input_file);
            let base = path.file_stem().unwrap().to_str().unwrap();
            let dir = path.parent().unwrap_or(&std::path::Path::new(""));
            dir.join(format!("{base}_filtered.wav")).to_string_lossy().to_string()
        })
    };

    let (samples, original_sr) = read_wav(input_file)?;

    let (resampled, effective_sr) = if let Some(target_sr) = sample_rate {
        if target_sr != original_sr {
            let coeffs = get_fir_resample_coeffs();
            let resampler = crate::Resampler::new(original_sr, target_sr);
            let mut phase = 0usize;
            let resampled = resampler.upsample(&samples, &coeffs, &mut phase);
            (resampled, target_sr)
        } else {
            (samples, original_sr)
        }
    } else {
        (samples, original_sr)
    };

    let filtered = match &config.coefficients {
        Coefficients::Fir { h0 } => {
            let mut filter = FirFilter::new(h0, config.gain, config.ratio_den, 'D');
            filter.process_block(&resampled)
        }
        Coefficients::IirParallel { gain, direct, b, c } => {
            let mut filter = IirFilter::new(*gain, *direct, b, c, config.ratio_den);
            filter.process_block(&resampled)
        }
        Coefficients::IirCascade { gain, b, a } => {
            let mut filter = CascadeIirFilter::new(*gain, b, a, config.ratio_den);
            filter.process_block(&resampled)
        }
        Coefficients::IirDirect { gain, b, a } => {
            let mut filter = DirectIirFilter::new(*gain, b, a, config.ratio_den);
            filter.process_block(&resampled)
        }
    };

    write_wav(&out_file, &filtered, effective_sr)?;
    Ok(out_file)
}

/// Filter a numpy array
#[pyfunction]
fn filter_array<'py>(
    py: Python<'py>,
    filter_id: &str,
    input_array: PyReadonlyArray1<'py, f64>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let fid = FilterId::from_str(filter_id)
        .map_err(|e| PyValueError::new_err(e))?;

    let config = get_filter_config(fid)
        .ok_or_else(|| PyValueError::new_err(format!("Filter {filter_id} not found")))?;

    let input = input_array.as_array().to_vec();

    let filtered = match &config.coefficients {
        Coefficients::Fir { h0 } => {
            let mut filter = FirFilter::new(h0, config.gain, config.ratio_den, 'D');
            filter.process_block(&input)
        }
        Coefficients::IirParallel { gain, direct, b, c } => {
            let mut filter = IirFilter::new(*gain, *direct, b, c, config.ratio_den);
            filter.process_block(&input)
        }
        Coefficients::IirCascade { gain, b, a } => {
            let mut filter = CascadeIirFilter::new(*gain, b, a, config.ratio_den);
            filter.process_block(&input)
        }
        Coefficients::IirDirect { gain, b, a } => {
            let mut filter = DirectIirFilter::new(*gain, b, a, config.ratio_den);
            filter.process_block(&input)
        }
    };

    Ok(PyArray1::from_vec_bound(py, filtered))
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
        .map_err(|e| PyValueError::new_err(e))?;

    let config = get_filter_config(fid)
        .ok_or_else(|| PyValueError::new_err(format!("Filter {filter_id} not found")))?;

    let mut ir = match &config.coefficients {
        Coefficients::Fir { h0 } => {
            // For FIR, resample coefficients if needed
            if (sample_rate - config.sample_rate).abs() < 1e-9 {
                h0.clone()
            } else {
                let resampler = crate::Resampler::new(config.sample_rate, sample_rate);
                let mut phase = 0usize;
                resampler.upsample(h0, h0, &mut phase)
            }
        }
        _ => {
            let dirac = vec![1.0; length];
            let filtered = match &config.coefficients {
                Coefficients::IirParallel { gain, direct, b, c } => {
                    let mut filter = IirFilter::new(*gain, *direct, b, c, config.ratio_den);
                    filter.process_block(&dirac)
                }
                Coefficients::IirCascade { gain, b, a } => {
                    let mut filter = CascadeIirFilter::new(*gain, b, a, config.ratio_den);
                    filter.process_block(&dirac)
                }
                Coefficients::IirDirect { gain, b, a } => {
                    let mut filter = DirectIirFilter::new(*gain, b, a, config.ratio_den);
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
        let mut phase = 0usize;
        ir = resampler.upsample(&ir, &ir, &mut phase);
    }

    write_wav(output_file, &ir, sample_rate)?;
    Ok(output_file.to_string())
}

/// Get filter coefficients in (b, a) format
#[pyfunction]
fn get_coefficients_ba_py(filter_id: &str) -> PyResult<(Vec<f64>, Vec<f64>)> {
    let fid = FilterId::from_str(filter_id)
        .map_err(|e| PyValueError::new_err(e))?;
    get_coefficients_ba(fid)
        .ok_or_else(|| PyValueError::new_err(format!("Cannot convert filter {filter_id} to (b,a) format")))
}

/// Get filter coefficients in SOS format
#[pyfunction]
fn get_coefficients_sos_py(filter_id: &str) -> PyResult<Vec<[f64; 6]>> {
    let fid = FilterId::from_str(filter_id)
        .map_err(|e| PyValueError::new_err(e))?;
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
        .map_err(|e| PyValueError::new_err(e))?;
    let info = filter_info(fid)
        .ok_or_else(|| PyValueError::new_err(format!("Filter {filter_id} not found")))?;

    let dict = PyDict::new_bound(py);
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

/// Compute frequency response of a filter
#[pyfunction]
fn get_frequency_response<'py>(
    py: Python<'py>,
    filter_id: &str,
    n_points: usize,
    sample_rate: f64,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let fid = FilterId::from_str(filter_id)
        .map_err(|e| PyValueError::new_err(e))?;

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

    Ok((PyArray1::from_vec_bound(py, freqs), PyArray1::from_vec_bound(py, mags)))
}

fn read_wav(path: &str) -> PyResult<(Vec<f64>, f64)> {
    use hound::WavReader;
    let reader = WavReader::open(path)
        .map_err(|e| PyValueError::new_err(format!("Failed to read WAV: {e}")))?;
    let spec = reader.spec();
    let sr = spec.sample_rate as f64;
    let max = 1 << (spec.bits_per_sample - 1);
    let samples: Vec<f64> = reader
        .into_samples::<i32>()
        .filter_map(|s| s.ok())
        .map(|s: i32| match spec.sample_format {
            hound::SampleFormat::Int => s as f64 / max as f64,
            hound::SampleFormat::Float => s as f64,
        })
        .collect();
    Ok((samples, sr))
}

fn write_wav(path: &str, data: &[f64], sample_rate: f64) -> PyResult<()> {
    use hound::{SampleFormat, WavSpec, WavWriter};
    let spec = WavSpec {
        channels: 1,
        sample_rate: sample_rate as u32,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)
        .map_err(|e| PyValueError::new_err(format!("Failed to write WAV: {e}")))?;

    for &sample in data {
        let clamped = sample.clamp(-1.0, 1.0);
        let int_sample = (clamped * 32767.0) as i16;
        writer.write_sample(int_sample)
            .map_err(|e| PyValueError::new_err(format!("Failed to write sample: {e}")))?;
    }
    writer.finalize()
        .map_err(|e| PyValueError::new_err(format!("Failed to finalize WAV: {e}")))?;
    Ok(())
}

fn get_fir_resample_coeffs() -> Vec<f64> {
    // Sinc-based low-pass filter for sample rate conversion
    let n = 64;
    let mut coeffs = vec![0.0; n];
    let fc = 0.5;
    let center = n / 2;
    for i in 0..n {
        let x = (i as i32 - center as i32) as f64;
        if x == 0.0 {
            coeffs[i] = 4.0 * fc;
        } else {
            coeffs[i] = 4.0 * fc * (std::f64::consts::PI * x * fc).sin() / (std::f64::consts::PI * x * fc);
        }
        let window = 0.54 - 0.46 * (2.0 * std::f64::consts::PI * i as f64 / (n - 1) as f64).cos();
        coeffs[i] *= window;
    }
    coeffs
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
    m.add("__version__", "0.1.0")?;
    Ok(())
}
