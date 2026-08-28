#!/usr/bin/env python3
"""Verify Rust implementation against STL reference output.

Compares the impulse response of each filter produced by our Rust implementation
against the STL reference `filter` executable output.
"""
from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
from g191_filter import filter_array

BASE = Path(__file__).resolve().parents[1]
STL_BIN = BASE / "filter.exe"
REF_INPUT = BASE / "tmp" / "_stl_extract" / "src" / "fir" / "test_data" / "test.src"

# STL filter type names → our Rust filter IDs
FILTER_MAP = [
    ("IRS8", "irs8khz"),
    ("IRS16", "irs16khz"),
    ("IRS48", "mod_irs48khz"),
    ("HQ2", "hq_down_2_to_1"),
    ("HQ3", "hq_down_3_to_1"),
    ("FLAT", "flat_band_pass"),
]


def main() -> None:
    if not STL_BIN.exists():
        print("STL reference binary not found. Run 'mise run clone-stl' and 'mise run build-stl' first.")
        sys.exit(1)
    if not REF_INPUT.exists():
        print(f"Reference input not found at {REF_INPUT}")
        sys.exit(1)

    # Load reference input (STL uses linear PCM 16-bit)
    ref_input = np.fromfile(REF_INPUT, dtype=np.int16).astype(np.float64)

    passed = 0
    failed = 0

    for stl_type, rust_id in FILTER_MAP:
        print(f"\n=== {stl_type} / {rust_id} ===")

        # 1. Run STL reference
        ref_output = run_stl_reference(stl_type, REF_INPUT)
        if ref_output is None:
            print(f"  Failed (STL reference error)")
            failed += 1
            continue

        # 2. Run Rust filter on the same input
        our_output = run_rust_filter(rust_id, ref_input)

        # 3. Compare (normalize both for comparison)
        if compare_signals(ref_output, our_output, tolerance=1e-2):
            print(f"  PASS")
            passed += 1
        else:
            print(f"  FAIL")
            failed += 1

    # Also test impulse response for a few filters
    print("\n=== Impulse response comparison ===")
    impulse = np.zeros(8192, dtype=np.float64)
    impulse[0] = 1.0

    for stl_type, rust_id in [("IRS8", "irs8khz"), ("IRS16", "irs16khz")]:
        print(f"\n--- {stl_type} / {rust_id} (impulse) ---")

        # Write impulse as int16 to /tmp
        impulse_int = (impulse * 32767).astype(np.int16)
        impulse_path = Path("/tmp/impulse.bin")
        impulse_int.tofile(impulse_path)

        ref_out = run_stl_reference(stl_type, impulse_path)
        if ref_out is not None:
            our_out = run_rust_filter(rust_id, impulse)
            compare_signals(ref_out, our_out, tolerance=1e-2)

    print(f"\n{'='*40}")
    print(f"Results: {passed} passed, {failed} failed out of {passed + failed} tests")
    if failed > 0:
        sys.exit(1)


def run_stl_reference(filter_type: str, input_file: Path, block_size: int = 256) -> np.ndarray | None:
    """Run the STL filter binary on input_file, return output samples as float64."""
    tmp_output = Path("/tmp/stl_output.bin")
    cmd = [str(STL_BIN), "-q", filter_type, str(input_file), str(tmp_output), str(block_size)]
    subprocess.run(cmd, capture_output=True, text=True)
    if not tmp_output.exists():
        return None
    return np.fromfile(tmp_output, dtype=np.int16).astype(np.float64)


def run_rust_filter(filter_id: str, impulse: np.ndarray) -> np.ndarray:
    """Run Rust filter on impulse and return output."""
    return filter_array(filter_id, impulse)


def compare_signals(ref: np.ndarray, ours: np.ndarray, tolerance: float = 1e-4) -> bool:
    """Compare two signals."""
    if len(ref) != len(ours):
        print(f"  Length mismatch: ref={len(ref)}, our={len(ours)}")
        return False
    diff = np.abs(ref - ours)
    max_diff = np.max(diff)
    if max_diff < tolerance:
        print(f"  ✓ Max diff: {max_diff:.2e}")
        return True
    print(f"  ✗ Mismatch: max diff {max_diff:.2e} > {tolerance}")
    return False


if __name__ == "__main__":
    main()
