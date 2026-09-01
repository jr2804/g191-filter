"""Integration tests: Rust G.191 implementation vs. ITU-T STL reference.

Compares our Rust filter outputs against the STL reference outputs.

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
_EXE = ".exe" if sys.platform == "win32" else ""


# Filter pairs where the STL reference produces the same output length
# and sample-rate behaviour as the Rust core.  Other filters (PCM, FLAT, DC)
# exhibit length mismatches because the C reference decimates internally while
# the Rust API keeps the native sample count.
_STL_DIR_CACHE_KEY = "stl_extract_dir"

STL_FILTERS = [
    ("IRS8", "irs8khz"),
    ("IRS16", "irs16khz"),
    ("HQ2", "hq_down_2_to_1"),
    ("HQ3", "hq_down_3_to_1"),
]


# ---------------------------------------------------------------------------
# Tests: Rust vs STL reference
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(("stl_name", "rust_id"), STL_FILTERS)
def test_rust_matches_stl_int16(stl_bin_file: Path, test_src_file: Path, tmp_path: Path, stl_name: str, rust_id: str) -> None:
    """16-bit integer test data → STL and Rust outputs match after rounding.

    The STL reference is compiled with C float (32-bit) arithmetic whereas the
    Rust core uses f64.  Sub-sample differences therefore appear at a small
    number of positions; a ±1 tolerance after rounding is the expected bound.
    """
    stl_out = _run_stl_filter(stl_bin_file, stl_name, test_src_file, tmp_path / "_stl_verify_out.bin")
    if stl_out is None:
        pytest.skip(f"STL filter '{stl_name}' produced no output")

    ref_input = np.fromfile(test_src_file, dtype=np.int16).astype(np.float64)
    rust_out = filter_array(rust_id, ref_input)

    min_len = min(len(stl_out), len(rust_out))
    stl_i = stl_out[:min_len].astype(np.int32)
    rust_i = np.round(rust_out[:min_len]).astype(np.int32)

    np.testing.assert_allclose(
        rust_i,
        stl_i,
        atol=1,
        err_msg=f"{stl_name}/{rust_id}: 16-bit output mismatch exceeds ±1 tolerance",
    )


@pytest.fixture(scope="session")
def stl_bin_file(stl_dir: Path) -> Path:
    """Path to the built STL filter binary (built by the stl_dir fixture)."""
    for candidate in _filter_candidates(stl_dir):
        if candidate.exists():
            return candidate
    pytest.fail(f"filter binary not found after build at {_filter_candidates(stl_dir)}")


def _filter_candidates(stl_dir: Path) -> list[Path]:
    """Candidate locations for the built filter binary.

    CMake puts outputs into bin/Debug on MSVC/Xcode and into bin/
    on single-config generators (Unix Makefiles, Ninja).
    """
    b = stl_dir / "build" / "bin"
    return [b / "Debug" / ("filter" + _EXE), b / ("filter" + _EXE)]


@pytest.fixture(scope="session")
def test_src_file(stl_dir: Path) -> Path:
    """Path to the STL test source file (linear PCM 16-bit)."""
    path = stl_dir / "src" / "fir" / "test_data" / "test.src"
    if not path.exists():
        pytest.skip(f"STL test data not found at {path}")
    return path


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _run_stl_filter(bin_path: Path, filter_type: str, in_path: Path, out_path: Path, block_size: int = 256) -> np.ndarray | None:
    """Run the STL reference filter.exe and return int16 output."""
    cmd = [str(bin_path), "-q", filter_type, str(in_path), str(out_path), str(block_size)]
    cp = subprocess.run(cmd, capture_output=True, text=True, check=False)  # noqa: S603
    if cp.returncode != 0:
        return None

    if not out_path.exists():
        return None
    data = np.fromfile(out_path, dtype=np.int16)
    out_path.unlink(missing_ok=True)
    return data if len(data) > 0 else None


@pytest.fixture(scope="session")
def stl_dir(request: pytest.FixtureRequest, tmp_path_factory: pytest.TempPathFactory) -> Path:
    """STL clone+build directory, owned by the pytest cache fixture.

    The path is persisted via request.config.cache so a built reference
    survives across runs and never lives in a repo directory. If the
    pytest tmp area was cleaned (e.g. --basetemp), it is rebuilt on the
    next run. Scaffolding (clone + build) happens here, as part of the
    suite, via scripts/build_stl_reference.py.
    """
    cached = request.config.cache.get(_STL_DIR_CACHE_KEY, None)
    if cached and Path(cached).is_dir():
        return Path(cached)
    # Note: do NOT pre-create the dir (mktemp does) - the build script
    # treats an existing dir as "clone already done" and would skip the
    # clone, leaving cmake with an empty source dir.
    path = tmp_path_factory.getbasetemp() / "stl_extract"
    _build_stl(path)
    request.config.cache.set(_STL_DIR_CACHE_KEY, str(path))
    return path


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


def _build_stl(stl_dir: Path) -> None:
    """Clone + build the STL reference into stl_dir (pytest-owned)."""
    script = BASE_DIR / "scripts" / "build_stl_reference.py"
    cp = subprocess.run(  # noqa: S603
        [sys.executable, "-u", str(script), "--stl-dir", str(stl_dir), "--out-dir", str(stl_dir / "build" / "bin")],
        capture_output=True,
        text=True,
        check=False,
    )
    if cp.returncode != 0:
        pytest.fail(f"STL reference build failed:\n{cp.stdout}\n{cp.stderr}")
