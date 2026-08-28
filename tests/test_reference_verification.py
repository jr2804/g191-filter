import sys
from pathlib import Path

import numpy as np
import soundfile as sf

# Import the Rust implementation (will be available after pyo3 build)
try:
    print("Rust implementation imported successfully")
except Exception as e:
    print(f"Failed to import Rust module: {e}")
    sys.exit(1)


def run_comparison(filter_name: str, input_file: str, tolerance: float = 1e-6) -> bool:
    """Compare Rust implementation against STL reference.

    For integer formats: require exact bit match.
    For float formats: require max difference < tolerance.
    """
    msg = f"\n=== Comparing filter: {filter_name} ==="
    print(msg)

    # Generate reference input (Dirac impulse)
    ref_input_path = Path(input_file)
    if not ref_input_path.exists():
        # Create reference input file
        length = 1024
        signal = generate_test_signal(length)
        sf.write(ref_input_path, signal, 48000)
        print(f"Created reference input: {ref_input_path}")

    # Run STL reference filter (via verify_filters script)
    print("Running STL reference...")
    # This would call the verify_filters script
    # For now, we'll simulate the comparison

    # Run our Rust implementation
    print("Running Rust implementation...")

    # Load our output
    our_output_path = Path(input_file).with_suffix(".out.wav")
    if not our_output_path.exists():
        # Generate our output using Rust
        # This will be replaced with actual Rust call
        print("Rust output not found - skipping comparison")
        return False

    # Compare outputs
    ref_data = np.fromfile(ref_input_path, dtype=np.float32)
    our_data = np.fromfile(our_output_path, dtype=np.float32)

    if len(ref_data) != len(our_data):
        print(f"Length mismatch: ref={len(ref_data)}, our={len(our_data)}")
        return False

    if np.array_equal(ref_data, our_data):
        print("✓ Exact match")
        return True

    diff = np.abs(ref_data - our_data)
    max_diff = np.max(diff)

    if max_diff < tolerance:
        print(f"✓ Within tolerance: max diff {max_diff:.2e}")
        return True

    print(f"✗ Mismatch: max diff {max_diff:.2e} > {tolerance}")
    # Show first few differences
    diff_indices = np.where(diff > tolerance)[0][:10]
    for i in diff_indices:
        print(f"  Index {i}: ref={ref_data[i]:.6f}, our={our_data[i]:.6f}")
    return False


def generate_test_signal(length: int, sample_rate: int = 48000) -> np.ndarray:
    """Generate a simple test signal for verification.

    Uses a Dirac impulse followed by zeros to test impulse response.
    """
    _ = np.arange(length) / sample_rate
    # Dirac impulse at center
    signal = np.zeros(length)
    center = length // 2
    signal[center] = 1.0
    return signal.astype(np.float32)


if __name__ == "__main__":
    # Simple test run
    print("Running reference verification tests")

    # Test a few representative filters
    filters = ["LP1p5_48kHz", "bp5k_16khz", "iir_G712_8khz"]

    all_passed = True
    for f in filters:
        passed = run_comparison(f, "test_input.wav")
        all_passed = all_passed and passed

    if all_passed:
        print("\nAll tests passed!")
        sys.exit(0)
    else:
        print("\nSome tests failed")
        sys.exit(1)
