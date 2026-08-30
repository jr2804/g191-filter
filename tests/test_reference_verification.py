"""Integration tests: Rust G.191 implementation vs. ITU-T STL reference.

Compares our Rust filter outputs against the STL reference outputs and runs
the openitu STL test suite (sanity + precision) as a smoke test.

The STL reference source is never committed; it is cloned on demand from
  https://github.com/openitu/STL  (branch STL2026_ITU-T_submission)
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import numpy as np
import pytest

from g191_filter import filter_array

BASE_DIR = Path(__file__).resolve().parents[1]
STL_DIR = BASE_DIR / "tmp" / "_stl_extract"
BUILD_BIN_DIR = STL_DIR / "build" / "bin" / "Debug"
_EXE = ".exe" if sys.platform == "win32" else ""
FILTER_BIN_FILE = BUILD_BIN_DIR / ("filter" + _EXE)
TEST_DATA_FILE = STL_DIR / "src" / "fir" / "test_data" / "test.src"

# Filter pairs where the STL reference produces the same output length
# and sample-rate behaviour as the Rust core.  Other filters (PCM, FLAT, DC)
# exhibit length mismatches because the C reference decimates internally while
# the Rust API keeps the native sample count.
STL_FILTERS = [
    ("IRS8", "irs8khz"),
    ("IRS16", "irs16khz"),
    ("HQ2", "hq_down_2_to_1"),
    ("HQ3", "hq_down_3_to_1"),
]


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture(scope="session")
def stl_bin_file() -> Path:
    """Ensure the STL reference binary exists; clone + build if not."""
    if FILTER_BIN_FILE.exists():
        return FILTER_BIN_FILE

    script = BASE_DIR / "scripts" / "build_stl_reference.py"
    cp = subprocess.run(  # noqa: S603
        [sys.executable, "-u", str(script)], capture_output=True, text=True, check=False
    )
    if cp.returncode != 0:
        pytest.fail(f"STL reference build failed:\n{cp.stdout}\n{cp.stderr}")
    if not FILTER_BIN_FILE.exists():
        pytest.fail(f"filter binary not found after build at {FILTER_BIN_FILE}")
    return FILTER_BIN_FILE


@pytest.fixture(scope="session")
def openitu_test_runner_file() -> Path:
    """Return the openitu STL test runner (basop_test.exe)."""
    runner = BUILD_BIN_DIR / ("basop_test" + _EXE)
    if not runner.exists():
        pytest.skip(f"openitu basop_test not found at {runner} — run CMake build first")
    return runner


@pytest.fixture(scope="session")
def test_src_file() -> Path:
    """Path to the STL test source file (linear PCM 16-bit)."""
    if not TEST_DATA_FILE.exists():
        pytest.skip(f"STL test data not found at {TEST_DATA_FILE}")
    return TEST_DATA_FILE


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _run_stl_filter(
    bin_path: Path, filter_type: str, in_path: Path, block_size: int = 256
) -> np.ndarray | None:
    """Run the STL reference filter.exe and return int16 output."""
    out_path = BASE_DIR / "tmp" / "_stl_verify_out.bin"
    cmd = [str(bin_path), "-q", filter_type, str(in_path), str(out_path), str(block_size)]
    cp = subprocess.run(cmd, capture_output=True, text=True, check=False)  # noqa: S603,S607
    if cp.returncode != 0:
        return None

    if not out_path.exists():
        return None
    data = np.fromfile(out_path, dtype=np.int16)
    out_path.unlink(missing_ok=True)
    return data if len(data) > 0 else None


# ---------------------------------------------------------------------------
# Tests: Rust vs STL reference
# ---------------------------------------------------------------------------

@pytest.mark.parametrize(("stl_name", "rust_id"), STL_FILTERS)
def test_rust_matches_stl_int16(
    stl_bin_file: Path, test_src_file: Path, stl_name: str, rust_id: str
) -> None:
    """16-bit integer test data → STL and Rust outputs match after rounding.

    The STL reference is compiled with C float (32-bit) arithmetic whereas the
    Rust core uses f64.  Sub-sample differences therefore appear at a small
    number of positions; a ±1 tolerance after rounding is the expected bound.
    """
    stl_out = _run_stl_filter(stl_bin_file, stl_name, test_src_file)
    if stl_out is None:
        pytest.skip(f"STL filter '{stl_name}' produced no output")

    ref_input = np.fromfile(test_src_file, dtype=np.int16).astype(np.float64)
    rust_out = filter_array(rust_id, ref_input)

    min_len = min(len(stl_out), len(rust_out))
    stl_i = stl_out[:min_len].astype(np.int32)
    rust_i = np.round(rust_out[:min_len]).astype(np.int32)

    np.testing.assert_allclose(
        rust_i, stl_i, atol=1,
        err_msg=f"{stl_name}/{rust_id}: 16-bit output mismatch exceeds ±1 tolerance",
    )


# ---------------------------------------------------------------------------
# Tests: openitu STL test suite
# ---------------------------------------------------------------------------

def test_openitu_sanity(openitu_test_runner_file: Path) -> None:
    """Run openitu STL sanity tests (Test_type=0); must pass."""
    cp = subprocess.run(  # noqa: S602,S603,S607
        [str(openitu_test_runner_file), "Test_type=0"],
        capture_output=True, text=True, check=True,
        cwd=str(STL_DIR / "src" / "basop" / "test_framework" / "test_data"),
        shell=True,
    )
    assert cp.returncode == 0, (
        f"openitu sanity tests failed (exit {cp.returncode}):\n{cp.stdout}\n{cp.stderr}"
    )


def test_openitu_precision(openitu_test_runner_file: Path) -> None:
    """Run openitu STL precision tests (Test_type=1); must pass."""
    cp = subprocess.run(  # noqa: S602,S603,S607
        [str(openitu_test_runner_file), "Test_type=1"],
        capture_output=True, text=True, check=True,
        cwd=str(STL_DIR / "src" / "basop" / "test_framework" / "test_data"),
        shell=True,
    )
    assert cp.returncode == 0, (
        f"openitu precision tests failed (exit {cp.returncode}):\n{cp.stdout}\n{cp.stderr}"
    )
