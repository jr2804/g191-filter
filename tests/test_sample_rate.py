"""Rate-aware filtering: filter_array/filter_wave sample-rate adaptation.

Regression tests for the silent 3x cutoff-shift footgun: applying a 16 kHz
design filter to 48 kHz data verbatim scales every designed frequency by
48000/16000 = 3 (rx_irs16khz: 3617 Hz -> 10851 Hz). With sample_rate=, the
library resamples to the design rate and back so the native response is
preserved at any operational rate.
"""

from __future__ import annotations

from pathlib import Path

import numpy as np
import pytest
import soundfile as sf
from numpy.testing import assert_allclose

from g191_filter import filter_array, filter_wave, get_filter_info

FS_NATIVE = 16000.0
FS_OPERATIONAL = 48000.0


def _impulse(length: int) -> np.ndarray:
    x = np.zeros(length)
    x[0] = 1.0
    return x


def _upper_3db_corner(y: np.ndarray, fs: float) -> float:
    """Upper -3 dB band edge of an impulse response (exact for FIR).

    The filters legitimately low-cut (rx_irs16khz DC is at -58 dB), so the
    corner of interest is the upper band edge, not the first sub--3 dB bin.
    """
    spectrum = np.abs(np.fft.rfft(y))
    freqs = np.fft.rfftfreq(len(y), d=1.0 / fs)
    mag_db = 20 * np.log10(np.maximum(spectrum / spectrum.max(), 1e-12))
    above = freqs[mag_db >= -3.0]
    return float(above.max())


@pytest.mark.parametrize(
    ("filter_id", "expected_corner"),
    [
        ("rx_irs16khz", 3617.0),  # NB receive IRS, low-pass corner
        ("p341_16khz", 6998.0),  # WB send weighting, low-pass corner
    ],
)
def test_native_reference(filter_id: str, expected_corner: float) -> None:
    """Sanity: native-rate corner matches the published response."""
    y = filter_array(filter_id, _impulse(int(FS_NATIVE)))
    assert abs(_upper_3db_corner(y, FS_NATIVE) - expected_corner) < 5.0


@pytest.mark.parametrize(
    ("filter_id", "expected_corner"),
    [
        ("rx_irs16khz", 3617.0),
        ("p341_16khz", 6998.0),
    ],
)
def test_adapted_at_48khz_preserves_corner(filter_id: str, expected_corner: float) -> None:
    """The core regression: 48 kHz data must yield the native corner, not 3x."""
    y = filter_array(filter_id, _impulse(int(FS_OPERATIONAL)), sample_rate=FS_OPERATIONAL)
    corner = _upper_3db_corner(y, FS_OPERATIONAL)
    assert abs(corner - expected_corner) < 15.0, f"corner {corner:.1f} Hz, expected {expected_corner}"


def test_adapted_length_matches_input() -> None:
    y = filter_array("rx_irs16khz", _impulse(47999), sample_rate=FS_OPERATIONAL)
    assert len(y) == 47999


def test_no_sample_rate_scales_frequencies() -> None:
    """Documented STL behavior: without sample_rate, coefficients apply
    verbatim and designed frequencies scale with the actual rate.
    """
    y = filter_array("rx_irs16khz", _impulse(int(FS_OPERATIONAL)))
    corner = _upper_3db_corner(y, FS_OPERATIONAL)
    assert corner == pytest.approx(3 * 3617.0, abs=15.0)


def test_matching_rate_is_bit_identical_to_verbatim() -> None:
    x = np.random.default_rng(7).standard_normal(8192)
    explicit = filter_array("rx_irs16khz", x, sample_rate=FS_NATIVE)
    verbatim = filter_array("rx_irs16khz", x)
    assert_allclose(explicit, verbatim, rtol=0.0, atol=0.0)


def test_converter_ignores_sample_rate() -> None:
    """Rate converters perform their own integrated conversion; any declared
    rate is accepted and the samples are untouched by adaptation.
    """
    x = np.random.default_rng(3).standard_normal(4800)
    declared_48k = filter_array("hq_down_3_to_1", x, sample_rate=48000.0)
    verbatim = filter_array("hq_down_3_to_1", x)
    assert_allclose(declared_48k, verbatim, rtol=0.0, atol=0.0)
    assert len(declared_48k) == 1600  # 3:1 decimation


def test_positive_rate_required() -> None:
    x = np.zeros(16)
    with pytest.raises(ValueError, match="positive"):
        filter_array("rx_irs16khz", x, sample_rate=0.0)


@pytest.fixture
def impulse_wav_48k(tmp_path: Path) -> Path:
    path = tmp_path / "impulse_48k.wav"
    sf.write(path, _impulse(int(FS_OPERATIONAL)), int(FS_OPERATIONAL), subtype="FLOAT")
    return path


def test_auto_adapts_to_header_rate(impulse_wav_48k: Path, tmp_path: Path) -> None:
    """No sample_rate given: the header rate (48 kHz) is used and the
    16 kHz-design response is adapted to it.
    """
    out = tmp_path / "out.wav"
    filter_wave("rx_irs16khz", str(impulse_wav_48k), output_file=str(out))
    y, rate = sf.read(out, dtype="float64")
    assert rate == int(FS_OPERATIONAL)
    assert abs(_upper_3db_corner(y, float(rate)) - 3617.0) < 15.0


def test_explicit_48khz_is_not_decorative(impulse_wav_48k: Path, tmp_path: Path) -> None:
    """sample_rate=48000 must behave identically to the header default."""
    out = tmp_path / "out.wav"
    filter_wave("rx_irs16khz", str(impulse_wav_48k), output_file=str(out), sample_rate=FS_OPERATIONAL)
    y, rate = sf.read(out, dtype="float64")
    assert rate == int(FS_OPERATIONAL)
    assert abs(_upper_3db_corner(y, float(rate)) - 3617.0) < 15.0


def test_downconvert_to_16khz_still_available(impulse_wav_48k: Path, tmp_path: Path) -> None:
    """sample_rate=16000 produces a correctly filtered 16 kHz file."""
    out = tmp_path / "out16.wav"
    filter_wave("rx_irs16khz", str(impulse_wav_48k), output_file=str(out), sample_rate=FS_NATIVE)
    y, rate = sf.read(out, dtype="float64")
    assert rate == int(FS_NATIVE)
    assert abs(_upper_3db_corner(y, float(rate)) - 3617.0) < 15.0


def test_rate_converter_writes_true_output_rate(tmp_path: Path) -> None:
    """hq_down_2_to_1 on a 16 kHz file must write an 8 kHz header (previously
    the decimated data was mislabeled with the input rate).
    """
    src = tmp_path / "in16.wav"
    sf.write(src, _impulse(int(FS_NATIVE)), int(FS_NATIVE), subtype="FLOAT")
    out = tmp_path / "out8.wav"
    filter_wave("hq_down_2_to_1", str(src), output_file=str(out))
    y, rate = sf.read(out, dtype="float64")
    assert rate == 8000
    assert len(y) == int(FS_NATIVE) // 2


def test_design_rates_exposed() -> None:
    """get_filter_info remains the authority for design rates."""
    assert get_filter_info("rx_irs16khz")["sample_rate"] == FS_NATIVE
    assert get_filter_info("p341_16khz")["sample_rate"] == FS_NATIVE
