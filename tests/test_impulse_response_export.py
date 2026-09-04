"""Regression tests for export_impulse_response.

Covers:
- Issue #1: FIR filters were double-resampled at non-native sample rates
  (e.g. msin16khz 185 taps at 48 kHz produced 1665 taps instead of 555).
- Issue #2: Resampled FIR coefficients carried an extra 1/L gain factor
  from the unit-DC-gain resampler (e.g. +9.54 dB at 16 kHz -> 48 kHz).

The IR is exported to a temp directory and read back with soundfile to
inspect length and passband gain via DTFT.
"""

from __future__ import annotations

import math
import sys
import tempfile
from pathlib import Path

import numpy as np
import pytest
import soundfile as sf

# Ensure the local src layout is importable (see project memory #550)
_HERE = Path(__file__).resolve().parent
_ROOT = _HERE.parent
sys.path.insert(0, str(_ROOT / "src"))

import g191_filter as g191  # noqa: E402

_TMP = Path(tempfile.mkdtemp(prefix="g191_ir_test_"))


def _read_ir(filter_id: str, sample_rate: float) -> tuple[np.ndarray, int]:
    out_path = _TMP / f"{filter_id}_{int(sample_rate)}.wav"
    g191.export_impulse_response(filter_id, str(out_path), float(sample_rate), 0)
    data, sr = sf.read(str(out_path), dtype="float64")
    return np.asarray(data), sr


def _passband_gain_db(b: np.ndarray, sr: int, freq_hz: float, n_fft: int | None = None) -> float:
    """DTFT magnitude in dB at ``freq_hz`` (zero-padded to ``n_fft``)."""
    if n_fft is None:
        n_fft = max(1 << 18, 1 << math.ceil(math.log2(len(b)) + 4))
    h = np.fft.rfft(b, n=n_fft)
    freqs = np.fft.rfftfreq(n_fft, d=1.0 / sr)
    idx = int(np.argmin(np.abs(freqs - freq_hz)))
    return float(20.0 * np.log10(np.abs(h[idx]) + 1e-30))


# -----------------------------------------------------------------------
# Issue #1 — FIR was double-resampled at non-native sample rates
# -----------------------------------------------------------------------
@pytest.mark.parametrize(
    ("filter_id", "native_sr", "target_sr", "expected_len"),
    [
        # msin16khz: 185 taps at 16 kHz -> 555 at 48 kHz (185 * 3).
        ("msin16khz", 16_000, 48_000, 555),
        # p341_16khz: 592 taps at 16 kHz -> 1776 at 48 kHz (592 * 3).
        ("p341_16khz", 16_000, 48_000, 1776),
    ],
)
def test_export_impulse_response_fir_length(filter_id: str, native_sr: int, target_sr: int, expected_len: int) -> None:
    _ = native_sr  # documented for context; length is checked against expected_len
    b, sr = _read_ir(filter_id, target_sr)
    assert sr == target_sr
    assert abs(len(b) - expected_len) <= 1, f"{filter_id} @ {target_sr}: got {len(b)} taps, expected ~{expected_len}"


def test_export_impulse_response_fir_length_downsample() -> None:
    """Down-sample path: 48 kHz FIR -> 16 kHz (integer ratio 1/3)."""
    filter_id, target_sr = "mod_irs48khz", 16_000
    b, sr = _read_ir(filter_id, target_sr)
    assert sr == target_sr
    cfg = g191.get_filter_info_py(filter_id)
    native_len = cfg["length"]
    expected = round(native_len * target_sr / cfg["sample_rate"])
    # ±1 tap rounding tolerance (windowed-sinc length rounding).
    assert abs(len(b) - expected) <= 1, f"{filter_id} @ {target_sr}: got {len(b)} taps, expected ~{expected}"


# -----------------------------------------------------------------------
# Issue #2 — resampled FIR carries an extra 1/L gain
# -----------------------------------------------------------------------
@pytest.mark.parametrize(
    ("filter_id", "native_sr", "target_sr"),
    [
        ("msin16khz", 16_000, 48_000),
        ("p341_16khz", 16_000, 48_000),
    ],
)
def test_export_impulse_response_fir_passband_gain_matches_native(filter_id: str, native_sr: int, target_sr: int) -> None:
    """Resampled FIR taps must have the same passband gain as the native taps
    (within the resampler's passband-coincidence tolerance), not 20*log10(L)
    too high.
    """
    b_native, _ = _read_ir(filter_id, native_sr)
    b_resamp, _ = _read_ir(filter_id, target_sr)

    # Pick a passband frequency well below both Nyquists.
    freq = 1_000.0
    gain_native = _passband_gain_db(b_native, native_sr, freq)
    gain_resamp = _passband_gain_db(b_resamp, target_sr, freq)

    diff = abs(gain_native - gain_resamp)
    # 1 dB tolerance absorbs windowed-sinc passband ripple at the FIR's
    # band edge (resampler sinc kernel has ~0.1 dB ripple within passband).
    assert diff < 1.0, (
        f"{filter_id}: native vs resampled passband gain mismatch at {freq} Hz: {gain_native:+.3f} dB vs {gain_resamp:+.3f} dB (diff {diff:.3f} dB)"
    )


# -----------------------------------------------------------------------
# IIR sanity — ensure the IIR path still works (it was untouched, just
# want a smoke test that lives next to the FIR regression).
# -----------------------------------------------------------------------
def test_export_impulse_response_iir_runs() -> None:
    # 4096 taps at 48 kHz of Dirac excitation fed through the direct-form
    # DC-removal IIR. DirectIirFilter's downsampling kernel emits one
    # output per input when idown=1, so the WAV length matches the
    # excitation length.
    out = _TMP / "dir_dc_removal.wav"
    g191.export_impulse_response("dir_dc_removal", str(out), 48_000.0, 4096)
    data, sr = sf.read(str(out), dtype="float64")
    assert sr == 48_000
    assert len(data) == 4096
    # IIR impulse response decays, so no NaN/inf and finite energy.
    assert np.isfinite(data).all()
